//! Windows startup registry support.
//!
//! Writes / removes `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\ValoTracker`
//! so the GUI launches automatically when the user logs in.

use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "ValoTracker";

/// Add or remove the Windows startup registry entry.
///
/// When `enabled` is `true`, writes the path to the running executable
/// (plus `--minimized`) into the Run key so Windows starts ValoTracker at login.
/// When `false`, silently removes the entry if it exists.
pub fn set_run_on_startup(enabled: bool) -> anyhow::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run = hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)?;

    if enabled {
        let exe = std::env::current_exe()?;
        let value = format!("\"{}\" --minimized", exe.display());
        run.set_value(VALUE_NAME, &value)?;
    } else {
        // Ignore "value not found" — already absent is fine
        let _ = run.delete_value(VALUE_NAME);
    }

    Ok(())
}
