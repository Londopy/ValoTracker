use std::collections::HashMap;

use reqwest::Client;

use crate::{
    agents,
    auth::Auth,
    coregame,
    error::ValoTrackerError,
    lockfile::Lockfile,
    models::{match_data::MatchSnapshot, player::ResolvedPlayer},
    names, party, pregame, presence,
    rank::{self, RankCache},
    state::GameState,
    stats,
};

/// High-level engine that builds a [`MatchSnapshot`] from the current game state.
pub struct Engine {
    pub lockfile: Lockfile,
    pub auth: Auth,
    pub local_client: Client,
    pub remote_client: Client,
    rank_cache: RankCache,
}

impl Engine {
    /// Initialise the engine from a running VALORANT instance.
    pub async fn init() -> Result<Self, ValoTrackerError> {
        let lockfile = Lockfile::read()?;
        let local_client = crate::client::build_local_client(&lockfile)?;
        let auth = Auth::from_lockfile(&lockfile, &local_client).await?;
        let remote_client = crate::client::build_remote_client(&auth)?;

        Ok(Self {
            lockfile,
            auth,
            local_client,
            remote_client,
            rank_cache: RankCache::new(),
        })
    }

    /// Check presence and determine the current game state.
    pub async fn poll_game_state(&self) -> Result<GameState, ValoTrackerError> {
        let presences = presence::get_presences(&self.local_client, &self.lockfile).await?;
        Ok(presence::get_game_state(&presences, &self.auth.puuid))
    }

    /// Build a full [`MatchSnapshot`] for the current match.
    ///
    /// Works for both Pregame and Ingame states.  The queue ID and map name
    /// are read from the player's presence blob so they reflect the actual
    /// mode (Swiftplay, Deathmatch, custom, etc.) rather than a hardcoded
    /// placeholder.
    pub async fn build_snapshot(&mut self) -> Result<MatchSnapshot, ValoTrackerError> {
        // 1. Determine game state, queue ID, and map name from presence
        let presences = presence::get_presences(&self.local_client, &self.lockfile).await?;
        let game_state = presence::get_game_state(&presences, &self.auth.puuid);
        let (queue_id, map_name) = presence::get_match_meta(&presences, &self.auth.puuid);

        let (match_id, raw_players): (String, Vec<RawPlayer>) = match &game_state {
            GameState::Pregame { .. } => {
                let (mid, players) =
                    pregame::get_pregame_players(&self.remote_client, &self.auth, &self.auth.puuid)
                        .await?;
                let raws = players
                    .into_iter()
                    .map(|p| RawPlayer {
                        puuid: p.puuid,
                        character_id: p.character_id,
                        team_id: "Blue".to_owned(), // ally team only in pregame
                        incognito: p.incognito,
                        hide_account_level: p.hide_account_level,
                        account_level: p.account_level,
                        game_pod_id: String::new(),
                    })
                    .collect();
                (mid, raws)
            }
            GameState::Ingame { .. } => {
                let (mid, players) =
                    coregame::get_ingame_players(&self.remote_client, &self.auth, &self.auth.puuid)
                        .await?;
                let raws = players
                    .into_iter()
                    .map(|p| RawPlayer {
                        puuid: p.puuid,
                        character_id: p.character_id,
                        team_id: p.team_id,
                        incognito: p.incognito,
                        hide_account_level: p.hide_account_level,
                        account_level: p.account_level,
                        game_pod_id: p.game_pod_id,
                    })
                    .collect();
                (mid, raws)
            }
            _ => return Err(ValoTrackerError::NotInMatch),
        };

        let server = raw_players
            .first()
            .map(|p| p.game_pod_id.clone())
            .unwrap_or_default();

        // 2. Determine my team
        let my_team = raw_players
            .iter()
            .find(|p| p.puuid == self.auth.puuid)
            .map(|p| p.team_id.clone())
            .unwrap_or_else(|| "Blue".to_owned());

        // 3. Resolve display names
        let puuids: Vec<String> = raw_players.iter().map(|p| p.puuid.clone()).collect();
        let names = names::fetch_names(&self.remote_client, &self.auth, &puuids).await?;

        // 4. Fetch ranks (rate-limited, cached)
        let ranks = self.fetch_ranks(&puuids, &match_id).await;

        // 5. Fetch stats concurrently
        let stats_futures: Vec<_> = puuids
            .iter()
            .map(|puuid| stats::get_player_stats(&self.remote_client, &self.auth, puuid))
            .collect();
        let stats_results = futures::future::join_all(stats_futures).await;
        let player_stats: HashMap<String, stats::PlayerStats> = puuids
            .iter()
            .zip(stats_results)
            .map(|(puuid, res)| (puuid.clone(), res.unwrap_or_default()))
            .collect();

        // 6. Build party map
        let party_map = party::build_party_map(&presences);
        let party_icons = party::assign_party_icons(&party_map);
        let player_teams: HashMap<String, String> = raw_players
            .iter()
            .map(|p| (p.puuid.clone(), p.team_id.clone()))
            .collect();
        let mut party_map_mut = party_map.clone();
        party::tag_enemy_parties(&mut party_map_mut, &my_team, &player_teams);

        // 7. Assemble ResolvedPlayer list
        let players: Vec<ResolvedPlayer> = raw_players
            .iter()
            .map(|raw| {
                assemble_player(
                    raw,
                    &my_team,
                    &names,
                    &ranks,
                    &player_stats,
                    &party_map_mut,
                    &party_icons,
                )
            })
            .collect();

        Ok(MatchSnapshot {
            game_state,
            match_id,
            map_name,
            queue_id,
            server,
            players,
            my_puuid: self.auth.puuid.clone(),
            fetched_at: std::time::Instant::now(),
        })
    }

    /// Fetch ranks for all `puuids`, honouring the per-match cache and
    /// inserting a 100 ms delay between uncached network calls to avoid
    /// rate-limiting.
    async fn fetch_ranks(
        &mut self,
        puuids: &[String],
        match_id: &str,
    ) -> HashMap<String, rank::PlayerRank> {
        let mut ranks: HashMap<String, rank::PlayerRank> = HashMap::new();
        for (i, puuid) in puuids.iter().enumerate() {
            if let Some(cached) = self.rank_cache.get(puuid, match_id) {
                ranks.insert(puuid.clone(), cached.clone());
            } else {
                if i > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                match rank::get_player_rank(&self.remote_client, &self.auth, puuid).await {
                    Ok(r) => {
                        self.rank_cache.insert(puuid, match_id, r.clone());
                        ranks.insert(puuid.clone(), r);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch rank for {puuid}: {e}");
                        ranks.insert(puuid.clone(), rank::PlayerRank::default());
                    }
                }
            }
        }
        ranks
    }
}

// ── Internal helpers ─────────────────────────────────────────────────────────

struct RawPlayer {
    puuid: String,
    character_id: String,
    team_id: String,
    incognito: bool,
    hide_account_level: bool,
    account_level: u32,
    game_pod_id: String,
}

/// Convert one `RawPlayer` into a fully resolved `ResolvedPlayer`.
fn assemble_player(
    raw: &RawPlayer,
    my_team: &str,
    names: &HashMap<String, names::PlayerName>,
    ranks: &HashMap<String, rank::PlayerRank>,
    player_stats: &HashMap<String, stats::PlayerStats>,
    party_map: &HashMap<String, party::PartyGroup>,
    party_icons: &HashMap<String, char>,
) -> ResolvedPlayer {
    let name = names.get(&raw.puuid);
    let rank = ranks.get(&raw.puuid).cloned().unwrap_or_default();
    let stat = player_stats.get(&raw.puuid).cloned().unwrap_or_default();

    let (party_id, party_icon, party_size, is_enemy_party) = party_map
        .values()
        .find(|g| g.members.contains(&raw.puuid))
        .map(|g| {
            let icon = party_icons.get(&g.party_id).copied().unwrap_or('·');
            (g.party_id.clone(), icon, g.size as u8, g.is_enemy)
        })
        .unwrap_or_else(|| (raw.puuid.clone(), '·', 1, raw.team_id != my_team));

    let display_name = name
        .map(|n| n.display.clone())
        .unwrap_or_else(|| "[Unknown]".to_owned());

    ResolvedPlayer {
        puuid: raw.puuid.clone(),
        game_name: name.map(|n| n.game_name.clone()).unwrap_or_default(),
        tag_line: name.map(|n| n.tag_line.clone()).unwrap_or_default(),
        display_name,
        incognito: raw.incognito,
        hide_account_level: raw.hide_account_level,
        account_level: raw.account_level,
        team_id: raw.team_id.clone(),
        is_ally: raw.team_id == my_team,
        character_id: raw.character_id.clone(),
        agent_name: agents::resolve_agent_name(&raw.character_id),
        rank,
        stats: stat,
        party_id,
        party_icon,
        party_size,
        is_enemy_party,
        times_seen: 0, // filled in by history module
        game_pod_id: raw.game_pod_id.clone(),
    }
}
