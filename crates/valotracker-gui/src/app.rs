//! valotracker-gui — egui desktop application.
//!
//! Architecture
//! ────────────
//! • The UI runs on the main thread (egui is single-threaded).
//! • A background OS thread owns the Engine and refreshes periodically.
//! • Shared state is a `Arc<Mutex<BgState>>` protected value.
//! • After each refresh the bg thread calls `ctx.request_repaint()` so
//!   egui wakes up immediately instead of waiting for the next frame.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIconBuilder, TrayIconEvent};
use valotracker_core::{
    discord::{DiscordRpc, PresenceUpdate},
    history::{MatchHistory, PlayerEncounter, SavedMatch},
    updater::{self, UpdateResult},
    Config, MatchSnapshot, ResolvedPlayer, ValoTrackerError,
};

use crate::{
    colors,
    views::{
        encounter::draw_encounter_panel,
        history::draw_history_view,
        match_view::draw_match_view,
        settings::draw_settings_modal,
        topbar::{draw_statusbar, draw_topbar},
    },
};

// ── Shared state (bg thread → render thread) ─────────────────────────────────

#[derive(Clone)]
pub(crate) struct BgState {
    pub(crate) snapshot: Option<MatchSnapshot>,
    pub(crate) error: Option<String>,
    pub(crate) loading: bool,
    pub(crate) load_duration: Option<Duration>,
    pub(crate) last_refresh: Option<Instant>,
    /// True once VALORANT has been detected at least once (lockfile found).
    pub(crate) valorant_detected: bool,
    /// Pending update notification from the auto-updater.
    pub(crate) update_notice: Option<String>,
}

impl Default for BgState {
    fn default() -> Self {
        Self {
            snapshot: None,
            error: None,
            loading: true,
            load_duration: None,
            last_refresh: None,
            valorant_detected: false,
            update_notice: None,
        }
    }
}

// ── Tab enum ──────────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy, Default)]
pub(crate) enum Tab {
    #[default]
    Match,
    History,
}

/// Sorted player list: ally team (highest rank first), then enemy team.
/// Free function used by match_view so it doesn't need a GuiApp reference.
pub(crate) fn sorted_players(snap: &MatchSnapshot) -> Vec<&ResolvedPlayer> {
    let mut allies: Vec<_> = snap.players.iter().filter(|p| p.is_ally).collect();
    let mut enemies: Vec<_> = snap.players.iter().filter(|p| !p.is_ally).collect();
    allies.sort_by_key(|p| std::cmp::Reverse(p.rank.tier));
    enemies.sort_by_key(|p| std::cmp::Reverse(p.rank.tier));
    allies.extend(enemies);
    allies
}

// ── Tray icon state ───────────────────────────────────────────────────────────

struct TrayState {
    /// Keep the TrayIcon alive — dropping it removes the tray icon.
    _icon: tray_icon::TrayIcon,
    show_id: tray_icon::menu::MenuId,
    quit_id: tray_icon::menu::MenuId,
}

// ── Main application struct ───────────────────────────────────────────────────

pub struct GuiApp {
    // Shared with bg thread
    bg: Arc<Mutex<BgState>>,
    refresh_flag: Arc<AtomicBool>,

    // UI state
    tab: Tab,
    config: Config,
    show_encounter: bool,
    encounter_name: String,
    encounter_data: Vec<PlayerEncounter>,
    history: Vec<SavedMatch>,
    history_sel: Option<usize>,
    status_msg: Option<(String, Instant)>,

    // Tray + window management
    tray: Option<TrayState>,
    quit_requested: bool,
    show_settings: bool,

    // Auto-updater
    update_rx: Option<mpsc::Receiver<UpdateResult>>,
    // Toast: (message, expiry_instant)
    toast: Option<(String, Instant)>,

    /// Single shared database connection — opened once at startup and reused
    /// across all history operations to avoid paying the open/schema-check cost
    /// on every user action.
    history_db: Option<Arc<Mutex<MatchHistory>>>,
}

impl GuiApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        // Dark theme with custom panel colours
        let mut vis = egui::Visuals::dark();
        vis.panel_fill = colors::BG_PANEL;
        vis.window_fill = egui::Color32::from_rgb(20, 20, 28);
        vis.faint_bg_color = egui::Color32::from_rgb(25, 25, 35);
        vis.extreme_bg_color = colors::BG_STATUSBAR;
        vis.override_text_color = Some(egui::Color32::from_rgb(215, 215, 215));
        cc.egui_ctx.set_visuals(vis);

        let bg = Arc::new(Mutex::new(BgState::default()));
        let refresh_flag = Arc::new(AtomicBool::new(false));

        // Load config first so we can pass discord settings to bg_thread
        let mut config = Config::load().unwrap_or_default();

        // Spawn background engine/refresh thread (passes Discord config)
        {
            let bg2 = bg.clone();
            let rf2 = refresh_flag.clone();
            let ctx = cc.egui_ctx.clone();
            let discord_enabled = config.features.discord_rpc;
            let discord_app_id = config.features.discord_app_id.clone();
            std::thread::spawn(move || bg_thread(bg2, rf2, ctx, discord_enabled, discord_app_id));
        }

        // Spawn background update check (non-blocking)
        let update_rx = if config.features.check_updates && config.update_check_due() {
            config.mark_update_checked();
            let (tx, rx) = mpsc::channel();
            updater::spawn_update_check(Some(tx));
            Some(rx)
        } else {
            None
        };

        // Open the history database once at startup.
        let history_db = match MatchHistory::open() {
            Ok(db) => Some(Arc::new(Mutex::new(db))),
            Err(e) => {
                tracing::error!("Failed to open history database at startup: {e}");
                None
            }
        };

        Self {
            bg,
            refresh_flag,
            tab: Tab::default(),
            config,
            show_encounter: false,
            encounter_name: String::new(),
            encounter_data: Vec::new(),
            history: Vec::new(),
            history_sel: None,
            status_msg: None,
            tray: build_tray(),
            quit_requested: false,
            show_settings: false,
            update_rx,
            toast: None,
            history_db,
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn trigger_refresh(&self) {
        self.refresh_flag.store(true, Ordering::Relaxed);
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = Some((msg.into(), Instant::now()));
    }

    fn save_match_action(&mut self, bg: &BgState) {
        let Some(snap) = &bg.snapshot else {
            self.set_status("No match to save");
            return;
        };
        let Some(db_arc) = &self.history_db else {
            self.set_status("DB unavailable");
            return;
        };
        match db_arc.lock().unwrap().save_match(
            &snap.match_id,
            &snap.map_name,
            &snap.queue_id,
            &snap.server,
            &snap.players,
            &snap.my_puuid,
            None,
        ) {
            Ok(_) => self.set_status("Match saved ✓"),
            Err(e) => self.set_status(format!("Save failed: {e}")),
        }
    }

    fn open_history(&mut self) {
        if let Some(db_arc) = &self.history_db {
            self.history = db_arc.lock().unwrap().list_matches(100).unwrap_or_default();
        }
        self.tab = Tab::History;
        self.history_sel = if self.history.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    fn open_encounter(&mut self, puuid: &str, name: &str) {
        let Some(db_arc) = &self.history_db else { return };
        if let Ok(enc) = db_arc.lock().unwrap().get_player_encounters(puuid) {
            self.encounter_data = enc;
            self.encounter_name = name.to_owned();
            self.show_encounter = true;
        }
    }

    /// Drain tray icon / tray menu events and act on them.
    fn process_tray_events(&mut self, ctx: &egui::Context) {
        // Double-click on the tray icon → restore window
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if matches!(event, TrayIconEvent::DoubleClick { .. }) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }

        // Tray menu clicks
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Some(ts) = &self.tray {
                if event.id == ts.show_id {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                } else if event.id == ts.quit_id {
                    self.quit_requested = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }
}

// ── Background engine thread ──────────────────────────────────────────────────

fn bg_thread(
    bg: Arc<Mutex<BgState>>,
    rf: Arc<AtomicBool>,
    ctx: egui::Context,
    discord_enabled: bool,
    discord_app_id: String,
) {
    // Initialise Discord RPC if enabled and app_id is set
    let rpc: Option<DiscordRpc> = if discord_enabled && !discord_app_id.is_empty() {
        Some(DiscordRpc::start(&discord_app_id))
    } else {
        None
    };

    // Set idle presence while waiting for VALORANT
    if let Some(r) = &rpc {
        r.send(PresenceUpdate::Idle);
    }

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async move {
        // ── Initialise engine (retry every 2s until VALORANT is running) ──────
        let mut engine = loop {
            match valotracker_core::engine::Engine::init().await {
                Ok(e) => break e,
                Err(_) => {
                    {
                        let mut s = bg.lock().unwrap();
                        s.valorant_detected = false;
                        s.loading = false;
                    }
                    ctx.request_repaint();
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        };
        {
            let mut s = bg.lock().unwrap();
            s.valorant_detected = true;
            s.error = None;
        }

        // ── Main refresh loop ─────────────────────────────────────────────────
        loop {
            {
                let mut s = bg.lock().unwrap();
                s.loading = true;
                s.error = None;
            }
            ctx.request_repaint();

            let t0 = Instant::now();
            match engine
                .build_snapshot("Unknown Map".to_owned(), "competitive".to_owned())
                .await
            {
                Ok(snap) => {
                    let player_count = snap.players.len();
                    let map_name = snap.map_name.clone();
                    let queue_id = snap.queue_id.clone();
                    // Find the local player's party size
                    let my_party_size = {
                        let me = snap.players.iter().find(|p| p.is_ally && p.party_size > 0);
                        me.map(|p| p.party_size).unwrap_or(1)
                    };

                    let mut s = bg.lock().unwrap();
                    let was_none = s.snapshot.is_none();
                    s.snapshot = Some(snap);
                    s.error = None;
                    s.loading = false;
                    s.load_duration = Some(t0.elapsed());
                    s.last_refresh = Some(Instant::now());
                    drop(s);

                    // Discord presence: In Match
                    if let Some(r) = &rpc {
                        r.send(PresenceUpdate::InMatch {
                            map: map_name.clone(),
                            mode: queue_id.clone(),
                            party_size: my_party_size,
                            party_max: 5,
                            start_epoch: 0,
                        });
                    }

                    // Fire toast only when a match is newly detected
                    if was_none {
                        valotracker_core::notifications::notify(
                            "ValoTracker — Match detected!",
                            &format!("Tracking {player_count} players."),
                            true,
                        );
                    }
                }
                Err(ValoTrackerError::NotInMatch) => {
                    let mut s = bg.lock().unwrap();
                    let had_snapshot = s.snapshot.is_some();
                    s.snapshot = None;
                    s.error = Some("Not in a match — waiting…".to_owned());
                    s.loading = false;
                    drop(s);

                    // Discord presence: back to Idle when match ends
                    if had_snapshot {
                        if let Some(r) = &rpc {
                            r.send(PresenceUpdate::Idle);
                        }
                    }
                }
                Err(e) => {
                    let mut s = bg.lock().unwrap();
                    s.error = Some(format!("{e}"));
                    s.loading = false;
                    drop(s);

                    // Discord: clear presence on unexpected error
                    if let Some(r) = &rpc {
                        r.send(PresenceUpdate::Clear);
                    }
                }
            }
            ctx.request_repaint();

            // Wait up to 30s; break early if refresh flag is set
            let deadline = Instant::now() + Duration::from_secs(30);
            loop {
                if rf.swap(false, Ordering::Relaxed) {
                    break;
                }
                if Instant::now() >= deadline {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    });
}

// ── eframe::App ───────────────────────────────────────────────────────────────

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Tray events + minimize-to-tray close handling ─────────────────────
        self.process_tray_events(ctx);
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.config.features.minimize_to_tray && !self.quit_requested {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }

        // ── Poll auto-updater result ──────────────────────────────────────────
        if let Some(rx) = &self.update_rx {
            if let Ok(result) = rx.try_recv() {
                if let UpdateResult::Updated(ver) = result {
                    self.toast = Some((
                        format!("ValoTracker updated to v{ver} — restart to apply"),
                        Instant::now(),
                    ));
                }
                self.update_rx = None;
            }
        }

        // ── Expire toast (6 seconds) ──────────────────────────────────────────
        if let Some((_, ts)) = &self.toast {
            if ts.elapsed() > Duration::from_secs(6) {
                self.toast = None;
            }
        }

        // Expire status messages after 3 seconds
        if let Some((_, ts)) = &self.status_msg {
            if ts.elapsed() > Duration::from_secs(3) {
                self.status_msg = None;
            }
        }

        // Escape closes the encounter panel
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_encounter = false;
        }

        // Snapshot bg state (keeps lock duration minimal)
        let bg = self.bg.lock().unwrap().clone();

        // ── Encounter side panel ──────────────────────────────────────────────
        if self.show_encounter {
            let enc = self.encounter_data.clone();
            let name = self.encounter_name.clone();
            let mut close = false;
            egui::SidePanel::right("encounter_panel")
                .exact_width(440.0)
                .frame(
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(20, 20, 28))
                        .inner_margin(egui::Margin::same(12.0)),
                )
                .show(ctx, |ui| {
                    draw_encounter_panel(ui, &name, &enc, &mut close);
                });
            if close {
                self.show_encounter = false;
            }
        }

        // ── Top bar ───────────────────────────────────────────────────────────
        let mut do_refresh = false;
        let mut do_save = false;
        let mut do_history = false;
        let mut do_settings = false;
        egui::TopBottomPanel::top("topbar")
            .frame(
                egui::Frame::none()
                    .fill(colors::BG_PANEL)
                    .inner_margin(egui::Margin::symmetric(12.0, 6.0)),
            )
            .show(ctx, |ui| {
                draw_topbar(
                    ui,
                    &bg,
                    &mut self.tab,
                    &mut do_refresh,
                    &mut do_save,
                    &mut do_history,
                    &mut do_settings,
                );
            });

        // ── Status bar ────────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("statusbar")
            .frame(
                egui::Frame::none()
                    .fill(colors::BG_STATUSBAR)
                    .inner_margin(egui::Margin::symmetric(12.0, 4.0)),
            )
            .show(ctx, |ui| {
                draw_statusbar(ui, &bg, &self.status_msg);
            });

        // ── Central panel ─────────────────────────────────────────────────────
        let mut open_enc: Option<(String, String)> = None;
        let mut delete_id: Option<String> = None;

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(colors::BG_CENTRAL)
                    .inner_margin(egui::Margin::same(10.0)),
            )
            .show(ctx, |ui| match self.tab {
                Tab::Match => draw_match_view(ui, &bg, &self.config, &mut open_enc),
                Tab::History => {
                    draw_history_view(ui, &mut self.history, &mut self.history_sel, &mut delete_id)
                }
            });

        // ── Process deferred actions (avoid borrow-checker issues) ────────────
        if do_refresh {
            self.trigger_refresh();
        }
        if do_save {
            self.save_match_action(&bg);
        }
        if do_history {
            self.open_history();
        }
        if do_settings {
            self.show_settings = !self.show_settings;
        }

        if let Some((puuid, name)) = open_enc {
            self.open_encounter(&puuid, &name);
        }

        if let Some(id) = delete_id {
            if let Some(db_arc) = &self.history_db {
                let _ = db_arc.lock().unwrap().delete_match(&id);
            }
            self.history.retain(|m| m.id != id);
            if self
                .history_sel
                .map(|i| i >= self.history.len())
                .unwrap_or(false)
            {
                self.history_sel = self.history.len().checked_sub(1);
            }
        }

        // ── Settings modal ────────────────────────────────────────────────────
        if self.show_settings {
            if let Some(msg) = draw_settings_modal(ctx, &mut self.config, &mut self.show_settings) {
                self.set_status(msg);
            }
        }

        // ── Update toast (bottom-right corner) ────────────────────────────────
        if let Some((msg, _)) = &self.toast {
            let msg = msg.clone();
            let screen = ctx.screen_rect();
            egui::Area::new("update_toast".into())
                .fixed_pos(egui::pos2(screen.max.x - 360.0, screen.max.y - 52.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(30, 80, 50))
                        .rounding(6.0)
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!("✓ {msg}"))
                                    .color(egui::Color32::from_rgb(120, 220, 140))
                                    .small(),
                            );
                        });
                });
        }
    }
}


// ── Tray icon builder ─────────────────────────────────────────────────────────

/// Build the system tray icon and menu. Returns `None` if the tray is
/// unavailable (e.g. running in a headless environment).
fn build_tray() -> Option<TrayState> {
    // 32×32 solid VALORANT-red (#FF4655) icon as RGBA bytes
    const SIZE: u32 = 32;
    let rgba: Vec<u8> = std::iter::repeat([0xFF_u8, 0x46, 0x55, 0xFF])
        .take((SIZE * SIZE) as usize)
        .flatten()
        .collect();

    let icon = tray_icon::Icon::from_rgba(rgba, SIZE, SIZE)
        .map_err(|e| eprintln!("tray icon: {e}"))
        .ok()?;

    let show_item = MenuItem::new("Open ValoTracker", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    let show_id = show_item.id().clone();
    let quit_id = quit_item.id().clone();

    let menu = Menu::new();
    menu.append_items(&[&show_item, &PredefinedMenuItem::separator(), &quit_item])
        .map_err(|e| eprintln!("tray menu: {e}"))
        .ok()?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("ValoTracker")
        .with_icon(icon)
        .build()
        .map_err(|e| eprintln!("tray: {e}"))
        .ok()?;

    Some(TrayState {
        _icon: tray,
        show_id,
        quit_id,
    })
}
