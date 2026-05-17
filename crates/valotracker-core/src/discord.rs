//! Discord Rich Presence integration.
//!
//! Only compiled when the `discord` feature is enabled.
//!
//! # Design
//! * Presence updates run on a dedicated **background OS thread** that holds
//!   the Discord IPC client. The main thread submits [`PresenceUpdate`]
//!   messages over a `std::sync::mpsc` channel.
//! * If Discord is not running the IPC connection attempt fails silently —
//!   no error is surfaced to the user. `discord-presence` 3.x manages
//!   reconnection internally, so we create the client once and let it retry.
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
    /// If Discord is not running the worker will silently drop updates until
    /// the connection is established.
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

    // Create the client and start its internal connection thread.
    // discord-presence 3.x manages reconnection automatically; if Discord
    // is not running, activity calls return Err(DiscordError::NotStarted)
    // which we log at debug level and otherwise ignore.
    let mut client = discord_presence::Client::new(app_id_u64);
    client.start();
    debug!("discord: client started (app_id={})", app_id_u64);

    for update in rx {
        let result: discord_presence::Result<()> = match &update {
            PresenceUpdate::Idle => set_idle(&mut client),
            PresenceUpdate::InMatch {
                map,
                mode,
                party_size,
                party_max,
                start_epoch,
            } => set_in_match(
                &mut client,
                map,
                mode,
                *party_size,
                *party_max,
                *start_epoch,
            ),
            PresenceUpdate::Clear => client.clear_activity().map(|_| ()),
        };

        if let Err(e) = result {
            // NotStarted is the normal case when Discord isn't running yet.
            debug!("discord: activity update failed: {e}");
        }
    }

    // Channel closed — clear presence before exiting
    let _ = client.clear_activity();
}

// ── Activity builders ─────────────────────────────────────────────────────────

fn set_idle(client: &mut discord_presence::Client) -> discord_presence::Result<()> {
    client
        .set_activity(|a| {
            a.state("Idle — Waiting for VALORANT").assets(|ast| {
                ast.large_image("valotracker_logo")
                    .large_text("ValoTracker")
            })
        })
        .map(|_| ())
}

fn set_in_match(
    client: &mut discord_presence::Client,
    map: &str,
    mode: &str,
    party_size: u8,
    party_max: u8,
    start_epoch: i64,
) -> discord_presence::Result<()> {
    client
        .set_activity(|a| {
            let a = a
                .state("In Match")
                .details(format!("{map} — {mode}"))
                .assets(|ast| {
                    ast.large_image("valotracker_logo")
                        .large_text("ValoTracker")
                });

            // Only include timestamps when a real start time was provided.
            let a = if start_epoch > 0 {
                a.timestamps(|ts| ts.start(start_epoch as u64))
            } else {
                a
            };

            // Only show party info for premades (2+ players).
            // discord-presence 3.x uses a (u32, u32) tuple for party size.
            if party_size >= 2 {
                a.party(|p| {
                    p.id(String::from("vt_party"))
                        .size((u32::from(party_size), u32::from(party_max)))
                })
            } else {
                a
            }
        })
        .map(|_| ())
}
