#![allow(dead_code)]

use reqwest::Client;
use serde::Deserialize;

use crate::{auth::Auth, error::ValoTrackerError};

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
    /// Recent match results, most-recent-first (true = win), up to last N games.
    pub recent_results: Vec<bool>,
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
) -> Result<PlayerStats, ValoTrackerError> {
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

    let afk_penalty = comp_updates.iter().any(|m| m.afk_penalty.unwrap_or(0) < 0);

    let (headshot_pct, kd_ratio, win_rate) = aggregate_stats(&details);
    // most-recent-first win/loss for the little form streak in the table
    let recent_results: Vec<bool> = details.iter().map(|s| s.won).collect();

    Ok(PlayerStats {
        headshot_pct,
        kd_ratio,
        win_rate,
        avg_rr_delta,
        afk_penalty,
        recent_results,
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
) -> Result<Vec<CompetitiveMatch>, ValoTrackerError> {
    let url = auth.pvp_url(&format!(
        "/mmr/v1/players/{puuid}/competitiveupdates?queue=competitive&endIndex={}",
        HISTORY_FETCH_COUNT
    ));
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Ok(Vec::new());
    }
    // parse loosely off a json Value so a renamed field doesnt blow up the whole
    // thing (riot changes these shapes and the old strict structs meant one tiny
    // change made everyone show 0 stats / unranked).
    let json: serde_json::Value = serde_json::from_str(&resp.text().await.unwrap_or_default())
        .unwrap_or(serde_json::Value::Null);
    let mut out = Vec::new();
    if let Some(arr) = json["Matches"].as_array() {
        for m in arr {
            out.push(CompetitiveMatch {
                rr_earned: m["RankedRatingEarned"].as_i64().unwrap_or(0) as i32,
                afk_penalty: m["AFKPenalty"].as_i64().map(|v| v as i32),
            });
        }
    }
    Ok(out)
}

async fn fetch_match_history(
    client: &Client,
    auth: &Auth,
    puuid: &str,
) -> Result<Vec<String>, ValoTrackerError> {
    let url = auth.pvp_url(&format!(
        "/match-history/v1/history/{puuid}?queue=competitive&endIndex={}",
        HISTORY_FETCH_COUNT
    ));
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Ok(Vec::new());
    }
    let json: serde_json::Value = serde_json::from_str(&resp.text().await.unwrap_or_default())
        .unwrap_or(serde_json::Value::Null);
    let mut ids = Vec::new();
    if let Some(arr) = json["History"].as_array() {
        for e in arr {
            if let Some(id) = e["MatchID"].as_str() {
                ids.push(id.to_string());
            }
        }
    }
    Ok(ids)
}

async fn fetch_match_details(
    client: &Client,
    auth: &Auth,
    puuid: &str,
    match_id: &str,
) -> Option<MatchStat> {
    let url = auth.pvp_url(&format!("/match-details/v1/matches/{match_id}"));
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_str(&resp.text().await.ok()?).ok()?;

    let players = json["players"].as_array()?;
    let me = players
        .iter()
        .find(|p| p["subject"].as_str() == Some(puuid))?;
    let st = &me["stats"];

    let kills = st["kills"].as_u64().unwrap_or(0) as u32;
    let deaths = st["deaths"].as_u64().unwrap_or(0) as u32;
    let headshots = st["headshots"].as_u64().unwrap_or(0) as u32;
    let bodyshots = st["bodyshots"].as_u64().unwrap_or(0) as u32;
    let legshots = st["legshots"].as_u64().unwrap_or(0) as u32;
    let total_shots = headshots + bodyshots + legshots;

    // figure out if this player won by matching their team in the teams array
    let team_id = me["teamId"].as_str().unwrap_or("");
    let won = json["teams"]
        .as_array()
        .and_then(|teams| {
            teams
                .iter()
                .find(|t| t["teamId"].as_str() == Some(team_id))
                .map(|t| t["won"].as_bool().unwrap_or(false))
        })
        .unwrap_or(false);

    Some(MatchStat {
        kills,
        deaths,
        headshots,
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
