use crate::{rank::PlayerRank, stats::PlayerStats};

/// A fully resolved player combining all data sources:
/// identity, rank, stats, party, and agent info.
///
/// This is what the UI layer works with — the core engine assembles it
/// from the individual module results.
#[derive(Debug, Clone, Default)]
pub struct ResolvedPlayer {
    // ── Identity ─────────────────────────────────────────────────────────────
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
    /// `"name#tag"` for display.
    pub display_name: String,
    pub incognito: bool,
    pub hide_account_level: bool,
    pub account_level: u32,

    // ── Team ─────────────────────────────────────────────────────────────────
    /// `"Blue"` or `"Red"`.
    pub team_id: String,
    pub is_ally: bool,

    // ── Agent ────────────────────────────────────────────────────────────────
    pub character_id: String,
    pub agent_name: String,

    // ── Rank ─────────────────────────────────────────────────────────────────
    pub rank: PlayerRank,

    // ── Stats ─────────────────────────────────────────────────────────────────
    pub stats: PlayerStats,

    // ── Party ─────────────────────────────────────────────────────────────────
    pub party_id: String,
    pub party_icon: char,
    pub party_size: u8,
    pub is_enemy_party: bool,

    // ── History ───────────────────────────────────────────────────────────────
    /// Times you've been in a match with this player (from local history DB).
    pub times_seen: u32,

    // ── Server ───────────────────────────────────────────────────────────────
    pub game_pod_id: String,
}

impl ResolvedPlayer {
    /// Returns the name to display, respecting streamer mode.
    pub fn display_name(&self) -> &str {
        if self.incognito {
            "[S]"
        } else if self.display_name.is_empty() {
            "Unknown"
        } else {
            &self.display_name
        }
    }

    /// Returns a short server label extracted from the game pod ID.
    ///
    /// e.g. `"aresriot.aws-rclusterprod-euw1-1.eu-gp-frankfurt-1"` → `"EU-WEST"`
    pub fn server_label(&self) -> &str {
        if self.game_pod_id.contains("euw") {
            "EU-WEST"
        } else if self.game_pod_id.contains("eun") {
            "EU-NORTH"
        } else if self.game_pod_id.contains("na") {
            "NA"
        } else if self.game_pod_id.contains("ap") {
            "AP"
        } else if self.game_pod_id.contains("kr") {
            "KR"
        } else if self.game_pod_id.contains("br") {
            "BR"
        } else if self.game_pod_id.contains("latam") {
            "LATAM"
        } else {
            &self.game_pod_id
        }
    }
}
