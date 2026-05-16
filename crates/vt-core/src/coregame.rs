use reqwest::Client;
use serde::Deserialize;

use crate::{auth::Auth, error::VtError};

/// A single player as seen during an active (in-progress) match.
#[derive(Debug, Clone)]
pub struct IngamePlayer {
    pub puuid: String,
    /// `"Blue"` or `"Red"`.
    pub team_id: String,
    /// Agent UUID.
    pub character_id: String,
    pub incognito: bool,
    pub hide_account_level: bool,
    pub account_level: u32,
    /// Server region/pod, e.g. `"aresriot.aws-rclusterprod-euw1-1.eu-gp-frankfurt-1"`.
    pub game_pod_id: String,
}

// ── Raw API structs ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CoreGamePlayerIdResponse {
    #[serde(rename = "MatchID")]
    match_id: String,
}

#[derive(Deserialize)]
struct CoreGameMatchResponse {
    #[serde(rename = "Players")]
    players: Vec<RawIngamePlayer>,
    #[serde(rename = "GamePodID")]
    game_pod_id: String,
}

#[derive(Deserialize)]
struct RawIngamePlayer {
    #[serde(rename = "Subject")]
    subject: String,
    #[serde(rename = "TeamID")]
    team_id: String,
    #[serde(rename = "CharacterID")]
    character_id: String,
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

/// Fetch all 10 players from an active match.
///
/// Returns `(match_id, players)`.
///
/// Endpoints:
/// 1. `GET {glz}/core-game/v1/players/{puuid}` → match ID
/// 2. `GET {glz}/core-game/v1/matches/{matchID}` → all players
pub async fn get_ingame_players(
    client: &Client,
    auth: &Auth,
    puuid: &str,
) -> Result<(String, Vec<IngamePlayer>), VtError> {
    // Step 1: resolve match ID
    let player_url = auth.glz_url(&format!("/core-game/v1/players/{puuid}"));
    let id_resp: CoreGamePlayerIdResponse = client
        .get(&player_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let match_id = id_resp.match_id;

    // Step 2: fetch match details
    let match_url = auth.glz_url(&format!("/core-game/v1/matches/{match_id}"));
    let match_resp: CoreGameMatchResponse = client
        .get(&match_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let pod = match_resp.game_pod_id;
    let players = match_resp
        .players
        .into_iter()
        .map(|p| IngamePlayer {
            puuid: p.subject,
            team_id: p.team_id,
            character_id: p.character_id,
            incognito: p.player_identity.incognito,
            hide_account_level: p.player_identity.hide_account_level,
            account_level: p.player_identity.account_level,
            game_pod_id: pod.clone(),
        })
        .collect();

    Ok((match_id, players))
}
