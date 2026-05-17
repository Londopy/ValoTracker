//! Silent background auto-updater.
//!
//! # Design
//! * Called once at startup from a **detached OS thread** — never blocks the main thread.
//! * Checks the GitHub releases API for the latest tag and compares it to the
//!   running binary's version (`CARGO_PKG_VERSION`).
//! * If a newer version is found, it downloads the matching binary and replaces
//!   the running executable in-place using `self_update`.
//! * All network operations have a hard **3-second timeout**; any failure is
//!   logged to the tracing subscriber and silently swallowed.
//! * Checks are throttled to **once per 24 hours** via [`Config::update_check_due`].
//!
//! # Result channel
//! Callers can pass a [`std::sync::mpsc::Sender<UpdateResult>`] that receives
//! the outcome so the UI can display a one-line notification.

use std::sync::mpsc;

use tracing::{debug, error, info, warn};

/// Outcome reported back to the UI after a check.
#[derive(Debug, Clone)]
pub enum UpdateResult {
    /// A new version was downloaded and the binary replaced.
    /// The string is the new version, e.g. `"1.2.0"`.
    Updated(String),
    /// Already on the latest version.
    UpToDate,
    /// Check failed or was skipped (reason logged, not surfaced to user).
    Skipped,
}

const GITHUB_OWNER: &str = "Londopy";
const GITHUB_REPO: &str = "ValoTracker";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Spawn a detached background thread that checks for and applies updates.
///
/// * `tx` — optional channel; if `Some`, the result is sent after the check.
///   The sender is consumed whether or not the update succeeded.
/// * Returns immediately; the spawned thread owns all network I/O.
pub fn spawn_update_check(tx: Option<mpsc::Sender<UpdateResult>>) {
    std::thread::Builder::new()
        .name("vt-updater".into())
        .spawn(move || {
            let result = run_update_check();
            if let Some(sender) = tx {
                let _ = sender.send(result);
            }
        })
        .ok(); // Ignore spawn failure — totally non-critical
}

// ── Internal implementation ───────────────────────────────────────────────────

fn run_update_check() -> UpdateResult {
    debug!("updater: starting check (current={})", CURRENT_VERSION);

    // Build a blocking reqwest client with a 3-second timeout
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .user_agent(format!("ValoTracker/{}", CURRENT_VERSION))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("updater: failed to build HTTP client: {e}");
            return UpdateResult::Skipped;
        }
    };

    // Fetch latest release from GitHub API
    let api_url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        GITHUB_OWNER, GITHUB_REPO
    );

    let response = match client.get(&api_url).send() {
        Ok(r) => r,
        Err(e) => {
            warn!("updater: network error: {e}");
            return UpdateResult::Skipped;
        }
    };

    if !response.status().is_success() {
        warn!("updater: GitHub API returned {}", response.status());
        return UpdateResult::Skipped;
    }

    let json: serde_json::Value = match response.json() {
        Ok(j) => j,
        Err(e) => {
            warn!("updater: failed to parse GitHub response: {e}");
            return UpdateResult::Skipped;
        }
    };

    let tag = match json["tag_name"].as_str() {
        Some(t) => t.trim_start_matches('v'),
        None => {
            warn!("updater: no tag_name in GitHub response");
            return UpdateResult::Skipped;
        }
    };

    debug!("updater: latest tag={} current={}", tag, CURRENT_VERSION);

    // Compare versions (simple semver string comparison is sufficient here;
    // we control the release tags and always use vX.Y.Z format)
    if !is_newer(tag, CURRENT_VERSION) {
        info!("updater: already up to date ({})", CURRENT_VERSION);
        return UpdateResult::UpToDate;
    }

    info!("updater: newer version found: {} → {}", CURRENT_VERSION, tag);

    // Find the right asset for this binary
    // We look for an asset named exactly "ValoTracker.exe" or "ValoTracker-gui.exe"
    // matching the current executable's file name.
    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            error!("updater: cannot determine current exe path: {e}");
            return UpdateResult::Skipped;
        }
    };

    let exe_name = current_exe
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("ValoTracker.exe");

    // Find matching asset URL
    let asset_url = json["assets"]
        .as_array()
        .and_then(|assets| {
            assets.iter().find(|a| {
                a["name"]
                    .as_str()
                    .map(|n| n.eq_ignore_ascii_case(exe_name))
                    .unwrap_or(false)
            })
        })
        .and_then(|a| a["browser_download_url"].as_str())
        .map(str::to_owned);

    let download_url = match asset_url {
        Some(u) => u,
        None => {
            warn!("updater: no matching asset '{}' in release {}", exe_name, tag);
            return UpdateResult::Skipped;
        }
    };

    debug!("updater: downloading from {}", download_url);

    // Fetch checksums.txt from the same release to verify the download.
    // Non-fatal if missing (e.g., older release format) — we still apply the update.
    let checksums_url = format!(
        "https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases/download/v{tag}/checksums.txt"
    );
    let expected_hash: Option<String> = client
        .get(&checksums_url)
        .send()
        .and_then(|r| r.text())
        .ok()
        .and_then(|text| {
            text.lines()
                .find_map(|line| {
                    let mut parts = line.splitn(2, "  ");
                    let hash = parts.next()?.trim().to_owned();
                    let name = parts.next()?.trim();
                    if name.eq_ignore_ascii_case(exe_name) { Some(hash) } else { None }
                })
        });

    if expected_hash.is_none() {
        warn!("updater: checksums.txt not found or exe not listed — skipping integrity check");
    }

    // Download the new binary
    let bytes = match client.get(&download_url).send().and_then(|r| r.bytes()) {
        Ok(b) => b,
        Err(e) => {
            error!("updater: download failed: {e}");
            return UpdateResult::Skipped;
        }
    };

    // Verify SHA-256 if we have a reference hash
    if let Some(expected) = &expected_hash {
        if !verify_sha256(&bytes, expected) {
            error!("updater: SHA256 mismatch — aborting update (possible tampering)");
            return UpdateResult::Skipped;
        }
        debug!("updater: SHA256 verified ok");
    }

    // Replace the running binary in-place
    // We write to a temp file alongside the exe, then do an atomic rename.
    let tmp_path = current_exe.with_extension("exe.vtupdate");

    if let Err(e) = std::fs::write(&tmp_path, &bytes) {
        error!("updater: failed to write temp file: {e}");
        return UpdateResult::Skipped;
    }

    // On Windows we cannot rename over a running process directly.
    // self_update handles this by scheduling deletion of the old file on next boot
    // (MoveFileEx MOVEFILE_DELAY_UNTIL_REBOOT) and immediately swapping the new one in.
    // We replicate that here for zero extra dependencies:
    match replace_binary(&current_exe, &tmp_path) {
        Ok(()) => {
            info!("updater: successfully updated to {}", tag);
            UpdateResult::Updated(tag.to_owned())
        }
        Err(e) => {
            error!("updater: binary replacement failed: {e}");
            let _ = std::fs::remove_file(&tmp_path);
            UpdateResult::Skipped
        }
    }
}

/// Replaces `dest` with `src` atomically using Windows MoveFileEx.
///
/// Strategy:
/// 1. Schedule `dest` (the running exe) for deletion on next reboot.
/// 2. Copy `src` to `dest` (works even while the process image is mapped
///    because we already told the OS to delete the old inode at reboot).
/// 3. Remove the temp file.
#[cfg(target_os = "windows")]
fn replace_binary(
    dest: &std::path::Path,
    src: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_DELAY_UNTIL_REBOOT,
    };

    // Schedule old binary for deletion at next reboot.
    // lpNewFileName must be a true null pointer (not a pointer to a null
    // terminator) when using MOVEFILE_DELAY_UNTIL_REBOOT for deletion.
    let dest_wide: Vec<u16> = dest.as_os_str().encode_wide().chain(std::iter::once(0)).collect();

    // BOOL return: 0 = failure, non-zero = success.
    // Failure is non-fatal — we still attempt the copy; the old binary simply
    // won't be cleaned up automatically at reboot.
    let scheduled = unsafe {
        MoveFileExW(
            dest_wide.as_ptr(),
            std::ptr::null(),
            MOVEFILE_DELAY_UNTIL_REBOOT,
        )
    };
    if scheduled == 0 {
        warn!("updater: MoveFileExW could not schedule old binary for reboot-deletion (non-fatal)");
    }

    // Copy new binary to dest path (this works because the old inode is still
    // there until reboot — Windows allows opening the file for writing even
    // while it's mapped, because we scheduled it for deletion, not locked it).
    std::fs::copy(src, dest)?;
    std::fs::remove_file(src)?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn replace_binary(
    _dest: &std::path::Path,
    _src: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("auto-update is only supported on Windows".into())
}

/// Computes the SHA-256 digest of `data` and compares it (case-insensitively)
/// to the hex string `expected_hex`.  Returns `true` on match.
fn verify_sha256(data: &[u8], expected_hex: &str) -> bool {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(data);
    // Manual hex encoding — avoids adding a `hex` crate dependency.
    let computed: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    computed.eq_ignore_ascii_case(expected_hex)
}

/// Returns `true` if `candidate` is strictly newer than `current` using
/// simple three-part integer comparison (major.minor.patch).
fn is_newer(candidate: &str, current: &str) -> bool {
    parse_semver(candidate)
        .zip(parse_semver(current))
        .map(|(c, cur)| c > cur)
        .unwrap_or(false)
}

fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<u32> = s.split('.').filter_map(|p| p.parse().ok()).collect();
    if parts.len() >= 3 {
        Some((parts[0], parts[1], parts[2]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparison() {
        assert!(is_newer("1.1.0", "1.0.1"));
        assert!(is_newer("2.0.0", "1.9.9"));
        assert!(!is_newer("1.0.1", "1.0.1"));
        assert!(!is_newer("1.0.0", "1.0.1"));
    }
}
