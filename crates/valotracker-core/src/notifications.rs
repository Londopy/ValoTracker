//! Windows desktop toast notifications.
//!
//! Exposes a single function [`notify`] that fires a non-blocking toast
//! notification using the WinRT `Windows.UI.Notifications` API via the
//! `winrt-notification` crate.
//!
//! # Rules
//! * Notifications are fire-and-forget — they never block the caller.
//! * If the Windows notification service is unavailable, or the crate fails
//!   for any reason, the error is silently swallowed (logged at `warn` level).
//! * On non-Windows platforms this module compiles to a no-op.

use tracing::warn;

/// Send a Windows desktop toast notification with a title and body.
///
/// Does nothing if `notifications_enabled` is `false`, or if the Windows
/// notification service is unavailable.
///
/// This function returns immediately — the notification is dispatched on a
/// background OS thread.
pub fn notify(title: &str, body: &str, notifications_enabled: bool) {
    if !notifications_enabled {
        return;
    }
    let title = title.to_owned();
    let body = body.to_owned();
    std::thread::Builder::new()
        .name("vt-toast".into())
        .spawn(move || send_toast(&title, &body))
        .ok();
}

// ── Platform implementation ───────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn send_toast(title: &str, body: &str) {
    use winrt_notification::{Duration, Sound, Toast};

    let result = Toast::new(Toast::POWERSHELL_APP_ID)
        .title(title)
        .text1(body)
        .sound(Some(Sound::Default))
        .duration(Duration::Short)
        .show();

    if let Err(e) = result {
        warn!("toast notification failed: {e}");
    }
}

#[cfg(not(target_os = "windows"))]
fn send_toast(_title: &str, _body: &str) {
    // No-op on non-Windows
}
