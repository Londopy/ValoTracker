use base64::Engine as _;
use reqwest::Client;
use serde::Deserialize;

use crate::{error::VtError, lockfile::Lockfile, state::GameState};

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
#[derive(Debug, Deserialize, Clone)]
pub struct PresencePrivate {
    #[serde(rename = "sessionLoopState")]
    pub session_loop_state: String,
    #[serde(rename = "partyId")]
    pub party_id: String,
    #[serde(rename = "partySize")]
    pub party_size: u8,
    #[serde(rename = "partyMaxSize")]
    pub party_max_size: u8,
    #[serde(rename = "queueId")]
    pub queue_id: String,
    #[serde(rename = "partyState")]
    pub party_state: String,
    #[serde(rename = "provisioningFlow")]
    pub provisioning_flow: String,
    #[serde(rename = "isValid")]
    pub is_valid: bool,
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
) -> Result<Vec<PlayerPresence>, VtError> {
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

// ── Private helpers ──────────────────────────────────────────────────────────

fn decode_private(b64: &str) -> Result<PresencePrivate, VtError> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64)?;
    let decoded = String::from_utf8(bytes)?;
    Ok(serde_json::from_str(&decoded)?)
}
