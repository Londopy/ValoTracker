use std::path::PathBuf;

use rusqlite::{params, Connection};

use crate::{error::VtError, models::player::ResolvedPlayer};

/// Local SQLite match history database.
///
/// Stored at `%APPDATA%\vt\history.db`.
pub struct MatchHistory {
    conn: Connection,
}

// ── Data structs ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SavedMatch {
    pub id: String,
    pub map: String,
    pub queue: String,
    pub server: String,
    /// Unix timestamp.
    pub saved_at: i64,
    pub won: Option<bool>,
    pub my_agent: String,
    pub my_rank_tier: u8,
    pub my_rr: i32,
    pub my_rr_delta: i32,
    pub my_kills: u32,
    pub my_deaths: u32,
    pub my_assists: u32,
    pub my_score: u32,
    pub player_count: usize,
}

#[derive(Debug, Clone)]
pub struct SavedPlayer {
    pub puuid: String,
    pub name: String,
    pub tag: String,
    pub team: String,
    pub agent: String,
    pub rank_tier: u8,
    pub rr: i32,
    pub peak_tier: u8,
    pub hs_pct: f32,
    pub kd_ratio: f32,
    pub account_lvl: u32,
    pub incognito: bool,
    pub party_id: String,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub score: u32,
    pub won: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PlayerEncounter {
    pub match_id: String,
    pub map: String,
    pub queue: String,
    pub saved_at: i64,
    pub agent: String,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub hs_pct: f32,
    pub kd_ratio: f32,
    pub rank_tier: u8,
    pub rr: i32,
    pub won: Option<bool>,
    pub was_enemy: bool,
}

#[derive(Debug, Clone)]
pub struct EncounterSummary {
    pub total_meetings: u32,
    pub wins_against: u32,
    pub losses_against: u32,
    pub most_played_agent: String,
    pub worst_game: Option<PlayerEncounter>,
    pub avg_kills: f32,
    pub avg_deaths: f32,
    pub avg_hs_pct: f32,
    pub rank_delta: i32,
}

#[derive(Debug, Clone)]
pub struct Nemesis {
    pub puuid: String,
    pub name: String,
    pub tag: String,
    pub encounters: u32,
    pub their_wins: u32,
    pub your_wins: u32,
    pub most_played_agent: String,
    pub avg_kd_vs_you: f32,
}

#[derive(Debug, Clone)]
pub struct AgentStats {
    pub agent: String,
    pub games: u32,
    pub avg_kd: f32,
    pub avg_hs: f32,
    pub win_rate: f32,
}

#[derive(Debug, Clone)]
pub struct MapStats {
    pub map: String,
    pub games: u32,
    pub avg_kd: f32,
    pub avg_hs: f32,
    pub win_rate: f32,
}

#[derive(Debug, Clone)]
pub struct SmurfFlag {
    pub puuid: String,
    pub first_tier: u8,
    pub current_tier: u8,
    pub tier_delta: i32,
    pub days_elapsed: u32,
    pub flagged: bool,
}

// ── Open / migrate ────────────────────────────────────────────────────────────

impl MatchHistory {
    /// Open or create the history database at `%APPDATA%\vt\history.db`.
    pub fn open() -> Result<Self, VtError> {
        let path = Self::db_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        let history = MatchHistory { conn };
        history.migrate()?;
        Ok(history)
    }

    fn db_path() -> Result<PathBuf, VtError> {
        let appdata = std::env::var("APPDATA")
            .map_err(|_| VtError::other("APPDATA environment variable not set"))?;
        Ok(PathBuf::from(appdata).join("vt").join("history.db"))
    }

    fn migrate(&self) -> Result<(), VtError> {
        self.conn.execute_batch(SCHEMA)?;
        Ok(())
    }
}

const SCHEMA: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS matches (
    id           TEXT    PRIMARY KEY,
    map          TEXT    NOT NULL,
    queue        TEXT    NOT NULL,
    server       TEXT,
    saved_at     INTEGER NOT NULL,
    won          INTEGER,
    my_agent     TEXT,
    my_rank_tier INTEGER,
    my_rr        INTEGER,
    my_rr_delta  INTEGER,
    my_kills     INTEGER,
    my_deaths    INTEGER,
    my_assists   INTEGER,
    my_score     INTEGER
);

CREATE TABLE IF NOT EXISTS match_players (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    match_id     TEXT    NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    puuid        TEXT    NOT NULL,
    name         TEXT,
    tag          TEXT,
    team         TEXT,
    agent        TEXT,
    rank_tier    INTEGER,
    rr           INTEGER,
    peak_tier    INTEGER,
    hs_pct       REAL,
    kd_ratio     REAL,
    account_lvl  INTEGER,
    incognito    INTEGER NOT NULL DEFAULT 0,
    party_id     TEXT,
    kills        INTEGER,
    deaths       INTEGER,
    assists      INTEGER,
    score        INTEGER,
    won          INTEGER
);

CREATE INDEX IF NOT EXISTS idx_match_players_puuid    ON match_players(puuid);
CREATE INDEX IF NOT EXISTS idx_match_players_match_id ON match_players(match_id);
"#;

// ── Write operations ──────────────────────────────────────────────────────────

impl MatchHistory {
    /// Save a full match snapshot to the database.
    pub fn save_match(
        &self,
        match_id: &str,
        map: &str,
        queue: &str,
        server: &str,
        players: &[ResolvedPlayer],
        my_puuid: &str,
        won: Option<bool>,
    ) -> Result<(), VtError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let me = players.iter().find(|p| p.puuid == my_puuid);
        let my_agent = me.map(|p| p.agent_name.as_str()).unwrap_or("").to_owned();
        let my_rank_tier = me.map(|p| p.rank.tier).unwrap_or(0) as i64;
        let my_rr = me.map(|p| p.rank.rr).unwrap_or(0) as i64;

        self.conn.execute(
            "INSERT OR REPLACE INTO matches
                (id, map, queue, server, saved_at, won, my_agent, my_rank_tier, my_rr)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                match_id,
                map,
                queue,
                server,
                now,
                won.map(|w| w as i32),
                my_agent,
                my_rank_tier,
                my_rr,
            ],
        )?;

        for player in players {
            self.conn.execute(
                "INSERT INTO match_players
                    (match_id, puuid, name, tag, team, agent, rank_tier, rr,
                     peak_tier, hs_pct, kd_ratio, account_lvl, incognito, party_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    match_id,
                    player.puuid,
                    player.game_name,
                    player.tag_line,
                    player.team_id,
                    player.agent_name,
                    player.rank.tier as i32,
                    player.rank.rr,
                    player.rank.peak_tier as i32,
                    player.stats.headshot_pct,
                    player.stats.kd_ratio,
                    player.account_level as i32,
                    player.incognito as i32,
                    player.party_id,
                ],
            )?;
        }

        Ok(())
    }

    /// Delete a saved match and all its players (CASCADE).
    pub fn delete_match(&self, match_id: &str) -> Result<(), VtError> {
        self.conn
            .execute("DELETE FROM matches WHERE id = ?1", params![match_id])?;
        Ok(())
    }
}

// ── Read operations ───────────────────────────────────────────────────────────

impl MatchHistory {
    /// List saved matches, newest first.
    pub fn list_matches(&self, limit: usize) -> Result<Vec<SavedMatch>, VtError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.map, m.queue, m.server, m.saved_at, m.won,
                    m.my_agent, m.my_rank_tier, m.my_rr, m.my_rr_delta,
                    m.my_kills, m.my_deaths, m.my_assists, m.my_score,
                    COUNT(mp.id) as player_count
             FROM matches m
             LEFT JOIN match_players mp ON mp.match_id = m.id
             GROUP BY m.id
             ORDER BY m.saved_at DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(SavedMatch {
                id: row.get(0)?,
                map: row.get(1)?,
                queue: row.get(2)?,
                server: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                saved_at: row.get(4)?,
                won: row.get::<_, Option<i32>>(5)?.map(|v| v != 0),
                my_agent: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                my_rank_tier: row.get::<_, Option<i32>>(7)?.unwrap_or(0) as u8,
                my_rr: row.get::<_, Option<i32>>(8)?.unwrap_or(0),
                my_rr_delta: row.get::<_, Option<i32>>(9)?.unwrap_or(0),
                my_kills: row.get::<_, Option<i32>>(10)?.unwrap_or(0) as u32,
                my_deaths: row.get::<_, Option<i32>>(11)?.unwrap_or(0) as u32,
                my_assists: row.get::<_, Option<i32>>(12)?.unwrap_or(0) as u32,
                my_score: row.get::<_, Option<i32>>(13)?.unwrap_or(0) as u32,
                player_count: row.get::<_, i64>(14)? as usize,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(VtError::Database)
    }

    /// Load all players for a specific match.
    pub fn get_match_players(&self, match_id: &str) -> Result<Vec<SavedPlayer>, VtError> {
        let mut stmt = self.conn.prepare(
            "SELECT puuid, name, tag, team, agent, rank_tier, rr, peak_tier,
                    hs_pct, kd_ratio, account_lvl, incognito, party_id,
                    kills, deaths, assists, score, won
             FROM match_players WHERE match_id = ?1",
        )?;

        let rows = stmt.query_map(params![match_id], |row| {
            Ok(SavedPlayer {
                puuid: row.get(0)?,
                name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                tag: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                team: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                agent: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                rank_tier: row.get::<_, Option<i32>>(5)?.unwrap_or(0) as u8,
                rr: row.get::<_, Option<i32>>(6)?.unwrap_or(0),
                peak_tier: row.get::<_, Option<i32>>(7)?.unwrap_or(0) as u8,
                hs_pct: row.get::<_, Option<f64>>(8)?.unwrap_or(0.0) as f32,
                kd_ratio: row.get::<_, Option<f64>>(9)?.unwrap_or(0.0) as f32,
                account_lvl: row.get::<_, Option<i32>>(10)?.unwrap_or(0) as u32,
                incognito: row.get::<_, i32>(11)? != 0,
                party_id: row.get::<_, Option<String>>(12)?.unwrap_or_default(),
                kills: row.get::<_, Option<i32>>(13)?.unwrap_or(0) as u32,
                deaths: row.get::<_, Option<i32>>(14)?.unwrap_or(0) as u32,
                assists: row.get::<_, Option<i32>>(15)?.unwrap_or(0) as u32,
                score: row.get::<_, Option<i32>>(16)?.unwrap_or(0) as u32,
                won: row.get::<_, Option<i32>>(17)?.map(|v| v != 0),
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(VtError::Database)
    }

    /// How many past matches have you shared with this player?
    pub fn times_played_with(&self, puuid: &str) -> Result<u32, VtError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT match_id) FROM match_players WHERE puuid = ?1",
            params![puuid],
            |row| row.get(0),
        )?;
        Ok(count as u32)
    }

    /// All past matches where this PUUID appeared.
    pub fn matches_with_player(&self, puuid: &str) -> Result<Vec<SavedMatch>, VtError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.map, m.queue, m.server, m.saved_at, m.won,
                    m.my_agent, m.my_rank_tier, m.my_rr, m.my_rr_delta,
                    m.my_kills, m.my_deaths, m.my_assists, m.my_score,
                    COUNT(mp2.id) as player_count
             FROM matches m
             JOIN match_players mp  ON mp.match_id  = m.id AND mp.puuid = ?1
             LEFT JOIN match_players mp2 ON mp2.match_id = m.id
             GROUP BY m.id
             ORDER BY m.saved_at DESC",
        )?;

        let rows = stmt.query_map(params![puuid], |row| {
            Ok(SavedMatch {
                id: row.get(0)?,
                map: row.get(1)?,
                queue: row.get(2)?,
                server: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                saved_at: row.get(4)?,
                won: row.get::<_, Option<i32>>(5)?.map(|v| v != 0),
                my_agent: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                my_rank_tier: row.get::<_, Option<i32>>(7)?.unwrap_or(0) as u8,
                my_rr: row.get::<_, Option<i32>>(8)?.unwrap_or(0),
                my_rr_delta: row.get::<_, Option<i32>>(9)?.unwrap_or(0),
                my_kills: row.get::<_, Option<i32>>(10)?.unwrap_or(0) as u32,
                my_deaths: row.get::<_, Option<i32>>(11)?.unwrap_or(0) as u32,
                my_assists: row.get::<_, Option<i32>>(12)?.unwrap_or(0) as u32,
                my_score: row.get::<_, Option<i32>>(13)?.unwrap_or(0) as u32,
                player_count: row.get::<_, i64>(14)? as usize,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(VtError::Database)
    }

    /// Full encounter history for a specific player, newest first.
    pub fn get_player_encounters(&self, puuid: &str) -> Result<Vec<PlayerEncounter>, VtError> {
        let mut stmt = self.conn.prepare(
            "SELECT mp.match_id, m.map, m.queue, m.saved_at, mp.agent,
                    mp.kills, mp.deaths, mp.assists, mp.hs_pct, mp.kd_ratio,
                    mp.rank_tier, mp.rr, mp.won, mp.team,
                    (SELECT mp_me.team FROM match_players mp_me
                     WHERE mp_me.match_id = mp.match_id
                       AND mp_me.puuid = ?1) AS my_team
             FROM match_players mp
             JOIN matches m ON m.id = mp.match_id
             WHERE mp.puuid = ?1
             ORDER BY m.saved_at DESC",
        )?;

        let rows = stmt.query_map(params![puuid], |row| {
            let their_team: Option<String> = row.get(13)?;
            let my_team: Option<String> = row.get(14)?;
            let was_enemy = match (their_team, my_team) {
                (Some(t), Some(m)) => t != m,
                _ => false,
            };
            Ok(PlayerEncounter {
                match_id: row.get(0)?,
                map: row.get(1)?,
                queue: row.get(2)?,
                saved_at: row.get(3)?,
                agent: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                kills: row.get::<_, Option<i32>>(5)?.unwrap_or(0) as u32,
                deaths: row.get::<_, Option<i32>>(6)?.unwrap_or(0) as u32,
                assists: row.get::<_, Option<i32>>(7)?.unwrap_or(0) as u32,
                hs_pct: row.get::<_, Option<f64>>(8)?.unwrap_or(0.0) as f32,
                kd_ratio: row.get::<_, Option<f64>>(9)?.unwrap_or(0.0) as f32,
                rank_tier: row.get::<_, Option<i32>>(10)?.unwrap_or(0) as u8,
                rr: row.get::<_, Option<i32>>(11)?.unwrap_or(0),
                won: row.get::<_, Option<i32>>(12)?.map(|v| v != 0),
                was_enemy,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(VtError::Database)
    }

    /// Check whether a player's rank has climbed suspiciously fast.
    pub fn check_smurf_flag(
        &self,
        puuid: &str,
        current_tier: u8,
        threshold_tiers: u8,
        threshold_days: u32,
    ) -> Result<Option<SmurfFlag>, VtError> {
        let result: Option<(i32, i64)> = self
            .conn
            .query_row(
                "SELECT rank_tier, saved_at FROM match_players mp
                 JOIN matches m ON m.id = mp.match_id
                 WHERE mp.puuid = ?1
                 ORDER BY m.saved_at ASC
                 LIMIT 1",
                params![puuid],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(VtError::Database)?;

        let Some((first_tier_raw, first_saved_at)) = result else {
            return Ok(None);
        };

        let first_tier = first_tier_raw as u8;
        let tier_delta = current_tier as i32 - first_tier as i32;

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let days_elapsed = ((now_secs - first_saved_at) / 86400).max(0) as u32;

        let flagged =
            tier_delta >= threshold_tiers as i32 && days_elapsed <= threshold_days;

        Ok(Some(SmurfFlag {
            puuid: puuid.to_owned(),
            first_tier,
            current_tier,
            tier_delta,
            days_elapsed,
            flagged,
        }))
    }

    /// Aggregate agent performance stats for a player.
    pub fn my_agent_stats(&self, puuid: &str) -> Result<Vec<AgentStats>, VtError> {
        let mut stmt = self.conn.prepare(
            "SELECT mp.agent,
                    COUNT(DISTINCT mp.match_id) AS games,
                    AVG(CASE WHEN mp.deaths > 0 THEN mp.kills * 1.0 / mp.deaths ELSE mp.kills END) AS avg_kd,
                    AVG(mp.hs_pct) AS avg_hs,
                    SUM(CASE WHEN mp.won = 1 THEN 1.0 ELSE 0.0 END) / COUNT(*) AS win_rate
             FROM match_players mp
             WHERE mp.puuid = ?1 AND mp.agent IS NOT NULL
             GROUP BY mp.agent
             ORDER BY games DESC",
        )?;

        let rows = stmt.query_map(params![puuid], |row| {
            Ok(AgentStats {
                agent: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                games: row.get::<_, i64>(1)? as u32,
                avg_kd: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0) as f32,
                avg_hs: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0) as f32,
                win_rate: row.get::<_, Option<f64>>(4)?.unwrap_or(0.0) as f32,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(VtError::Database)
    }

    /// Aggregate map performance stats for a player.
    pub fn my_map_stats(&self, puuid: &str) -> Result<Vec<MapStats>, VtError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.map,
                    COUNT(DISTINCT m.id) AS games,
                    AVG(CASE WHEN mp.deaths > 0 THEN mp.kills * 1.0 / mp.deaths ELSE mp.kills END) AS avg_kd,
                    AVG(mp.hs_pct) AS avg_hs,
                    SUM(CASE WHEN mp.won = 1 THEN 1.0 ELSE 0.0 END) / COUNT(*) AS win_rate
             FROM match_players mp
             JOIN matches m ON m.id = mp.match_id
             WHERE mp.puuid = ?1
             GROUP BY m.map
             ORDER BY games DESC",
        )?;

        let rows = stmt.query_map(params![puuid], |row| {
            Ok(MapStats {
                map: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                games: row.get::<_, i64>(1)? as u32,
                avg_kd: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0) as f32,
                avg_hs: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0) as f32,
                win_rate: row.get::<_, Option<f64>>(4)?.unwrap_or(0.0) as f32,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(VtError::Database)
    }

    /// Find the player's nemesis — the opponent they've lost to most.
    pub fn get_nemesis(&self, puuid: &str) -> Result<Option<Nemesis>, VtError> {
        let rivalries = self.get_rivalries(puuid)?;
        Ok(rivalries.into_iter().next())
    }

    /// Full rivalry leaderboard — all recurring opponents ranked by their wins.
    pub fn get_rivalries(&self, puuid: &str) -> Result<Vec<Nemesis>, VtError> {
        // For each opponent who has appeared in ≥3 matches: count wins/losses
        let mut stmt = self.conn.prepare(
            "SELECT opp.puuid, MAX(opp.name), MAX(opp.tag),
                    COUNT(DISTINCT opp.match_id) AS encounters,
                    SUM(CASE WHEN opp.won = 1 AND opp.team != me.team THEN 1 ELSE 0 END) AS their_wins,
                    SUM(CASE WHEN me.won = 1 AND opp.team != me.team THEN 1 ELSE 0 END) AS your_wins,
                    MAX(opp.agent) AS most_agent,
                    AVG(CASE WHEN opp.deaths > 0 THEN opp.kills * 1.0 / opp.deaths ELSE opp.kills END) AS avg_kd
             FROM match_players me
             JOIN match_players opp ON opp.match_id = me.match_id AND opp.puuid != me.puuid
             WHERE me.puuid = ?1
             GROUP BY opp.puuid
             HAVING encounters >= 3
             ORDER BY their_wins DESC",
        )?;

        let rows = stmt.query_map(params![puuid], |row| {
            Ok(Nemesis {
                puuid: row.get(0)?,
                name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                tag: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                encounters: row.get::<_, i64>(3)? as u32,
                their_wins: row.get::<_, i64>(4)? as u32,
                your_wins: row.get::<_, i64>(5)? as u32,
                most_played_agent: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                avg_kd_vs_you: row.get::<_, Option<f64>>(7)?.unwrap_or(0.0) as f32,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(VtError::Database)
    }
}

/// Compute a summary over a player's encounter history.
pub fn summarize_encounters(encounters: &[PlayerEncounter]) -> EncounterSummary {
    if encounters.is_empty() {
        return EncounterSummary {
            total_meetings: 0,
            wins_against: 0,
            losses_against: 0,
            most_played_agent: String::new(),
            worst_game: None,
            avg_kills: 0.0,
            avg_deaths: 0.0,
            avg_hs_pct: 0.0,
            rank_delta: 0,
        };
    }

    let total = encounters.len() as u32;
    let wins_against = encounters
        .iter()
        .filter(|e| e.was_enemy && e.won == Some(true))
        .count() as u32;
    let losses_against = encounters
        .iter()
        .filter(|e| e.was_enemy && e.won == Some(false))
        .count() as u32;

    // Most played agent
    let mut agent_counts: std::collections::HashMap<&str, u32> =
        std::collections::HashMap::new();
    for e in encounters {
        *agent_counts.entry(e.agent.as_str()).or_default() += 1;
    }
    let most_played_agent = agent_counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(a, _)| a.to_owned())
        .unwrap_or_default();

    // Worst game (lowest K/D, minimum kills / most deaths)
    let worst_game = encounters
        .iter()
        .min_by(|a, b| {
            let kd_a = if a.deaths > 0 { a.kills as f32 / a.deaths as f32 } else { a.kills as f32 };
            let kd_b = if b.deaths > 0 { b.kills as f32 / b.deaths as f32 } else { b.kills as f32 };
            kd_a.partial_cmp(&kd_b).unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned();

    let avg_kills = encounters.iter().map(|e| e.kills as f32).sum::<f32>() / total as f32;
    let avg_deaths = encounters.iter().map(|e| e.deaths as f32).sum::<f32>() / total as f32;
    let avg_hs_pct = encounters.iter().map(|e| e.hs_pct).sum::<f32>() / total as f32;

    let first_tier = encounters.last().map(|e| e.rank_tier as i32).unwrap_or(0);
    let last_tier = encounters.first().map(|e| e.rank_tier as i32).unwrap_or(0);
    let rank_delta = last_tier - first_tier;

    EncounterSummary {
        total_meetings: total,
        wins_against,
        losses_against,
        most_played_agent,
        worst_game,
        avg_kills,
        avg_deaths,
        avg_hs_pct,
        rank_delta,
    }
}

// ── rusqlite optional helper ──────────────────────────────────────────────────

trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

// ── Phase 6: Extended analytics ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MyMatchResult {
    pub match_id:  String,
    pub saved_at:  i64,
    pub map:       String,
    pub queue:     String,
    pub agent:     String,
    pub kills:     u32,
    pub deaths:    u32,
    pub assists:   u32,
    pub hs_pct:    f32,
    pub kd_ratio:  f32,
    pub rank_tier: u8,
    pub rr:        i32,
    pub rr_delta:  i32,
    pub won:       Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PartyWinRate {
    pub party_size: u8,
    pub games:      u32,
    pub win_rate:   f32,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id:          i64,
    pub started_at:  i64,
    pub ended_at:    i64,
    pub games:       u32,
    pub wins:        u32,
    pub rr_delta:    i32,
    /// Game number within the session where win/loss streak first flipped.
    pub tilt_point:  Option<u32>,
}

#[derive(Debug, Clone)]
pub struct HourlyStats {
    pub hour:     u8,
    pub games:    u32,
    pub win_rate: f32,
    pub avg_kd:   f32,
}

impl MatchHistory {
    /// Your own match results (newest first), optionally filtered by queue.
    pub fn my_match_history(
        &self,
        puuid: &str,
        queue: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MyMatchResult>, VtError> {
        let sql = if queue.is_some() {
            "SELECT m.id, m.saved_at, m.map, m.queue, mp.agent,
                    mp.kills, mp.deaths, mp.assists, mp.hs_pct, mp.kd_ratio,
                    mp.rank_tier, mp.rr, m.my_rr_delta, mp.won
             FROM matches m
             JOIN match_players mp ON mp.match_id = m.id AND mp.puuid = ?1
             WHERE m.queue = ?3
             ORDER BY m.saved_at DESC
             LIMIT ?2"
        } else {
            "SELECT m.id, m.saved_at, m.map, m.queue, mp.agent,
                    mp.kills, mp.deaths, mp.assists, mp.hs_pct, mp.kd_ratio,
                    mp.rank_tier, mp.rr, m.my_rr_delta, mp.won
             FROM matches m
             JOIN match_players mp ON mp.match_id = m.id AND mp.puuid = ?1
             ORDER BY m.saved_at DESC
             LIMIT ?2"
        };

        let mut stmt = self.conn.prepare(sql)?;

        let map_row = |row: &rusqlite::Row| {
            Ok(MyMatchResult {
                match_id:  row.get(0)?,
                saved_at:  row.get(1)?,
                map:       row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                queue:     row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                agent:     row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                kills:     row.get::<_, Option<i32>>(5)?.unwrap_or(0) as u32,
                deaths:    row.get::<_, Option<i32>>(6)?.unwrap_or(0) as u32,
                assists:   row.get::<_, Option<i32>>(7)?.unwrap_or(0) as u32,
                hs_pct:    row.get::<_, Option<f64>>(8)?.unwrap_or(0.0) as f32,
                kd_ratio:  row.get::<_, Option<f64>>(9)?.unwrap_or(0.0) as f32,
                rank_tier: row.get::<_, Option<i32>>(10)?.unwrap_or(0) as u8,
                rr:        row.get::<_, Option<i32>>(11)?.unwrap_or(0),
                rr_delta:  row.get::<_, Option<i32>>(12)?.unwrap_or(0),
                won:       row.get::<_, Option<i32>>(13)?.map(|v| v != 0),
            })
        };

        let rows = if let Some(q) = queue {
            stmt.query_map(rusqlite::params![puuid, limit as i64, q], map_row)?
        } else {
            stmt.query_map(rusqlite::params![puuid, limit as i64], map_row)?
        };

        rows.collect::<Result<Vec<_>, _>>().map_err(VtError::Database)
    }

    /// RR over time — returns `(unix_timestamp, rr)` pairs ordered oldest first.
    pub fn my_rr_timeline(&self, puuid: &str) -> Result<Vec<(i64, i32)>, VtError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.saved_at, mp.rr
             FROM matches m
             JOIN match_players mp ON mp.match_id = m.id AND mp.puuid = ?1
             ORDER BY m.saved_at ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![puuid], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i32>>(1)?.unwrap_or(0)))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(VtError::Database)
    }

    /// Win rate broken down by how many people you queued with (1 = solo, 5 = full stack).
    pub fn my_party_winrates(&self, puuid: &str) -> Result<Vec<PartyWinRate>, VtError> {
        let mut stmt = self.conn.prepare(
            "SELECT party_size_bucket,
                    COUNT(*)  AS games,
                    SUM(CASE WHEN won = 1 THEN 1.0 ELSE 0.0 END) / COUNT(*) AS win_rate
             FROM (
                 SELECT m.id, mp.won,
                        (SELECT COUNT(*) FROM match_players mp2
                         WHERE mp2.match_id = m.id
                           AND mp2.party_id = mp.party_id
                           AND mp2.party_id != '') AS party_size_bucket
                 FROM matches m
                 JOIN match_players mp ON mp.match_id = m.id AND mp.puuid = ?1
             )
             WHERE party_size_bucket > 0
             GROUP BY party_size_bucket
             ORDER BY party_size_bucket",
        )?;
        let rows = stmt.query_map(rusqlite::params![puuid], |row| {
            Ok(PartyWinRate {
                party_size: row.get::<_, i64>(0)? as u8,
                games:      row.get::<_, i64>(1)? as u32,
                win_rate:   row.get::<_, f64>(2)? as f32,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(VtError::Database)
    }

    /// Group matches into sessions (gap > 60 min = new session).
    pub fn my_sessions(&self, puuid: &str) -> Result<Vec<Session>, VtError> {
        // Fetch all matches ordered by time
        let history = self.my_match_history(puuid, None, 10_000)?;
        if history.is_empty() {
            return Ok(Vec::new());
        }

        const SESSION_GAP_SECS: i64 = 3600; // 60 minutes
        let mut sessions: Vec<Session> = Vec::new();
        let mut session_id = 1i64;
        let mut matches_rev: Vec<_> = history.into_iter().rev().collect(); // oldest first

        let mut cur_games: Vec<MyMatchResult> = Vec::new();

        for m in matches_rev.drain(..) {
            if cur_games.is_empty() {
                cur_games.push(m);
            } else {
                let prev_ts = cur_games.last().unwrap().saved_at;
                if m.saved_at - prev_ts > SESSION_GAP_SECS {
                    // Flush current session
                    sessions.push(build_session(session_id, &cur_games));
                    session_id += 1;
                    cur_games.clear();
                }
                cur_games.push(m);
            }
        }
        if !cur_games.is_empty() {
            sessions.push(build_session(session_id, &cur_games));
        }

        // Return newest session first
        sessions.reverse();
        Ok(sessions)
    }

    /// Win rate and K/D broken down by hour of day (0-23).
    pub fn my_hourly_stats(&self, puuid: &str) -> Result<Vec<HourlyStats>, VtError> {
        let mut stmt = self.conn.prepare(
            "SELECT (m.saved_at / 3600) % 24 AS hour,
                    COUNT(*) AS games,
                    SUM(CASE WHEN mp.won = 1 THEN 1.0 ELSE 0.0 END) / COUNT(*) AS win_rate,
                    AVG(CASE WHEN mp.deaths > 0 THEN mp.kills * 1.0 / mp.deaths ELSE mp.kills END) AS avg_kd
             FROM matches m
             JOIN match_players mp ON mp.match_id = m.id AND mp.puuid = ?1
             GROUP BY hour
             ORDER BY hour",
        )?;
        let rows = stmt.query_map(rusqlite::params![puuid], |row| {
            Ok(HourlyStats {
                hour:     row.get::<_, i64>(0)? as u8,
                games:    row.get::<_, i64>(1)? as u32,
                win_rate: row.get::<_, f64>(2)? as f32,
                avg_kd:   row.get::<_, Option<f64>>(3)?.unwrap_or(0.0) as f32,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(VtError::Database)
    }
}

fn build_session(id: i64, matches: &[MyMatchResult]) -> Session {
    let started_at = matches.first().map(|m| m.saved_at).unwrap_or(0);
    let ended_at   = matches.last().map(|m| m.saved_at).unwrap_or(0);
    let games      = matches.len() as u32;
    let wins       = matches.iter().filter(|m| m.won == Some(true)).count() as u32;
    let rr_delta   = matches.iter().map(|m| m.rr_delta).sum::<i32>();

    // Tilt point: first game where the running streak flipped direction
    let mut tilt_point = None;
    let mut last_won: Option<bool> = None;
    let mut streak_start_won: Option<bool> = None;
    for (i, m) in matches.iter().enumerate() {
        let w = m.won.unwrap_or(false);
        if streak_start_won.is_none() {
            streak_start_won = Some(w);
            last_won = Some(w);
        } else if Some(w) != last_won {
            // streak flipped
            if Some(w) != streak_start_won {
                tilt_point = Some(i as u32 + 1);
                break;
            }
            last_won = Some(w);
        }
    }

    Session { id, started_at, ended_at, games, wins, rr_delta, tilt_point }
}
