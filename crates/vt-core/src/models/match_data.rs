use crate::{models::player::ResolvedPlayer, state::GameState};

/// A fully assembled snapshot of the current match.
///
/// Built by the engine once all async fetches complete.
#[derive(Debug, Clone)]
pub struct MatchSnapshot {
    pub game_state: GameState,
    pub match_id: String,
    pub map_name: String,
    pub queue_id: String,
    /// Server label, e.g. `"EU-WEST"`.
    pub server: String,
    pub players: Vec<ResolvedPlayer>,
    /// The local player's PUUID.
    pub my_puuid: String,
    /// Timestamp when this snapshot was assembled.
    pub fetched_at: std::time::Instant,
}

impl MatchSnapshot {
    /// Players on the local player's team.
    pub fn ally_team(&self) -> impl Iterator<Item = &ResolvedPlayer> {
        self.players.iter().filter(|p| p.is_ally)
    }

    /// Players on the opposing team.
    pub fn enemy_team(&self) -> impl Iterator<Item = &ResolvedPlayer> {
        self.players.iter().filter(|p| !p.is_ally)
    }

    /// How long ago this snapshot was fetched.
    pub fn age(&self) -> std::time::Duration {
        self.fetched_at.elapsed()
    }
}
