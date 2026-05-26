use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use valotracker_core::{
    engine::Engine,
    history::MatchHistory,
    models::match_data::MatchSnapshot,
    updater::{self, UpdateResult},
    Config, ValoTrackerError,
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

    // ── Auto-updater ─────────────────────────────────────────────────────────
    /// Receives the result of the background update check (if one was started).
    update_rx: Option<mpsc::Receiver<UpdateResult>>,

    /// Shared history DB — opened once at startup to avoid repeated open cost.
    pub history_db: Option<Arc<Mutex<MatchHistory>>>,
}

impl App {
    pub async fn new() -> Self {
        let mut config = Config::load().unwrap_or_default();

        // Spawn background update check (non-blocking, 3s timeout)
        let update_rx = if config.features.check_updates && config.update_check_due() {
            config.mark_update_checked();
            let (tx, rx) = mpsc::channel();
            updater::spawn_update_check(Some(tx));
            Some(rx)
        } else {
            None
        };

        let history_db = match MatchHistory::open() {
            Ok(db) => Some(Arc::new(Mutex::new(db))),
            Err(e) => {
                tracing::error!("Failed to open history database at startup: {e}");
                None
            }
        };

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
            update_rx,
            history_db,
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
            .build_snapshot()
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
        // Poll the update receiver; if a newer version was installed, show a
        // one-line notification in the status bar. Never block — try_recv only.
        if let Some(rx) = &self.update_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    UpdateResult::Updated(ver) => {
                        self.set_status(format!(
                            "ValoTracker updated to v{ver} — restart to apply"
                        ));
                    }
                    UpdateResult::UpToDate | UpdateResult::Skipped => {}
                }
                // Drain the receiver after we've read the result
                self.update_rx = None;
            }
        }

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
        if let Some(db_arc) = &self.history_db {
            self.history = db_arc.lock().unwrap().list_matches(100).ok();
        }
        self.view = View::History;
    }

    /// Save the current match to history.
    pub async fn save_current_match(&mut self) {
        let Some(snap) = &self.snapshot else {
            self.set_status("No match to save".to_owned());
            return;
        };
        let Some(db_arc) = &self.history_db else {
            self.set_status("DB unavailable".to_owned());
            return;
        };
        let result = db_arc.lock().unwrap().save_match(
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

    /// Open an encounter drill-down for the selected player.
    pub fn open_encounter(&mut self, puuid: &str, display_name: &str) {
        let Some(db_arc) = &self.history_db else {
            return;
        };
        if let Ok(encounters) = db_arc.lock().unwrap().get_player_encounters(puuid) {
            self.encounter_data = Some(encounters);
            self.encounter_name = display_name.to_owned();
            self.view = View::Encounter {
                puuid: puuid.to_owned(),
            };
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
