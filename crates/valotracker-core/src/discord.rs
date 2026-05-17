//! Discord Rich Presence integration.
//!
//! Only compiled when the `discord` feature is enabled.
//!
//! # Design
//! * Presence updates run on a dedicated **background OS thread** that holds
//!   the Discord IPC client. The main thread submits [`PresenceUpdate`]
//!   messages over a `std::sync::mpsc` channel.
//! * If Discord is not running the IPC connection attempt fails silently —
//!   no error is surfaced to the user.
//! * The worker reconnects automatically when the channel's sender sends any
//!   message after a disconnection (the loop retries connect on each update).
//! * Clear the presence by sending [`PresenceUpdate::Clear`].
//!
//! # Usage
//! ```no_run
//! use valotracker_core::discord::{DiscordRpc, PresenceUpdate};
//!
//! let rpc = DiscordRpc::start("123456789012345678");
//! rpc.send(PresenceUpdate::Idle);
//! rpc.send(PresenceUpdate::InMatch {
//!     map: "Ascent".into(),
//!     mode: "Competitive".into(),
//!     party_size: 2,
//!     party_max: 5,
//!     start_epoch: 0,
//! });
//! rpc.send(PresenceUpdate::Clear);
//! ```

use std::sync::mpsc;
use std::time::Duration;

use tracing::{debug, warn};

// ── Public types ──────────────────────────────────────────────────────────────

/// Messages sent from the main thread to the Discord RPC worker.
#[derive(Debug, Clone)]
pub enum PresenceUpdate {
    /// Player is idle — VALORANT is open but no match is in progress.
    Idle,
    /// Player is in a match.
    InMatch {
        map: String,
        mode: String,
        /// Premade party size (0 if solo / unknown).
        party_size: u8,
        /// Maximum party size for this queue (usually 5).
        party_max: u8,
        /// Unix timestamp (seconds) when the match started.
        start_epoch: i64,
    },
    /// VALORANT closed — clear presence entirely.
    Clear,
}

/// Handle to the Discord RPC background thread.
///
/// Dropping this handle does **not** stop the background thread; the thread
/// exits naturally when the channel is closed (i.e., all senders are dropped).
pub struct DiscordRpc {
    tx: mpsc::Sender<PresenceUpdate>,
}

impl DiscordRpc {
    /// Spawn the Discord RPC worker thread for `app_id`.
    ///
    /// Returns immediately; the worker connects to Discord asynchronously.
    /// If Discord is not running the worker will silently wait for the next
    /// `send()` call before attempting to reconnect.
    pub fn start(app_id: &str) -> Self {
        let (tx, rx) = mpsc::channel::<PresenceUpdate>();
        let app_id = app_id.to_owned();
        std::thread::Builder::new()
            .name("vt-discord-rpc".into())
            .spawn(move || worker(app_id, rx))
            .ok();
        Self { tx }
    }

    /// Send a presence update. Non-blocking; silently drops the message if
    /// the worker thread has exited.
    pub fn send(&self, update: PresenceUpdate) {
        let _ = self.tx.send(update);
    }
}

// ── Worker thread ─────────────────────────────────────────────────────────────

/// The official ValoTracker Discord application ID, used when the user has
/// not configured their own `discord_app_id`.
const DEFAULT_APP_ID: &str = "1505656422631866480";

fn worker(app_id: String, rx: mpsc::Receiver<PresenceUpdate>) {
    // Fall back to the official app ID if the user left the field blank
    let resolved = if app_id.is_empty() {
        DEFAULT_APP_ID.to_owned()
    } else {
        app_id
    };

    // Parse the app ID once
    let app_id_u64: u64 = match resolved.parse() {
        Ok(id) => id,
        Err(_) => {
            warn!("discord: invalid app_id '{}' — RPC disabled", resolved);
            return;
        }
    };

    let mut client: Option<discord_presence::Client> = None;

    for update in rx {
        // Attempt (re-)connection if needed
        if client.is_none() {
            match try_connect(app_id_u64) {
                Some(c) => {
                    debug!("discord: connected (app_id={})", app_id_u64);
                    client = Some(c);
                }
                None => {
                    // Discord not running — skip this update silently
                    debug!("discord: connection failed, skipping update");
                    continue;
                }
            }
        }

        let c = client.as_mut().unwrap();

        let result = match &update {
            PresenceUpdate::Idle => set_idle(c),
            PresenceUpdate::InMatch {
                map,
                mode,
                party_size,
                party_max,
                start_epoch,
            } => set_in_match(c, map, mode, *party_size, *party_max, *start_epoch),
            PresenceUpdate::Clear => {
                let _ = c.clear_activity();
                Ok(())
            }
        };

        if let Err(e) = result {
            warn!("discord: activity update failed: {e}");
            // Treat any error as a disconnect — reconnect on next update
            client = None;
        }
    }

    // Channel closed — clear presence before exiting
    if let Some(c) = client.as_mut() {
        let _ = c.clear_activity();
    }
}

// ── Connection helper ─────────────────────────────────────────────────────────

fn try_connect(app_id: u64) -> Option<discord_presence::Client> {
    let mut client = discord_presence::Client::new(app_id);
    // Attempt to connect; time out quickly so we don't stall startup
    match client.start() {
        Ok(_) => Some(client),
        Err(e) => {
            debug!("discord: connect error: {e}");
            None
        }
    }
}

// ── Activity builders ─────────────────────────────────────────────────────────

fn set_idle(client: &mut discord_presence::Client) -> Result<(), discord_presence::Error> {
    client.set_activity(|a| {
        a.state("Idle — Waiting for VALORANT")
            .assets(|ast| ast.large_image("valotracker_logo").large_text("ValoTracker"))
    })
}

fn set_in_match(
    client: &mut discord_presence::Client,
    map: &str,
    mode: &str,
    party_size: u8,
    party_max: u8,
    start_epoch: i64,
) -> Result<(), discord_presence::Error> {
    client.set_activity(|a| {
        let a = a
            .state("In Match")
            .details(format!("{} — {}", map, mode))
            .assets(|ast| ast.large_image("valotracker_logo").large_text("ValoTracker"));

        let a = if start_epoch > 0 {
            a.timestamps(|ts| ts.start(start_epoch))
        } else {
            a
        };

        if party_size >= 2 {
            a.party(|p| {
                p.id("vt_party")
                    .size([party_size as i32, party_max as i32])
            })
        } else {
            a
        }
    })
}
