use reqwest::Client;
use serde::Deserialize;

use crate::{auth::Auth, error::VtError};

/// Agent selection state for a player in agent-select.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSelectState {
    Locked,
    Selected,
    None,
}

impl AgentSelectState {
    fn from_str(s: &str) -> Self {
        match s {
            "locked" => Self::Locked,
            "selected" => Self::Selected,
            _ => Self::None,
        }
    }
}

/// A single player as seen during agent select.
#[derive(Debug, Clone)]
pub struct PregamePlayer {
    pub puuid: String,
    /// Agent UUID (map to name via valorant-api.com or a local table).
    pub character_id: String,
    pub selection_state: AgentSelectState,
    /// True if the player has enabled streamer / incognito mode.
    pub incognito: bool,
    pub hide_account_level: bool,
    pub account_level: u32,
}

// ── Raw API structs ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct PregamePlayerIdResponse {
    #[serde(rename = "MatchID")]
    match_id: String,
}

#[derive(Deserialize)]
struct PregameMatchResponse {
    #[serde(rename = "AllyTeam")]
    ally_team: AllyTeam,
}

#[derive(Deserialize)]
struct AllyTeam {
    #[serde(rename = "Players")]
    players: Vec<RawPregamePlayer>,
}

#[derive(Deserialize)]
struct RawPregamePlayer {
    #[serde(rename = "Subject")]
    subject: String,
    #[serde(rename = "CharacterID")]
    character_id: String,
    #[serde(rename = "CharacterSelectionState")]
    character_selection_state: String,
    #[serde(rename = "PlayerIdentity")]
    player_identity: PlayerIdentity,
}

#[derive(Deserialize)]
struct PlayerIdentity {
    #[serde(rename = "Incognito")]
    incognito: bool,
    #[serde(rename = "HideAccountLevel")]
    hide_account_level: bool,
    #[serde(rename = "AccountLevel")]
    account_level: u32,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Fetch the list of ally-team players during agent select.
///
/// Returns `(match_id, players)`.
///
/// Endpoints:
/// 1. `GET {glz}/pregame/v1/players/{puuid}` → match ID
/// 2. `GET {glz}/pregame/v1/matches/{matchID}` → player list
pub async fn get_pregame_players(
    client: &Client,
    auth: &Auth,
    puuid: &str,
) -> Result<(String, Vec<PregamePlayer>), VtError> {
    // Step 1: get the pre-game match ID for this player
    let player_url = auth.glz_url(&format!("/pregame/v1/players/{puuid}"));
    let id_resp: PregamePlayerIdResponse = client
        .get(&player_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let match_id = id_resp.match_id;

    // Step 2: fetch match details
    let match_url = auth.glz_url(&format!("/pregame/v1/matches/{match_id}"));
    let match_resp: PregameMatchResponse = client
        .get(&match_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let players = match_resp
        .ally_team
        .players
        .into_iter()
        .map(|p| PregamePlayer {
            puuid: p.subject,
            character_id: p.character_id,
            selection_state: AgentSelectState::from_str(&p.character_selection_state),
            incognito: p.player_identity.incognito,
            hide_account_level: p.player_identity.hide_account_level,
            account_level: p.player_identity.account_level,
        })
        .collect();

    Ok((match_id, players))
}
