// the auto-updater.
//
// when the app opens a background thread pings github to see if theres a newer
// release out. if there is, the ui pops up an "update available" thing - it
// never just updates on its own without asking first.
// if you say yes it downloads the installer, checks the sha256 so we know it
// didnt get messed with, runs it silently and reopens the app after.
// portable builds (no unins000.exe sitting next to the exe) cant really replace
// themselves so those just open the releases page in the browser instead.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use tracing::{debug, warn};

const GITHUB_OWNER: &str = "Londopy";
const GITHUB_REPO: &str = "ValoTracker";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

// what the background "is there a new version" check came back with
#[derive(Debug, Clone)]
pub enum UpdateState {
    // still checking
    Checking,
    // theres a newer one. the string is the version without the v, like "3.0.4"
    Available(String),
    // already on the latest, nothing to do
    UpToDate,
    // check broke (no internet / api hiccup). not a big deal, just logged
    Failed,
}

// stuff the download thread sends back while its doing its thing
#[derive(Debug, Clone)]
pub enum DownloadMsg {
    // how far along we are, 0.0 to 1.0
    Progress(f32),
    // done downloading, checking the hash now
    Verifying,
    // installer is good to go, heres where it landed
    Done(PathBuf),
    // something went wrong (this text gets shown to the user)
    Failed(String),
    // portable build - opened the releases page instead of downloading
    OpenedBrowser,
}

// ── version check ─────────────────────────────────────────────────────────────

// kick off the version check on its own thread so the ui doesnt freeze.
// sends Checking first, then whatever it found.
pub fn spawn_update_check(tx: Sender<UpdateState>) {
    std::thread::Builder::new()
        .name("vt-update-check".into())
        .spawn(move || {
            let _ = tx.send(UpdateState::Checking);
            let state = match check_for_update() {
                Ok(Some(version)) => UpdateState::Available(version),
                Ok(None) => UpdateState::UpToDate,
                Err(e) => {
                    warn!("updater: check failed: {e}");
                    UpdateState::Failed
                }
            };
            let _ = tx.send(state);
        })
        .ok();
}

// ask github for the latest release and see if its newer than what were on.
// gives back Some(version) (no v) if theres something newer.
pub fn check_for_update() -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent(format!("ValoTracker/{CURRENT_VERSION}"))
        .build()?;

    let url = format!("https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases/latest");
    let json: serde_json::Value = client.get(&url).send()?.error_for_status()?.json()?;

    let raw_tag = json["tag_name"].as_str().unwrap_or("").trim().to_string();
    if raw_tag.is_empty() {
        return Ok(None);
    }

    match (parse_semver(CURRENT_VERSION), parse_semver(&raw_tag)) {
        (Some(current), Some(remote)) if remote > current => {
            Ok(Some(raw_tag.trim_start_matches('v').to_string()))
        }
        _ => Ok(None),
    }
}

// ── download + hash check ─────────────────────────────────────────────────────

// download + hash-check the installer for `version` on a background thread,
// sending progress back over `tx`.
// portable builds just get sent to the releases page (see the note up top).
pub fn start_download(version: String, tx: Sender<DownloadMsg>) {
    std::thread::Builder::new()
        .name("vt-update-dl".into())
        .spawn(move || {
            if is_portable() {
                open_releases_page();
                let _ = tx.send(DownloadMsg::OpenedBrowser);
                return;
            }
            match download_and_verify(&version, &tx) {
                Ok(path) => {
                    let _ = tx.send(DownloadMsg::Done(path));
                }
                Err(e) => {
                    let _ = tx.send(DownloadMsg::Failed(e.to_string()));
                }
            }
        })
        .ok();
}

fn download_and_verify(
    version: &str,
    tx: &Sender<DownloadMsg>,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    use sha2::{Digest, Sha256};
    use std::io::{Read, Write};

    // grab the installer that matches whatever cpu were on
    #[cfg(target_arch = "aarch64")]
    let arch = "arm64";
    #[cfg(not(target_arch = "aarch64"))]
    let arch = "x64";
    let installer_name = format!("ValoTracker-Setup-{version}-{arch}.exe");
    let url = format!(
        "https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases/download/v{version}/{installer_name}"
    );
    let dest = std::env::temp_dir().join(format!("ValoTracker-update-{version}.exe"));

    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("ValoTracker/{CURRENT_VERSION}"))
        .build()?;

    let mut resp = client.get(&url).send()?.error_for_status()?;
    let total = resp.content_length().unwrap_or(0);

    let mut file = std::fs::File::create(&dest)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 65_536];
    let mut downloaded: u64 = 0;

    // pull it down in chunks so we can show a progress bar + hash as we go
    loop {
        let n = resp.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        hasher.update(&buf[..n]);
        downloaded += n as u64;
        if total > 0 {
            let _ = tx.send(DownloadMsg::Progress(downloaded as f32 / total as f32));
        }
    }
    file.flush()?;
    drop(file);

    // check the hash against checksums.txt. if our installer isnt listed (older
    // releases didnt have it) just roll with it instead of erroring out.
    let _ = tx.send(DownloadMsg::Verifying);
    let actual = hex_lower(hasher.finalize().as_slice());
    match fetch_expected_hash(&client, version, &installer_name) {
        Some(expected) if expected == actual => debug!("updater: SHA256 verified"),
        Some(expected) => {
            let _ = std::fs::remove_file(&dest);
            return Err(format!("SHA256 mismatch — expected {expected}, got {actual}").into());
        }
        None => warn!("updater: installer not in checksums.txt — skipping integrity check"),
    }

    Ok(dest)
}

// grab checksums.txt off the release and find the line for our installer
fn fetch_expected_hash(
    client: &reqwest::blocking::Client,
    version: &str,
    installer_name: &str,
) -> Option<String> {
    let url = format!(
        "https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases/download/v{version}/checksums.txt"
    );
    let body = client.get(&url).send().ok()?.text().ok()?;
    body.lines().find_map(|line| {
        // lines look like "<hash>  <filename>"
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?;
        if name.eq_ignore_ascii_case(installer_name) {
            Some(hash.to_lowercase())
        } else {
            None
        }
    })
}

// ── actually installing ───────────────────────────────────────────────────────

// run the installer quietly and reopen the app after.
// returns right away once the helper is kicked off - whoever calls this HAS to
// quit straight after, otherwise the exe is still locked and the installer cant
// overwrite it.
#[cfg(target_os = "windows")]
pub fn spawn_installer(installer: &Path) -> std::io::Result<()> {
    let our_exe = std::env::current_exe().unwrap_or_else(|_| installer.to_path_buf());
    let installer_str = installer.display().to_string().replace('\'', "''");
    let exe_str = our_exe.display().to_string().replace('\'', "''");

    // wait a couple secs for us to fully close (so the exe isnt locked), run the
    // installer with no popups, then open the new version back up
    let script = format!(
        "Start-Sleep -Seconds 2; \
         Start-Process '{installer_str}' -ArgumentList '/SILENT /SUPPRESSMSGBOXES /NORESTART' -Wait; \
         Start-Process '{exe_str}'"
    );

    std::process::Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
        .spawn()
        .map(|_| ())
}

// nothing to do off windows
#[cfg(not(target_os = "windows"))]
pub fn spawn_installer(_installer: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "auto-update is only supported on Windows",
    ))
}

// ── little helpers ────────────────────────────────────────────────────────────

// true if this is a portable copy (no uninstaller next to it), which means we
// cant safely run the installer over the top of it
pub fn is_portable() -> bool {
    match std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
    {
        Some(dir) => !dir.join("unins000.exe").exists(),
        None => true,
    }
}

// just open the releases page in the browser
fn open_releases_page() {
    let _url = format!("https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases/latest");
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer.exe").arg(&_url).spawn();
    }
}

// bytes -> lowercase hex string
fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// turn "major.minor.patch" into numbers we can compare. ignores a leading v and
// anything after a - (like -alpha.1)
fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.trim().trim_start_matches('v');
    let mut parts = s.splitn(4, '.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts
        .next()
        .map(|p| p.split('-').next().unwrap_or(p))
        .and_then(|p| p.parse().ok())?;
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    // quick sanity checks on the version comparing
    #[test]
    fn semver_compare() {
        assert!(parse_semver("3.0.4") > parse_semver("3.0.3"));
        assert!(parse_semver("v3.1.0") > parse_semver("3.0.9"));
        assert_eq!(parse_semver("3.0.3"), parse_semver("v3.0.3"));
        assert!(parse_semver("3.0.4-alpha.1").is_some());
        assert!(parse_semver("2.0.0") > parse_semver("1.9.9"));
    }
}
