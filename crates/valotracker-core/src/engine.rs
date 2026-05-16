use std::collections::HashMap;

use reqwest::Client;

use crate::{
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
    /// Works for both Pregame and Ingame states.
    pub async fn build_snapshot(
        &mut self,
        map_name: String,
        queue_id: String,
    ) -> Result<MatchSnapshot, ValoTrackerError> {
        // 1. Determine game state
        let presences = presence::get_presences(&self.local_client, &self.lockfile).await?;
        let game_state = presence::get_game_state(&presences, &self.auth.puuid);

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

        // 4. Fetch ranks concurrently (rate-limit: ~100ms spacing)
        let mut ranks: HashMap<String, rank::PlayerRank> = HashMap::new();
        for (i, puuid) in puuids.iter().enumerate() {
            if let Some(cached) = self.rank_cache.get(puuid, &match_id) {
                ranks.insert(puuid.clone(), cached.clone());
            } else {
                if i > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                match rank::get_player_rank(&self.remote_client, &self.auth, puuid).await {
                    Ok(r) => {
                        self.rank_cache.insert(puuid, &match_id, r.clone());
                        ranks.insert(puuid.clone(), r);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch rank for {puuid}: {e}");
                        ranks.insert(puuid.clone(), rank::PlayerRank::default());
                    }
                }
            }
        }

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
                let name = names.get(&raw.puuid);
                let rank = ranks.get(&raw.puuid).cloned().unwrap_or_default();
                let stat = player_stats.get(&raw.puuid).cloned().unwrap_or_default();

                // Find the party this player belongs to
                let (party_id, party_icon, party_size, is_enemy_party) = party_map_mut
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
                    agent_name: resolve_agent_name(&raw.character_id),
                    rank,
                    stats: stat,
                    party_id,
                    party_icon,
                    party_size,
                    is_enemy_party,
                    times_seen: 0, // filled in by history module
                    game_pod_id: raw.game_pod_id.clone(),
                }
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

/// Resolve a VALORANT agent UUID to its display name.
///
/// This uses a hardcoded lookup table. For a fully dynamic solution you'd
/// fetch from `https://valorant-api.com/v1/agents`.
fn resolve_agent_name(character_id: &str) -> String {
    // Map is lowercased UUID → agent name
    let map: &[(&str, &str)] = &[
        ("e370fa57-4757-3604-3648-499e1f642d3f", "Gekko"),
        ("dade69b4-4f5a-8528-247b-219e5a1facd6", "Fade"),
        ("5f8d3a7f-467b-97f3-062c-13acf203c006", "Breach"),
        ("cc8b64c8-4b25-4ff9-6e7f-37b4da43d235", "Deadlock"),
        ("f94c3b30-42be-e959-889c-5aa313dba261", "Raze"),
        ("22697a3d-45bf-8dd7-4fec-84a9e28c69d7", "Chamber"),
        ("601dbbe7-43ce-be57-2a40-4abd24953621", "KAY/O"),
        ("6f2a04ca-43e0-be17-7f36-b3908627744d", "Skye"),
        ("117ed9e3-49f3-6512-3ccf-0cada7e3823b", "Cypher"),
        ("320b2a48-4d9b-a075-30f1-1f93a9b638fa", "Sova"),
        ("1dbf2edd-4729-0984-3115-daa5eed44993", "Killjoy"),
        ("95b78ed7-4637-86d9-7e41-71ba8c293152", "Harbor"),
        ("7f94d92c-4234-0a36-9646-3a87eb8b5edc", "Viper"),
        ("eb93336a-449b-9c1b-0a54-a891f7921d69", "Phoenix"),
        ("41fb69c1-4189-7b37-f117-bcaf1e96f1bf", "Astra"),
        ("9f0d8ba9-4140-b941-57d3-a7ad57c6b417", "Brimstone"),
        ("0e38b510-41a8-5780-5e8f-568b2a4f2d6c", "Iso"),
        ("bb2a4828-46eb-8cd1-e765-15848195d751", "Neon"),
        ("8e253930-4c05-31dd-1b6c-968525494517", "Omen"),
        ("1e58de9c-4950-5125-93e9-a0aee9f98746", "Clove"),
        ("dea89a98-4c10-36a5-8d26-9db77a2b7a5e", "Waylay"),
        ("569fdd95-4d10-43ab-ca70-79becc718b46", "Sage"),
        ("a3bfb853-43b2-7238-a4f1-ad90e9e46bcc", "Reyna"),
        ("707eab51-4836-f488-046a-cda6bf494859", "Viper"),
        ("8370acf1-4667-35f5-310b-958a3defdba3", "Tejo"),
        ("7e73a75c-465b-8870-5647-9950a4788de1", "Vyse"),
        ("efba5359-4016-a1e5-7626-b1ae976b0e68", "Yoru"),
        ("add6443a-41bd-e414-f6ad-e58d267f4e95", "Jett"),
        ("f0767e4e-97f1-4d22-b7b2-08a7aef7afc5", "Smoked Out"), // Clove variant
    ];

    let id_lower = character_id.to_lowercase();
    map.iter()
        .find(|(uuid, _)| *uuid == id_lower)
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| "Unknown".to_owned())
}
