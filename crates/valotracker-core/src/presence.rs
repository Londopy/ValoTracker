use base64::Engine as _;
use reqwest::Client;
use serde::Deserialize;

use crate::{error::ValoTrackerError, lockfile::Lockfile, state::GameState};

// ── Raw API response structs ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct PresencesResponse {
    presences: Vec<RawPresence>,
}

#[derive(Debug, Deserialize)]
struct RawPresence {
    puuid: String,
    #[serde(rename = "game_name")]
    game_name: Option<String>,
    #[serde(rename = "game_tag")]
    game_tag: Option<String>,
    /// Base64-encoded JSON private blob.
    private: Option<String>,
}

/// Decoded private blob inside each presence entry.
///
/// All fields use `#[serde(default)]` so that non-standard queue types
/// (Swiftplay, custom games, Deathmatch) — which may omit certain keys —
/// still deserialize successfully instead of silently failing and causing
/// the tracker to report `Disconnected` while the player is in a match.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct PresencePrivate {
    #[serde(rename = "sessionLoopState", default)]
    pub session_loop_state: String,
    #[serde(rename = "partyId", default)]
    pub party_id: String,
    #[serde(rename = "partySize", default)]
    pub party_size: u8,
    #[serde(rename = "partyMaxSize", default)]
    pub party_max_size: u8,
    /// Queue identifier, e.g. `"competitive"`, `"swiftplay"`, `"deathmatch"`.
    /// Empty string for custom games.
    #[serde(rename = "queueId", default)]
    pub queue_id: String,
    #[serde(rename = "partyState", default)]
    pub party_state: String,
    #[serde(rename = "provisioningFlow", default)]
    pub provisioning_flow: String,
    #[serde(rename = "isValid", default)]
    pub is_valid: bool,
    /// Map asset path, e.g. `/Game/Maps/Ascent/Ascent`.
    /// Present during PREGAME and INGAME states.
    #[serde(rename = "matchMap", default)]
    pub match_map: String,
}

/// Fully decoded presence for a single player.
#[derive(Debug, Clone)]
pub struct PlayerPresence {
    pub puuid: String,
    pub game_name: String,
    pub game_tag: String,
    pub private: Option<PresencePrivate>,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Fetch all presences from the local Riot Client API.
///
/// Endpoint: `GET https://127.0.0.1:{port}/chat/v4/presences`
pub async fn get_presences(
    client: &Client,
    lockfile: &Lockfile,
) -> Result<Vec<PlayerPresence>, ValoTrackerError> {
    let url = lockfile.local_url("/chat/v4/presences");
    let raw: PresencesResponse = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let presences = raw
        .presences
        .into_iter()
        .map(|p| {
            let private = p
                .private
                .as_deref()
                .and_then(|b64| decode_private(b64).ok());

            PlayerPresence {
                puuid: p.puuid,
                game_name: p.game_name.unwrap_or_default(),
                game_tag: p.game_tag.unwrap_or_default(),
                private,
            }
        })
        .collect();

    Ok(presences)
}

/// Determine the current [`GameState`] for `puuid` from a set of presences.
pub fn get_game_state(presences: &[PlayerPresence], puuid: &str) -> GameState {
    let presence = presences.iter().find(|p| p.puuid == puuid);

    match presence.and_then(|p| p.private.as_ref()) {
        None => GameState::Disconnected,
        Some(priv_data) => match priv_data.session_loop_state.as_str() {
            "PREGAME" => GameState::Pregame {
                match_id: String::new(), // filled in by pregame.rs
            },
            "INGAME" => GameState::Ingame {
                match_id: String::new(), // filled in by coregame.rs
            },
            _ => GameState::Menu,
        },
    }
}

/// Extract the queue ID and human-readable map name from presence data.
///
/// Called by the engine so it can store the real values in the snapshot
/// rather than hardcoded placeholders.
///
/// Returns `(queue_id, map_name)`:
/// - `queue_id`: e.g. `"swiftplay"`, `"competitive"`, `"custom"` (fallback when empty).
/// - `map_name`: display-friendly name derived from the asset path, e.g. `"Ascent"`.
pub fn get_match_meta(presences: &[PlayerPresence], puuid: &str) -> (String, String) {
    let priv_data = presences
        .iter()
        .find(|p| p.puuid == puuid)
        .and_then(|p| p.private.as_ref());

    match priv_data {
        Some(d) => {
            let queue = if d.queue_id.is_empty() {
                "custom".to_owned()
            } else {
                d.queue_id.clone()
            };
            let map = map_path_to_name(&d.match_map);
            (queue, map)
        }
        None => ("unknown".to_owned(), "Unknown Map".to_owned()),
    }
}

// ── Private helpers ──────────────────────────────────────────────────────────

/// Decode the base64 presence blob into a [`PresencePrivate`].
///
/// We parse as a raw [`serde_json::Value`] first so that a type mismatch on
/// any single field (Riot has changed the schema across patches) does **not**
/// silently discard the entire blob.  Each field is then pulled out
/// individually with a safe fallback.
fn decode_private(b64: &str) -> Result<PresencePrivate, ValoTrackerError> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64)?;
    let decoded = String::from_utf8(bytes)?;

    // Try a clean struct deserialise first — this is the fast path and works
    // on all known Valorant builds.
    if let Ok(p) = serde_json::from_str::<PresencePrivate>(&decoded) {
        return Ok(p);
    }

    // Fallback: parse as a generic Value and extract only what we need so
    // that an unexpected type on any other field cannot kill game detection.
    let v: serde_json::Value = serde_json::from_str(&decoded)?;
    let str_field = |key: &str| -> String {
        v.get(key)
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_owned()
    };
    let u8_field = |key: &str| -> u8 {
        v.get(key)
            .and_then(|x| x.as_u64())
            .unwrap_or(0)
            .min(255) as u8
    };

    Ok(PresencePrivate {
        session_loop_state: str_field("sessionLoopState"),
        party_id: str_field("partyId"),
        party_size: u8_field("partySize"),
        party_max_size: u8_field("partyMaxSize"),
        queue_id: str_field("queueId"),
        party_state: str_field("partyState"),
        provisioning_flow: str_field("provisioningFlow"),
        is_valid: v
            .get("isValid")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        match_map: str_field("matchMap"),
    })
}

/// Convert a Valorant map asset path to a display name.
///
/// `/Game/Maps/Ascent/Ascent` → `"Ascent"`
/// `/Game/Maps/Triad/Triad`   → `"Triad"` (Haven's internal name)
/// `""`                       → `"Unknown Map"`
fn map_path_to_name(path: &str) -> String {
    path.split('/')
        .rfind(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| "Unknown Map".to_owned())
}
