/// Represents the current phase of a VALORANT session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameState {
    /// Player is in the main menus (or lobby), not in any match.
    Menu,
    /// Agent-select / pre-game lobby for an upcoming match.
    Pregame { match_id: String },
    /// An active in-progress match.
    Ingame { match_id: String },
    /// VALORANT is not running or presence could not be read.
    Disconnected,
}

impl GameState {
    /// Returns the match ID if in Pregame or Ingame state.
    pub fn match_id(&self) -> Option<&str> {
        match self {
            Self::Pregame { match_id } | Self::Ingame { match_id } => Some(match_id),
            _ => None,
        }
    }

    /// True if the player is in any match phase (pregame or ingame).
    pub fn is_in_match(&self) -> bool {
        matches!(self, Self::Pregame { .. } | Self::Ingame { .. })
    }

    /// Returns a short human-readable label for display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Menu => "Menu",
            Self::Pregame { .. } => "Agent Select",
            Self::Ingame { .. } => "In Game",
            Self::Disconnected => "Disconnected",
        }
    }
}

impl std::fmt::Display for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}
