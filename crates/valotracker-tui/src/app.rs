use std::time::{Duration, Instant};

use valotracker_core::{
    engine::Engine, history::MatchHistory, models::match_data::MatchSnapshot, Config,
    ValoTrackerError,
};

const REFRESH_INTERVAL: Duration = Duration::from_secs(30);

/// UI view state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    /// Live match table.
    Match,
    /// Match history list.
    History,
    /// Player encounter drill-down.
    Encounter { puuid: String },
    /// Inline config editor overlay.
    Config,
}

/// Top-level application state machine.
pub struct App {
    pub config: Config,
    pub view: View,

    // ── Match state ───────────────────────────────────────────────────────────
    pub engine: Option<Engine>,
    pub snapshot: Option<MatchSnapshot>,
    pub load_error: Option<String>,
    pub is_loading: bool,
    pub last_refresh: Option<Instant>,
    pub load_duration: Option<Duration>,

    // ── Table navigation ──────────────────────────────────────────────────────
    /// Currently selected row index in the player table.
    pub selected_row: Option<usize>,

    // ── History view ──────────────────────────────────────────────────────────
    pub history: Option<Vec<valotracker_core::history::SavedMatch>>,
    pub history_selected: usize,

    // ── Encounter drill-down ──────────────────────────────────────────────────
    pub encounter_data: Option<Vec<valotracker_core::history::PlayerEncounter>>,
    pub encounter_name: String,

    // ── Status bar ───────────────────────────────────────────────────────────
    pub status_msg: Option<(String, Instant)>,
}

impl App {
    pub async fn new() -> Self {
        let config = Config::load().unwrap_or_default();
        let mut app = App {
            config,
            view: View::Match,
            engine: None,
            snapshot: None,
            load_error: None,
            is_loading: false,
            last_refresh: None,
            load_duration: None,
            selected_row: None,
            history: None,
            history_selected: 0,
            encounter_data: None,
            encounter_name: String::new(),
            status_msg: None,
        };
        app.init_engine().await;
        app
    }

    /// Try to initialise the engine (requires VALORANT to be running).
    async fn init_engine(&mut self) {
        match Engine::init().await {
            Ok(engine) => {
                self.engine = Some(engine);
                self.refresh().await;
            }
            Err(e) => {
                self.load_error = Some(format!("{e}"));
            }
        }
    }

    /// Refresh the match snapshot.
    pub async fn refresh(&mut self) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        self.is_loading = true;
        let start = Instant::now();

        match engine
            .build_snapshot("Unknown Map".to_owned(), "competitive".to_owned())
            .await
        {
            Ok(snap) => {
                self.snapshot = Some(snap);
                self.load_error = None;
                self.load_duration = Some(start.elapsed());
            }
            Err(ValoTrackerError::NotInMatch) => {
                self.snapshot = None;
                self.load_error = Some("Not in a match — waiting…".to_owned());
            }
            Err(e) => {
                self.load_error = Some(format!("{e}"));
            }
        }

        self.is_loading = false;
        self.last_refresh = Some(Instant::now());
    }

    /// Called every frame tick — auto-refresh if interval has elapsed.
    pub async fn tick(&mut self) {
        // Clear expired status messages (3s TTL)
        if let Some((_, ts)) = &self.status_msg {
            if ts.elapsed() > Duration::from_secs(3) {
                self.status_msg = None;
            }
        }

        // Auto-refresh
        if let Some(last) = self.last_refresh {
            if last.elapsed() >= REFRESH_INTERVAL {
                self.refresh().await;
            }
        }
    }

    /// Open the history view.
    pub fn open_history(&mut self) {
        if let Ok(db) = MatchHistory::open() {
            self.history = db.list_matches(100).ok();
        }
        self.view = View::History;
    }

    /// Save the current match to history.
    pub async fn save_current_match(&mut self) {
        let Some(snap) = &self.snapshot else {
            self.set_status("No match to save".to_owned());
            return;
        };
        match MatchHistory::open() {
            Ok(db) => {
                let result = db.save_match(
                    &snap.match_id,
                    &snap.map_name,
                    &snap.queue_id,
                    &snap.server,
                    &snap.players,
                    &snap.my_puuid,
                    None,
                );
                match result {
                    Ok(_) => self.set_status("Match saved ✓".to_owned()),
                    Err(e) => self.set_status(format!("Save failed: {e}")),
                }
            }
            Err(e) => self.set_status(format!("DB error: {e}")),
        }
    }

    /// Open an encounter drill-down for the selected player.
    pub fn open_encounter(&mut self, puuid: &str, display_name: &str) {
        if let Ok(db) = MatchHistory::open() {
            if let Ok(encounters) = db.get_player_encounters(puuid) {
                self.encounter_data = Some(encounters);
                self.encounter_name = display_name.to_owned();
                self.view = View::Encounter {
                    puuid: puuid.to_owned(),
                };
            }
        }
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_msg = Some((msg, Instant::now()));
    }

    pub fn go_back(&mut self) {
        self.view = View::Match;
    }

    /// Returns all players sorted for display: ally team first, then enemy.
    pub fn display_players(&self) -> Vec<&valotracker_core::ResolvedPlayer> {
        let Some(snap) = &self.snapshot else {
            return Vec::new();
        };
        let mut allies: Vec<_> = snap.players.iter().filter(|p| p.is_ally).collect();
        let mut enemies: Vec<_> = snap.players.iter().filter(|p| !p.is_ally).collect();
        allies.sort_by_key(|p| p.rank.tier);
        enemies.sort_by_key(|p| p.rank.tier);
        allies.extend(enemies);
        allies
    }
}
