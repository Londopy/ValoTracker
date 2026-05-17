use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::ValoTrackerError;

/// Full application configuration, stored at `%APPDATA%\ValoTracker\config.toml`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub weapon: WeaponConfig,
    #[serde(default)]
    pub features: FeaturesConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DisplayConfig {
    /// Show `[S]` icon for players with streamer mode enabled.
    pub show_streamer_tag: bool,
    /// Show `(3)` party size next to the party icon.
    pub show_party_size: bool,
    /// Highlight enemy premade parties in red.
    pub highlight_enemy_parties: bool,
    /// Use short rank names: "D2" instead of "Diamond 2".
    pub short_ranks: bool,
    /// Show peak act alongside peak rank.
    pub show_peak_act: bool,
    pub show_level: bool,
    pub show_kd: bool,
    pub show_hs: bool,
    pub show_wr: bool,
    pub show_rr_delta: bool,
    /// Clear terminal between refreshes.
    pub auto_clear: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WeaponConfig {
    /// Preferred weapon skin to display in the table.
    pub preferred: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FeaturesConfig {
    /// Discord Rich Presence integration.
    pub discord_rpc: bool,
    /// Discord application ID (read from config so users can substitute their own).
    #[serde(default = "default_discord_app_id")]
    pub discord_app_id: String,
    /// Launch the egui GUI instead of the TUI.
    pub gui: bool,
    /// Flag players who climbed ≥ this many tiers in ≤ smurf_flag_threshold_days.
    pub smurf_flag_threshold_tiers: u8,
    pub smurf_flag_threshold_days: u32,
    /// Minimize to the system tray instead of closing the window (GUI only).
    #[serde(default)]
    pub minimize_to_tray: bool,
    /// Add ValoTracker to the Windows startup registry so it launches at login (GUI only).
    #[serde(default)]
    pub run_on_startup: bool,
    /// Check for updates in the background on startup. Set to false to opt out.
    #[serde(default = "default_true")]
    pub check_updates: bool,
    /// Send Windows desktop toast notifications for key events. Set to false to opt out.
    #[serde(default = "default_true")]
    pub notifications: bool,
    /// Unix timestamp (seconds) of the last successful update check.
    /// Used to throttle checks to once per 24 hours.
    #[serde(default)]
    pub last_update_checked: u64,
}

fn default_true() -> bool { true }

/// The official ValoTracker Discord application ID.
///
/// Register your own at https://discord.com/developers/applications if you
/// want to customise the presence (different assets, name, etc.).
/// Leave this as the default to use the bundled ValoTracker app.
const VALOTRACKER_DISCORD_APP_ID: &str = "1505656422631866480";

fn default_discord_app_id() -> String {
    VALOTRACKER_DISCORD_APP_ID.to_owned()
}

// ── Defaults ──────────────────────────────────────────────────────────────────

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            show_streamer_tag: true,
            show_party_size: true,
            highlight_enemy_parties: true,
            short_ranks: false,
            show_peak_act: true,
            show_level: true,
            show_kd: true,
            show_hs: true,
            show_wr: true,
            show_rr_delta: true,
            auto_clear: true,
        }
    }
}

impl Default for WeaponConfig {
    fn default() -> Self {
        Self {
            preferred: "Vandal".to_owned(),
        }
    }
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            discord_rpc: false,
            discord_app_id: String::new(),
            gui: false,
            smurf_flag_threshold_tiers: 8,
            smurf_flag_threshold_days: 30,
            minimize_to_tray: false,
            run_on_startup: false,
            check_updates: true,
            notifications: true,
            last_update_checked: 0,
        }
    }
}

impl Config {
    /// Returns the current Unix timestamp in seconds.
    pub fn now_unix() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Returns `true` if the last update check was more than 24 hours ago (or never).
    pub fn update_check_due(&self) -> bool {
        let elapsed = Self::now_unix().saturating_sub(self.features.last_update_checked);
        elapsed >= 86_400
    }

    /// Stamp `last_update_checked` with now and persist to disk.
    pub fn mark_update_checked(&mut self) {
        self.features.last_update_checked = Self::now_unix();
        let _ = self.save();
    }
}

// ── Load / Save ───────────────────────────────────────────────────────────────

impl Config {
    /// Load config from `%APPDATA%\ValoTracker\config.toml`.
    ///
    /// Returns default config if the file does not exist yet.
    pub fn load() -> Result<Self, ValoTrackerError> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path)?;
        let cfg = toml::from_str(&raw)?;
        Ok(cfg)
    }

    /// Save config to `%APPDATA%\ValoTracker\config.toml`, creating the directory if needed.
    pub fn save(&self) -> Result<(), ValoTrackerError> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    fn path() -> Result<PathBuf, ValoTrackerError> {
        let appdata = std::env::var("APPDATA")
            .map_err(|_| ValoTrackerError::other("APPDATA environment variable not set"))?;
        Ok(PathBuf::from(appdata)
            .join("ValoTracker")
            .join("config.toml"))
    }
}
