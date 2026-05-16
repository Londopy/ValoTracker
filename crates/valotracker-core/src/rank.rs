#![allow(dead_code)]

use std::collections::HashMap;

use reqwest::Client;
use serde::Deserialize;

use crate::{auth::Auth, error::ValoTrackerError};

/// Competitive rank data for a single player.
#[derive(Debug, Clone, Default)]
pub struct PlayerRank {
    /// Tier index 0-27 (0 = Unranked, 3 = Iron 1 … 27 = Radiant).
    pub tier: u8,
    /// Current rank rating (0-100, or higher for Immortal/Radiant).
    pub rr: i32,
    /// All-time peak tier.
    pub peak_tier: u8,
    /// Episode number of peak rank.
    pub peak_episode: u8,
    /// Act number of peak rank.
    pub peak_act: u8,
    /// Leaderboard position (> 0 for Radiant / top Immortal players).
    pub leaderboard_rank: u32,
    /// Win rate over last N games (0.0–1.0).
    pub win_rate: f32,
    /// Number of ranked games played this act.
    pub games_played: u32,
}

// ── Tier name / color tables ──────────────────────────────────────────────────

/// Map a tier index (0-27) to a human-readable name.
pub fn tier_to_name(tier: u8) -> &'static str {
    const NAMES: &[&str] = &[
        "Unranked",    // 0
        "Unknown",     // 1
        "Unknown",     // 2
        "Iron 1",      // 3
        "Iron 2",      // 4
        "Iron 3",      // 5
        "Bronze 1",    // 6
        "Bronze 2",    // 7
        "Bronze 3",    // 8
        "Silver 1",    // 9
        "Silver 2",    // 10
        "Silver 3",    // 11
        "Gold 1",      // 12
        "Gold 2",      // 13
        "Gold 3",      // 14
        "Platinum 1",  // 15
        "Platinum 2",  // 16
        "Platinum 3",  // 17
        "Diamond 1",   // 18
        "Diamond 2",   // 19
        "Diamond 3",   // 20
        "Ascendant 1", // 21
        "Ascendant 2", // 22
        "Ascendant 3", // 23
        "Immortal 1",  // 24
        "Immortal 2",  // 25
        "Immortal 3",  // 26
        "Radiant",     // 27
    ];
    NAMES.get(tier as usize).copied().unwrap_or("Unknown")
}

/// Short form: "D2", "Imm1", "Rad" etc.
pub fn tier_to_short(tier: u8) -> &'static str {
    const SHORT: &[&str] = &[
        "—", "?", "?", "I1", "I2", "I3", "B1", "B2", "B3", "S1", "S2", "S3", "G1", "G2", "G3",
        "P1", "P2", "P3", "D1", "D2", "D3", "A1", "A2", "A3", "Im1", "Im2", "Im3", "Rad",
    ];
    SHORT.get(tier as usize).copied().unwrap_or("?")
}

/// RGB color for a rank tier.
pub fn tier_to_color(tier: u8) -> (u8, u8, u8) {
    match tier {
        0..=2 => (150, 150, 150),   // Unranked — grey
        3..=5 => (90, 70, 55),      // Iron — dark brown
        6..=8 => (160, 105, 60),    // Bronze
        9..=11 => (180, 180, 180),  // Silver
        12..=14 => (210, 175, 50),  // Gold
        15..=17 => (70, 180, 180),  // Platinum — teal
        18..=20 => (100, 150, 240), // Diamond — blue
        21..=23 => (100, 210, 140), // Ascendant — green
        24..=26 => (220, 80, 80),   // Immortal — red
        27 => (255, 215, 0),        // Radiant — gold
        _ => (150, 150, 150),
    }
}

// ── API fetch ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct MmrResponse {
    #[serde(rename = "LatestCompetitiveUpdate")]
    latest_competitive_update: Option<CompetitiveUpdate>,
    #[serde(rename = "QueueSkills")]
    queue_skills: Option<QueueSkills>,
    #[serde(rename = "SeasonalInfoBySeasonID")]
    seasonal_info: Option<HashMap<String, SeasonalInfo>>,
}

#[derive(Deserialize)]
struct CompetitiveUpdate {
    #[serde(rename = "TierAfterUpdate")]
    tier_after_update: u8,
    #[serde(rename = "RankedRatingAfterUpdate")]
    rr_after_update: i32,
    #[serde(rename = "LeaderboardRanked")]
    leaderboard_ranked: Option<u32>,
}

#[derive(Deserialize)]
struct QueueSkills {
    #[serde(rename = "competitive")]
    competitive: Option<CompetitiveSkill>,
}

#[derive(Deserialize)]
struct CompetitiveSkill {
    #[serde(rename = "TotalGamesNeededForRating")]
    total_games_needed_for_rating: Option<u32>,
    #[serde(rename = "NumberOfGamesRequiredForRating")]
    number_of_games_required_for_rating: Option<u32>,
    #[serde(rename = "CurrentSeasonGamesNeededForRating")]
    current_season_games_needed_for_rating: Option<u32>,
    #[serde(rename = "SeasonalInfoBySeasonID")]
    seasonal_info: Option<HashMap<String, SeasonalInfo>>,
}

#[derive(Deserialize)]
struct SeasonalInfo {
    #[serde(rename = "SeasonID")]
    season_id: String,
    #[serde(rename = "NumberOfWins")]
    number_of_wins: u32,
    #[serde(rename = "NumberOfWinsWithPlacements")]
    number_of_wins_with_placements: u32,
    #[serde(rename = "NumberOfGames")]
    number_of_games: u32,
    #[serde(rename = "Rank")]
    rank: u8,
    #[serde(rename = "CapstoneWins")]
    capstone_wins: u32,
    #[serde(rename = "LeaderboardRank")]
    leaderboard_rank: u32,
    #[serde(rename = "CompetitiveTier")]
    competitive_tier: u8,
    #[serde(rename = "RankedRating")]
    ranked_rating: i32,
    #[serde(rename = "WinsByTier")]
    wins_by_tier: Option<HashMap<String, u32>>,
    #[serde(rename = "GamesNeededForRating")]
    games_needed_for_rating: u32,
    #[serde(rename = "TotalWinsNeededForUpgrade")]
    total_wins_needed_for_upgrade: u32,
}

/// Fetch the ranked MMR data for a single player.
///
/// Endpoint: `GET https://pd.{shard}.a.pvp.net/mmr/v1/players/{puuid}`
pub async fn get_player_rank(
    client: &Client,
    auth: &Auth,
    puuid: &str,
) -> Result<PlayerRank, ValoTrackerError> {
    let url = auth.pvp_url(&format!("/mmr/v1/players/{puuid}"));
    let mmr: MmrResponse = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let (tier, rr, leaderboard_rank) = mmr
        .latest_competitive_update
        .map(|u| {
            (
                u.tier_after_update,
                u.rr_after_update,
                u.leaderboard_ranked.unwrap_or(0),
            )
        })
        .unwrap_or((0, 0, 0));

    // Find peak rank across all seasons
    let (peak_tier, peak_episode, peak_act) = find_peak_rank(&mmr.seasonal_info);

    // Win rate from current season
    let (win_rate, games_played) = current_season_stats(&mmr.seasonal_info);

    Ok(PlayerRank {
        tier,
        rr,
        peak_tier,
        peak_episode,
        peak_act,
        leaderboard_rank,
        win_rate,
        games_played,
    })
}

fn find_peak_rank(seasonal: &Option<HashMap<String, SeasonalInfo>>) -> (u8, u8, u8) {
    let Some(map) = seasonal else {
        return (0, 0, 0);
    };

    let best = map.values().max_by_key(|s| s.competitive_tier);
    match best {
        Some(s) => {
            // Season IDs are like "0df5adb9-4dcb-6899-1306-3e8860c1e106" — we can't
            // reliably parse episode/act from the UUID without a lookup table.
            // Store 0 for now; the TUI will just show the peak tier name.
            (s.competitive_tier, 0, 0)
        }
        None => (0, 0, 0),
    }
}

fn current_season_stats(seasonal: &Option<HashMap<String, SeasonalInfo>>) -> (f32, u32) {
    let Some(map) = seasonal else { return (0.0, 0) };

    // Use the season with the most recent (highest) competitive tier as "current"
    let best = map.values().max_by_key(|s| s.number_of_games);
    match best {
        Some(s) if s.number_of_games > 0 => {
            let wr = s.number_of_wins as f32 / s.number_of_games as f32;
            (wr, s.number_of_games)
        }
        _ => (0.0, 0),
    }
}

// ── Rank cache ────────────────────────────────────────────────────────────────

/// In-memory cache for rank data, keyed by (puuid, match_id).
///
/// Invalidated when a new match starts.
#[derive(Debug, Default)]
pub struct RankCache {
    match_id: String,
    data: HashMap<String, PlayerRank>,
}

impl RankCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Retrieve a cached rank if the match ID hasn't changed.
    pub fn get(&self, puuid: &str, match_id: &str) -> Option<&PlayerRank> {
        if self.match_id == match_id {
            self.data.get(puuid)
        } else {
            None
        }
    }

    /// Store a rank for this match.
    pub fn insert(&mut self, puuid: &str, match_id: &str, rank: PlayerRank) {
        if self.match_id != match_id {
            self.invalidate();
            self.match_id = match_id.to_owned();
        }
        self.data.insert(puuid.to_owned(), rank);
    }

    /// Clear all cached entries.
    pub fn invalidate(&mut self) {
        self.data.clear();
        self.match_id.clear();
    }
}
