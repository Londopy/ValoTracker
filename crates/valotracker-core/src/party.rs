use std::collections::HashMap;

use crate::presence::PlayerPresence;

/// A group of players queuing together, identified by a shared `partyId`.
#[derive(Debug, Clone)]
pub struct PartyGroup {
    pub party_id: String,
    /// PUUIDs of all members in this party.
    pub members: Vec<String>,
    pub size: usize,
    /// Set to `true` after team assignment reveals this is an enemy group.
    pub is_enemy: bool,
}

/// Build a map of `partyId → PartyGroup` from the current presence list.
///
/// Players without a private blob (offline / unknown) are grouped under their
/// own PUUID as a solo party.
pub fn build_party_map(presences: &[PlayerPresence]) -> HashMap<String, PartyGroup> {
    let mut map: HashMap<String, PartyGroup> = HashMap::new();

    for presence in presences {
        let party_id = presence
            .private
            .as_ref()
            .map(|p| p.party_id.clone())
            .unwrap_or_else(|| presence.puuid.clone());

        let entry = map.entry(party_id.clone()).or_insert_with(|| PartyGroup {
            party_id: party_id.clone(),
            members: Vec::new(),
            size: 0,
            is_enemy: false,
        });

        if !entry.members.contains(&presence.puuid) {
            entry.members.push(presence.puuid.clone());
            entry.size += 1;
        }
    }

    map
}

/// Assign a display icon to each unique party.
///
/// Solo parties (size == 1) get a plain `·`.
/// Multi-player parties get cycling symbols: ★ ▲ ● ■ ◆
pub fn assign_party_icons(groups: &HashMap<String, PartyGroup>) -> HashMap<String, char> {
    const ICONS: &[char] = &['★', '▲', '●', '■', '◆'];

    // Collect multi-player groups, sorted by first member for stable output.
    let mut multi: Vec<&PartyGroup> = groups.values().filter(|g| g.size > 1).collect();
    multi.sort_by_key(|g| &g.party_id);

    let mut icons: HashMap<String, char> = HashMap::new();
    let mut icon_idx = 0;

    for group in &multi {
        let icon = ICONS[icon_idx % ICONS.len()];
        icons.insert(group.party_id.clone(), icon);
        icon_idx += 1;
    }

    // Solo parties — just a dot
    for group in groups.values().filter(|g| g.size == 1) {
        icons.entry(group.party_id.clone()).or_insert('·');
    }

    icons
}

/// Mark party groups that belong to the enemy team.
///
/// `ally_team` is `"Blue"` or `"Red"`.
/// `player_teams` maps PUUID → team string.
pub fn tag_enemy_parties(
    groups: &mut HashMap<String, PartyGroup>,
    ally_team: &str,
    player_teams: &HashMap<String, String>,
) {
    for group in groups.values_mut() {
        // A group is enemy if ANY of its members are on the opposing team.
        let is_enemy = group.members.iter().any(|puuid| {
            player_teams
                .get(puuid)
                .map(|t| t != ally_team)
                .unwrap_or(false)
        });
        group.is_enemy = is_enemy;
    }
}
