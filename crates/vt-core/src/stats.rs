#![allow(dead_code)]

use reqwest::Client;
use serde::Deserialize;

use crate::{auth::Auth, error::VtError};

/// Recent performance statistics for a single player.
#[derive(Debug, Clone, Default)]
pub struct PlayerStats {
    /// Average headshot percentage across last N games (0.0–1.0).
    pub headshot_pct: f32,
    /// Kill/death ratio across last N games.
    pub kd_ratio: f32,
    /// Win rate across last N games (0.0–1.0).
    pub win_rate: f32,
    /// Average RR delta per game (positive = gaining).
    pub avg_rr_delta: f32,
    /// True if the player has received an AFK penalty recently.
    pub afk_penalty: bool,
}

// ── Raw API structs ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct MatchHistoryResponse {
    #[serde(rename = "History")]
    history: Vec<HistoryEntry>,
}

#[derive(Deserialize)]
struct HistoryEntry {
    #[serde(rename = "MatchID")]
    match_id: String,
}

#[derive(Deserialize)]
struct MatchDetailsResponse {
    #[serde(rename = "players")]
    players: Vec<MatchPlayer>,
    #[serde(rename = "teams")]
    teams: Option<Vec<TeamResult>>,
}

#[derive(Deserialize)]
struct MatchPlayer {
    #[serde(rename = "subject")]
    subject: String,
    #[serde(rename = "stats")]
    stats: Option<RawStats>,
    #[serde(rename = "teamId")]
    team_id: String,
}

#[derive(Deserialize)]
struct RawStats {
    #[serde(rename = "kills")]
    kills: u32,
    #[serde(rename = "deaths")]
    deaths: u32,
    #[serde(rename = "assists")]
    assists: u32,
    #[serde(rename = "headshots")]
    headshots: u32,
    #[serde(rename = "bodyshots")]
    bodyshots: u32,
    #[serde(rename = "legshots")]
    legshots: u32,
}

#[derive(Deserialize)]
struct TeamResult {
    #[serde(rename = "teamId")]
    team_id: String,
    #[serde(rename = "won")]
    won: bool,
}

#[derive(Deserialize)]
struct CompetitiveUpdatesResponse {
    #[serde(rename = "Matches")]
    matches: Vec<CompetitiveMatch>,
}

#[derive(Deserialize)]
struct CompetitiveMatch {
    #[serde(rename = "RankedRatingEarned")]
    rr_earned: i32,
    #[serde(rename = "AFKPenalty")]
    afk_penalty: Option<i32>,
}

// ── Public API ───────────────────────────────────────────────────────────────

const HISTORY_FETCH_COUNT: usize = 5;

/// Fetch recent stats (HS%, K/D, WR, avg RR delta) for a player.
///
/// This involves chained async requests:
/// 1. Competitive updates → RR delta + AFK check
/// 2. Match history → last N match IDs
/// 3. Match details → headshots / kills / deaths per game
///
/// Uses `futures::join!` to fetch match details in parallel.
pub async fn get_player_stats(
    client: &Client,
    auth: &Auth,
    puuid: &str,
) -> Result<PlayerStats, VtError> {
    // Fetch competitive updates and match history concurrently
    let (comp_result, hist_result) = futures::join!(
        fetch_competitive_updates(client, auth, puuid),
        fetch_match_history(client, auth, puuid),
    );

    let comp_updates = comp_result.unwrap_or_default();
    let match_ids = hist_result.unwrap_or_default();

    // Fetch match details for each match in parallel
    let details_futures: Vec<_> = match_ids
        .iter()
        .map(|id| fetch_match_details(client, auth, puuid, id))
        .collect();

    let details_results = futures::future::join_all(details_futures).await;
    let details: Vec<MatchStat> = details_results.into_iter().flatten().collect();

    // Aggregate
    let avg_rr_delta = if comp_updates.is_empty() {
        0.0
    } else {
        comp_updates.iter().map(|m| m.rr_earned as f32).sum::<f32>() / comp_updates.len() as f32
    };

    let afk_penalty = comp_updates
        .iter()
        .any(|m| m.afk_penalty.unwrap_or(0) < 0);

    let (headshot_pct, kd_ratio, win_rate) = aggregate_stats(&details);

    Ok(PlayerStats {
        headshot_pct,
        kd_ratio,
        win_rate,
        avg_rr_delta,
        afk_penalty,
    })
}

// ── Helpers ──────────────────────────────────────────────────────────────────

struct MatchStat {
    kills: u32,
    deaths: u32,
    headshots: u32,
    total_shots: u32,
    won: bool,
}

async fn fetch_competitive_updates(
    client: &Client,
    auth: &Auth,
    puuid: &str,
) -> Result<Vec<CompetitiveMatch>, VtError> {
    let url = auth.pvp_url(&format!(
        "/mmr/v1/players/{puuid}/competitiveupdates?queue=competitive&endIndex={}",
        HISTORY_FETCH_COUNT
    ));
    let resp: CompetitiveUpdatesResponse = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(resp.matches)
}

async fn fetch_match_history(
    client: &Client,
    auth: &Auth,
    puuid: &str,
) -> Result<Vec<String>, VtError> {
    let url = auth.pvp_url(&format!(
        "/match-history/v1/history/{puuid}?queue=competitive&endIndex={}",
        HISTORY_FETCH_COUNT
    ));
    let resp: MatchHistoryResponse = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(resp.history.into_iter().map(|e| e.match_id).collect())
}

async fn fetch_match_details(
    client: &Client,
    auth: &Auth,
    puuid: &str,
    match_id: &str,
) -> Option<MatchStat> {
    let url = auth.pvp_url(&format!("/match-details/v1/matches/{match_id}"));
    let resp: MatchDetailsResponse = client
        .get(&url)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;

    let player = resp.players.iter().find(|p| p.subject == puuid)?;
    let stats = player.stats.as_ref()?;

    let total_shots = stats.headshots + stats.bodyshots + stats.legshots;
    let team_id = &player.team_id;
    let won = resp
        .teams
        .as_deref()
        .and_then(|teams| teams.iter().find(|t| &t.team_id == team_id))
        .map(|t| t.won)
        .unwrap_or(false);

    Some(MatchStat {
        kills: stats.kills,
        deaths: stats.deaths,
        headshots: stats.headshots,
        total_shots,
        won,
    })
}

fn aggregate_stats(stats: &[MatchStat]) -> (f32, f32, f32) {
    if stats.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let total_kills: u32 = stats.iter().map(|s| s.kills).sum();
    let total_deaths: u32 = stats.iter().map(|s| s.deaths).sum();
    let total_hs: u32 = stats.iter().map(|s| s.headshots).sum();
    let total_shots: u32 = stats.iter().map(|s| s.total_shots).sum();
    let wins: u32 = stats.iter().filter(|s| s.won).count() as u32;

    let headshot_pct = if total_shots > 0 {
        total_hs as f32 / total_shots as f32
    } else {
        0.0
    };

    let kd_ratio = if total_deaths > 0 {
        total_kills as f32 / total_deaths as f32
    } else {
        total_kills as f32
    };

    let win_rate = wins as f32 / stats.len() as f32;

    (headshot_pct, kd_ratio, win_rate)
}
kd_ratio, win_rate)
}
