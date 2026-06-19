use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use valotracker_core::{
    engine::Engine,
    history::MatchHistory,
    models::match_data::MatchSnapshot,
    updater::{self, DownloadMsg, UpdateState},
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

// download state for the updater ui
#[derive(Clone)]
pub enum DownloadState {
    Idle,
    Downloading(f32),
    Verifying,
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
    update_rx: Option<mpsc::Receiver<UpdateState>>,
    // the newest version we found, if theres one
    pub update_available: Option<String>,
    // gets download progress once an update starts
    download_rx: Option<mpsc::Receiver<DownloadMsg>>,
    // where the download is at rn
    pub download_state: DownloadState,
    // set once the installer is downloaded + good. the main loop sees this,
    // quits, puts the terminal back to normal, then runs it
    pub install_pending: Option<std::path::PathBuf>,

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
            updater::spawn_update_check(tx);
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
            update_available: None,
            download_rx: None,
            download_state: DownloadState::Idle,
            install_pending: None,
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

        match engine.build_snapshot().await {
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
        // see if the update check found anything, show it in the footer
        if let Some(rx) = self.update_rx.take() {
            let mut keep = true;
            loop {
                match rx.try_recv() {
                    Ok(UpdateState::Available(ver)) => {
                        self.update_available = Some(ver.clone());
                        self.set_status(format!("Update v{ver} available — press u to install"));
                    }
                    Ok(_) => {}
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        keep = false;
                        break;
                    }
                }
            }
            if keep {
                self.update_rx = Some(rx);
            }
        }

        // check on the download if ones going
        if let Some(rx) = self.download_rx.take() {
            let mut keep = true;
            loop {
                match rx.try_recv() {
                    Ok(DownloadMsg::Progress(p)) => {
                        self.download_state = DownloadState::Downloading(p);
                    }
                    Ok(DownloadMsg::Verifying) => {
                        self.download_state = DownloadState::Verifying;
                    }
                    Ok(DownloadMsg::OpenedBrowser) => {
                        self.download_state = DownloadState::Idle;
                        self.set_status("Opened the releases page in your browser".to_owned());
                        keep = false;
                    }
                    Ok(DownloadMsg::Failed(e)) => {
                        self.download_state = DownloadState::Idle;
                        self.set_status(format!("Update failed: {e}"));
                        keep = false;
                    }
                    Ok(DownloadMsg::Done(path)) => {
                        self.install_pending = Some(path);
                        keep = false;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        keep = false;
                        break;
                    }
                }
            }
            if keep {
                self.download_rx = Some(rx);
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

    // start grabbing the update (runs when you hit u)
    pub fn start_update(&mut self) {
        if self.download_rx.is_some() {
            return;
        }
        if let Some(ver) = self.update_available.clone() {
            let (tx, rx) = mpsc::channel();
            self.download_rx = Some(rx);
            self.download_state = DownloadState::Downloading(0.0);
            self.set_status(format!("Downloading update v{ver}…"));
            updater::start_download(ver, tx);
        }
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
