use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use valotracker_core::{
    engine::Engine,
    rank::tier_to_name,
    state::GameState,
};

// ── PyPlayer ──────────────────────────────────────────────────────────────────

/// A fully resolved VALORANT player exposed to Python.
#[pyclass]
pub struct PyPlayer {
    inner: valotracker_core::ResolvedPlayer,
}

#[pymethods]
impl PyPlayer {
    /// Display name (`"name#tag"`, or `"[S]"` in streamer mode).
    #[getter]
    fn name(&self) -> &str {
        self.inner.display_name()
    }

    /// Riot tag line (the part after `#`).
    #[getter]
    fn tag(&self) -> &str {
        &self.inner.tag_line
    }

    /// Competitive tier index (0 = Unranked … 27 = Radiant).
    #[getter]
    fn rank_tier(&self) -> u8 {
        self.inner.rank.tier
    }

    /// Human-readable rank name, e.g. `"Gold 2"`.
    #[getter]
    fn rank_name(&self) -> &str {
        tier_to_name(self.inner.rank.tier)
    }

    /// Current ranked rating (0-100, or higher for Immortal/Radiant).
    #[getter]
    fn rr(&self) -> i32 {
        self.inner.rank.rr
    }

    /// All-time peak tier index.
    #[getter]
    fn peak_tier(&self) -> u8 {
        self.inner.rank.peak_tier
    }

    /// Team ID: `"Blue"` (ally) or `"Red"` (enemy).
    #[getter]
    fn team(&self) -> &str {
        &self.inner.team_id
    }

    /// `True` if this player is on the same team as the local player.
    #[getter]
    fn is_ally(&self) -> bool {
        self.inner.is_ally
    }

    /// Opaque party identifier string.
    #[getter]
    fn party_id(&self) -> &str {
        &self.inner.party_id
    }

    /// Number of players in this player's party.
    #[getter]
    fn party_size(&self) -> u8 {
        self.inner.party_size
    }

    /// Single-character party icon (Unicode, e.g. `"★"`).
    #[getter]
    fn party_icon(&self) -> String {
        self.inner.party_icon.to_string()
    }

    /// `True` if the player has streamer mode (incognito) enabled.
    #[getter]
    fn incognito(&self) -> bool {
        self.inner.incognito
    }

    /// Average headshot percentage over recent games (0.0–1.0).
    #[getter]
    fn headshot_pct(&self) -> f32 {
        self.inner.stats.headshot_pct
    }

    /// Kill/death ratio over recent games.
    #[getter]
    fn kd_ratio(&self) -> f32 {
        self.inner.stats.kd_ratio
    }

    /// Win rate over recent games (0.0–1.0).
    #[getter]
    fn win_rate(&self) -> f32 {
        self.inner.stats.win_rate
    }

    /// Account level (0 if hidden).
    #[getter]
    fn account_level(&self) -> u32 {
        self.inner.account_level
    }

    /// Number of times the local player has been in a match with this player.
    #[getter]
    fn times_seen(&self) -> u32 {
        self.inner.times_seen
    }

    /// Agent display name, e.g. `"Jett"`.
    #[getter]
    fn agent(&self) -> &str {
        &self.inner.agent_name
    }

    fn __repr__(&self) -> String {
        format!(
            "PyPlayer(name={:?}, agent={:?}, rank={:?}, team={:?})",
            self.inner.display_name(),
            self.inner.agent_name,
            tier_to_name(self.inner.rank.tier),
            self.inner.team_id,
        )
    }
}

// ── ValoTrackerClient ──────────────────────────────────────────────────────────────────

/// Async-capable VALORANT tracker client.
///
/// Wraps a Tokio runtime so that all async engine calls are driven
/// synchronously from Python.
#[pyclass]
pub struct ValoTrackerClient {
    rt: tokio::runtime::Runtime,
    engine: Engine,
}

#[pymethods]
impl ValoTrackerClient {
    /// Create a new client, connecting to the running VALORANT instance.
    ///
    /// Raises `RuntimeError` if VALORANT is not running or auth fails.
    #[new]
    fn new() -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let engine = rt
            .block_on(Engine::init())
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { rt, engine })
    }

    /// Return the current game state as a string.
    ///
    /// Returns one of: `"Menu"`, `"Pregame"`, `"Ingame"`, `"Disconnected"`.
    fn get_game_state(&self) -> PyResult<String> {
        let state = self
            .rt
            .block_on(self.engine.poll_game_state())
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let label = match state {
            GameState::Menu => "Menu",
            GameState::Pregame { .. } => "Pregame",
            GameState::Ingame { .. } => "Ingame",
            GameState::Disconnected => "Disconnected",
        };
        Ok(label.to_owned())
    }

    /// Fetch all players in the current match.
    ///
    /// Works for both Pregame (agent select) and Ingame phases.
    /// Raises `RuntimeError` if the player is not in a match.
    fn get_players(&mut self) -> PyResult<Vec<PyPlayer>> {
        let snapshot = self
            .rt
            .block_on(self.engine.build_snapshot(
                String::new(), // map_name — unknown without extra fetch
                String::new(), // queue_id — unknown without extra fetch
            ))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let players = snapshot
            .players
            .into_iter()
            .map(|p| PyPlayer { inner: p })
            .collect();

        Ok(players)
    }

    /// Block until the local player enters Pregame or Ingame, polling every 2 seconds.
    ///
    /// Returns immediately if already in a match phase.
    fn wait_for_match(&self) -> PyResult<()> {
        self.rt
            .block_on(async {
                loop {
                    match self.engine.poll_game_state().await? {
                        GameState::Pregame { .. } | GameState::Ingame { .. } => return Ok(()),
                        _ => {}
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                #[allow(unreachable_code)]
                Ok::<(), valotracker_core::ValoTrackerError>(())
            })
            .map_err(|e: valotracker_core::ValoTrackerError| PyRuntimeError::new_err(e.to_string()))
    }
}

// ── Module registration ───────────────────────────────────────────────────────

/// Python extension module `valotracker._valotracker`
#[pymodule]
fn valotracker(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPlayer>()?;
    m.add_class::<ValoTrackerClient>()?;
    Ok(())
}
