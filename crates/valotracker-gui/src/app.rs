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
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use valotracker_core::{
    history::{MatchHistory, PlayerEncounter, SavedMatch},
    tier_to_name, tier_to_short, Config, MatchSnapshot, ResolvedPlayer, ValoTrackerError,
};

use crate::colors;

// ── Shared state (bg thread → render thread) ─────────────────────────────────

#[derive(Clone)]
struct BgState {
    snapshot: Option<MatchSnapshot>,
    error: Option<String>,
    loading: bool,
    load_duration: Option<Duration>,
    last_refresh: Option<Instant>,
}

impl Default for BgState {
    fn default() -> Self {
        Self {
            snapshot: None,
            error: None,
            loading: true,
            load_duration: None,
            last_refresh: None,
        }
    }
}

// ── Tab enum ──────────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy, Default)]
enum Tab {
    #[default]
    Match,
    History,
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

        // Spawn background engine/refresh thread
        {
            let bg2 = bg.clone();
            let rf2 = refresh_flag.clone();
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || bg_thread(bg2, rf2, ctx));
        }

        Self {
            bg,
            refresh_flag,
            tab: Tab::default(),
            config: Config::load().unwrap_or_default(),
            show_encounter: false,
            encounter_name: String::new(),
            encounter_data: Vec::new(),
            history: Vec::new(),
            history_sel: None,
            status_msg: None,
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn trigger_refresh(&self) {
        self.refresh_flag.store(true, Ordering::Relaxed);
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = Some((msg.into(), Instant::now()));
    }

    /// Sorted player list: ally team (highest rank first), then enemy team.
    fn display_players<'a>(snap: &'a MatchSnapshot) -> Vec<&'a ResolvedPlayer> {
        let mut allies: Vec<_> = snap.players.iter().filter(|p| p.is_ally).collect();
        let mut enemies: Vec<_> = snap.players.iter().filter(|p| !p.is_ally).collect();
        allies.sort_by_key(|p| std::cmp::Reverse(p.rank.tier));
        enemies.sort_by_key(|p| std::cmp::Reverse(p.rank.tier));
        allies.extend(enemies);
        allies
    }

    fn save_match_action(&mut self, bg: &BgState) {
        let Some(snap) = &bg.snapshot else {
            self.set_status("No match to save");
            return;
        };
        match MatchHistory::open() {
            Ok(db) => match db.save_match(
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
            },
            Err(e) => self.set_status(format!("DB error: {e}")),
        }
    }

    fn open_history(&mut self) {
        if let Ok(db) = MatchHistory::open() {
            self.history = db.list_matches(100).unwrap_or_default();
        }
        self.tab = Tab::History;
        self.history_sel = if self.history.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    fn open_encounter(&mut self, puuid: &str, name: &str) {
        if let Ok(db) = MatchHistory::open() {
            if let Ok(enc) = db.get_player_encounters(puuid) {
                self.encounter_data = enc;
                self.encounter_name = name.to_owned();
                self.show_encounter = true;
            }
        }
    }
}

// ── Background engine thread ──────────────────────────────────────────────────

fn bg_thread(bg: Arc<Mutex<BgState>>, rf: Arc<AtomicBool>, ctx: egui::Context) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async move {
        // ── Initialise engine (retry every 5s until VALORANT is running) ──────
        let mut engine = loop {
            match valotracker_core::engine::Engine::init().await {
                Ok(e) => break e,
                Err(e) => {
                    bg.lock().unwrap().error = Some(format!("{e}"));
                    ctx.request_repaint();
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        };
        bg.lock().unwrap().error = None;

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
                    let mut s = bg.lock().unwrap();
                    s.snapshot = Some(snap);
                    s.error = None;
                    s.loading = false;
                    s.load_duration = Some(t0.elapsed());
                    s.last_refresh = Some(Instant::now());
                }
                Err(ValoTrackerError::NotInMatch) => {
                    let mut s = bg.lock().unwrap();
                    s.snapshot = None;
                    s.error = Some("Not in a match — waiting…".to_owned());
                    s.loading = false;
                }
                Err(e) => {
                    let mut s = bg.lock().unwrap();
                    s.error = Some(format!("{e}"));
                    s.loading = false;
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

        if let Some((puuid, name)) = open_enc {
            self.open_encounter(&puuid, &name);
        }

        if let Some(id) = delete_id {
            if let Ok(db) = MatchHistory::open() {
                let _ = db.delete_match(&id);
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
    }
}

// ── Top bar ───────────────────────────────────────────────────────────────────

fn draw_topbar(
    ui: &mut egui::Ui,
    bg: &BgState,
    tab: &mut Tab,
    do_refresh: &mut bool,
    do_save: &mut bool,
    do_history: &mut bool,
) {
    ui.horizontal(|ui| {
        // Logo
        ui.label(
            egui::RichText::new("ValoTracker")
                .strong()
                .size(17.0)
                .color(colors::ACCENT),
        );
        ui.separator();

        // Match info
        if let Some(snap) = &bg.snapshot {
            ui.label(egui::RichText::new(&snap.map_name).strong());
            ui.separator();
            ui.label(
                egui::RichText::new(&snap.queue_id).color(egui::Color32::from_rgb(180, 180, 180)),
            );
            if !snap.server.is_empty() {
                ui.separator();
                ui.label(
                    egui::RichText::new(&snap.server).color(egui::Color32::from_rgb(130, 130, 150)),
                );
            }
        } else if bg.loading {
            ui.spinner();
            ui.label(egui::RichText::new("Connecting…").color(egui::Color32::GRAY));
        } else if let Some(err) = &bg.error {
            ui.label(egui::RichText::new(err).color(egui::Color32::from_rgb(200, 100, 100)));
        }

        // Right-aligned controls
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(4.0);
            // Tabs
            ui.selectable_value(tab, Tab::History, "📋 History");
            if ui.selectable_value(tab, Tab::Match, "🎮 Live").clicked() {}
            ui.separator();

            if ui
                .add_enabled(!bg.loading, egui::Button::new("⟳ Refresh"))
                .on_hover_text("Force a data refresh (r)")
                .clicked()
            {
                *do_refresh = true;
            }

            if ui
                .add_enabled(bg.snapshot.is_some(), egui::Button::new("💾 Save"))
                .on_hover_text("Save current match to history (s)")
                .clicked()
            {
                *do_save = true;
            }

            if ui
                .button("📋 History")
                .on_hover_text("Open match history (h)")
                .clicked()
            {
                *do_history = true;
                *tab = Tab::History;
            }
        });
    });
}

// ── Status bar ────────────────────────────────────────────────────────────────

fn draw_statusbar(ui: &mut egui::Ui, bg: &BgState, status_msg: &Option<(String, Instant)>) {
    ui.horizontal(|ui| {
        if let Some((msg, _)) = status_msg {
            ui.label(
                egui::RichText::new(msg)
                    .color(egui::Color32::from_rgb(100, 220, 100))
                    .small(),
            );
        } else {
            ui.label(
                egui::RichText::new("[⟳] Refresh   [💾] Save   [📋] History   [Esc] Close panel")
                    .color(colors::DIM)
                    .small(),
            );
        }

        if let Some(dur) = bg.load_duration {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("Loaded in {:.1}s", dur.as_secs_f32()))
                        .color(colors::DIM)
                        .small(),
                );
            });
        }
    });
}

// ── Match view ────────────────────────────────────────────────────────────────

fn draw_match_view(
    ui: &mut egui::Ui,
    bg: &BgState,
    config: &Config,
    open_enc: &mut Option<(String, String)>,
) {
    if bg.loading && bg.snapshot.is_none() {
        ui.centered_and_justified(|ui| {
            ui.add(egui::Spinner::new().size(36.0));
        });
        return;
    }

    if bg.snapshot.is_none() {
        ui.centered_and_justified(|ui| {
            let msg = bg.error.as_deref().unwrap_or("Waiting for match…");
            ui.label(
                egui::RichText::new(msg)
                    .color(egui::Color32::DARK_GRAY)
                    .size(18.0),
            );
        });
        return;
    }

    let snap = bg.snapshot.as_ref().unwrap().clone();
    let players = GuiApp::display_players(&snap);
    let short = config.display.short_ranks;

    // Column pixel widths
    const W: [f32; 12] = [
        44.0, 100.0, 185.0, 110.0, 38.0, 90.0, 45.0, 45.0, 46.0, 38.0, 45.0, 35.0,
    ];

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("match_grid")
                .num_columns(12)
                .striped(false)
                .spacing([5.0, 3.0])
                .min_col_width(0.0)
                .show(ui, |ui| {
                    // ── Column headers ────────────────────────────────────────
                    for (i, h) in [
                        "PTY", "AGENT", "NAME", "RANK", "RR", "PEAK", "HS%", "WR%", "K/D", "LVL",
                        "ΔRR", "MET",
                    ]
                    .iter()
                    .enumerate()
                    {
                        ui.add_sized(
                            [W[i], 16.0],
                            egui::Label::new(
                                egui::RichText::new(*h)
                                    .strong()
                                    .color(colors::HEADER)
                                    .small(),
                            ),
                        );
                    }
                    ui.end_row();

                    // Thin separator under headers
                    for w in &W {
                        ui.add_sized([*w, 2.0], egui::Separator::default().horizontal());
                    }
                    ui.end_row();

                    // ── Player rows ───────────────────────────────────────────
                    let mut last_ally: Option<bool> = None;

                    for player in players.iter() {
                        // Team divider
                        if let Some(la) = last_ally {
                            if la != player.is_ally {
                                for w in &W {
                                    ui.add_sized(
                                        [*w, 6.0],
                                        egui::Label::new(
                                            egui::RichText::new("──────")
                                                .color(egui::Color32::from_rgb(50, 50, 60)),
                                        ),
                                    );
                                }
                                ui.end_row();
                            }
                        }
                        last_ally = Some(player.is_ally);

                        // ── Party ─────────────────────────────────────────────
                        let p_col =
                            if player.is_enemy_party && config.display.highlight_enemy_parties {
                                colors::PARTY_ENEMY
                            } else {
                                egui::Color32::from_rgb(180, 180, 220)
                            };
                        let p_str = if player.party_size > 1 && config.display.show_party_size {
                            format!("{} ({})", player.party_icon, player.party_size)
                        } else {
                            player.party_icon.to_string()
                        };
                        ui.add_sized(
                            [W[0], 20.0],
                            egui::Label::new(egui::RichText::new(p_str).color(p_col).monospace()),
                        );

                        // ── Agent ─────────────────────────────────────────────
                        ui.add_sized(
                            [W[1], 20.0],
                            egui::Label::new(
                                egui::RichText::new(&player.agent_name)
                                    .color(egui::Color32::from_rgb(200, 200, 200)),
                            ),
                        );

                        // ── Name (clickable if seen before) ───────────────────
                        let name_col = if player.is_ally {
                            colors::ALLY_COLOR
                        } else {
                            colors::ENEMY_COLOR
                        };
                        let mut name_str = player.display_name().to_owned();
                        if player.incognito && config.display.show_streamer_tag {
                            name_str.push_str(" [S]");
                        }

                        if player.times_seen > 0 {
                            let lbl = format!("{} 👁", name_str);
                            let resp = ui
                                .add_sized(
                                    [W[2], 20.0],
                                    egui::Button::new(egui::RichText::new(&lbl).color(name_col))
                                        .frame(false),
                                )
                                .on_hover_text(format!(
                                    "Seen {} time(s) before — click for history",
                                    player.times_seen
                                ));
                            if resp.clicked() {
                                *open_enc =
                                    Some((player.puuid.clone(), player.display_name().to_owned()));
                            }
                        } else {
                            ui.add_sized(
                                [W[2], 20.0],
                                egui::Label::new(egui::RichText::new(&name_str).color(name_col)),
                            );
                        }

                        // ── Rank ──────────────────────────────────────────────
                        let rank_str = if short {
                            tier_to_short(player.rank.tier).to_owned()
                        } else {
                            tier_to_name(player.rank.tier).to_owned()
                        };
                        ui.add_sized(
                            [W[3], 20.0],
                            egui::Label::new(
                                egui::RichText::new(rank_str)
                                    .color(colors::rank_color(player.rank.tier))
                                    .strong(),
                            ),
                        );

                        // ── RR ────────────────────────────────────────────────
                        ui.add_sized(
                            [W[4], 20.0],
                            egui::Label::new(
                                egui::RichText::new(player.rank.rr.to_string())
                                    .monospace()
                                    .color(egui::Color32::from_rgb(200, 200, 200)),
                            ),
                        );

                        // ── Peak ──────────────────────────────────────────────
                        let peak_str = if short {
                            tier_to_short(player.rank.peak_tier).to_owned()
                        } else {
                            let n = tier_to_name(player.rank.peak_tier);
                            if n.len() > 8 {
                                n[..8].to_owned()
                            } else {
                                n.to_owned()
                            }
                        };
                        ui.add_sized(
                            [W[5], 20.0],
                            egui::Label::new(
                                egui::RichText::new(peak_str)
                                    .color(colors::rank_color(player.rank.peak_tier))
                                    .weak(),
                            ),
                        );

                        // ── HS% ───────────────────────────────────────────────
                        let hs = player.stats.headshot_pct;
                        let hs_str = if config.display.show_hs {
                            format!("{:.0}%", hs * 100.0)
                        } else {
                            "—".into()
                        };
                        ui.add_sized(
                            [W[6], 20.0],
                            egui::Label::new(
                                egui::RichText::new(hs_str)
                                    .monospace()
                                    .color(colors::hs_color(hs)),
                            ),
                        );

                        // ── WR% ───────────────────────────────────────────────
                        let wr = player.stats.win_rate;
                        let wr_str = if config.display.show_wr {
                            format!("{:.0}%", wr * 100.0)
                        } else {
                            "—".into()
                        };
                        ui.add_sized(
                            [W[7], 20.0],
                            egui::Label::new(
                                egui::RichText::new(wr_str)
                                    .monospace()
                                    .color(colors::wr_color(wr)),
                            ),
                        );

                        // ── K/D ───────────────────────────────────────────────
                        let kd = player.stats.kd_ratio;
                        let kd_str = if config.display.show_kd {
                            format!("{:.2}", kd)
                        } else {
                            "—".into()
                        };
                        ui.add_sized(
                            [W[8], 20.0],
                            egui::Label::new(
                                egui::RichText::new(kd_str)
                                    .monospace()
                                    .color(colors::kd_color(kd)),
                            ),
                        );

                        // ── Level ─────────────────────────────────────────────
                        let lvl_str = if config.display.show_level && !player.hide_account_level {
                            player.account_level.to_string()
                        } else {
                            "—".into()
                        };
                        ui.add_sized(
                            [W[9], 20.0],
                            egui::Label::new(
                                egui::RichText::new(lvl_str).monospace().color(colors::DIM),
                            ),
                        );

                        // ── ΔRR ──────────────────────────────────────────────
                        let rr_d = player.stats.avg_rr_delta;
                        let rr_str = if config.display.show_rr_delta {
                            if rr_d > 0.0 {
                                format!("+{:.0}", rr_d)
                            } else {
                                format!("{:.0}", rr_d)
                            }
                        } else {
                            "—".into()
                        };
                        ui.add_sized(
                            [W[10], 20.0],
                            egui::Label::new(
                                egui::RichText::new(rr_str)
                                    .monospace()
                                    .color(colors::rr_delta_color(rr_d)),
                            ),
                        );

                        // ── MET ───────────────────────────────────────────────
                        let met_str = if player.times_seen > 0 {
                            player.times_seen.to_string()
                        } else {
                            "—".into()
                        };
                        let met_col = if player.times_seen > 0 {
                            colors::MET_COLOR
                        } else {
                            colors::DIM
                        };
                        ui.add_sized(
                            [W[11], 20.0],
                            egui::Label::new(
                                egui::RichText::new(met_str).monospace().color(met_col),
                            ),
                        );

                        ui.end_row();
                    }
                });
        });
}

// ── History view ──────────────────────────────────────────────────────────────

fn draw_history_view(
    ui: &mut egui::Ui,
    history: &mut Vec<SavedMatch>,
    history_sel: &mut Option<usize>,
    delete_id: &mut Option<String>,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("Match History  ·  {} saved", history.len()))
                .strong()
                .color(egui::Color32::from_rgb(100, 180, 255)),
        );
    });
    ui.add_space(4.0);

    if history.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new(
                    "No saved matches yet.\nSwitch to Live tab and press 💾 Save during a match.",
                )
                .color(colors::DIM),
            );
        });
        return;
    }

    const CW: [f32; 7] = [90.0, 90.0, 115.0, 35.0, 80.0, 50.0, 24.0];

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("history_grid")
                .num_columns(7)
                .striped(true)
                .spacing([8.0, 4.0])
                .min_col_width(0.0)
                .show(ui, |ui| {
                    // Header
                    for (i, h) in ["Date", "Map", "Queue", "W/L", "Rank", "ΔRR", ""]
                        .iter()
                        .enumerate()
                    {
                        ui.add_sized(
                            [CW[i], 16.0],
                            egui::Label::new(
                                egui::RichText::new(*h)
                                    .strong()
                                    .color(colors::HEADER)
                                    .small(),
                            ),
                        );
                    }
                    ui.end_row();

                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;

                    let mut idx_to_delete: Option<usize> = None;

                    for (i, m) in history.iter().enumerate() {
                        let sel = *history_sel == Some(i);

                        let age_days = (now - m.saved_at) / 86400;
                        let date_str = match age_days {
                            0 => "Today".to_owned(),
                            1 => "Yesterday".to_owned(),
                            d => format!("{d}d ago"),
                        };

                        let (wl_str, wl_col) = match m.won {
                            Some(true) => ("W", egui::Color32::from_rgb(80, 220, 100)),
                            Some(false) => ("L", egui::Color32::from_rgb(220, 80, 80)),
                            None => ("?", colors::DIM),
                        };

                        let rr_sign = if m.my_rr_delta >= 0 { "+" } else { "" };
                        let rr_str = format!("{rr_sign}{}", m.my_rr_delta);
                        let rr_col = if m.my_rr_delta > 0 {
                            egui::Color32::from_rgb(80, 220, 100)
                        } else if m.my_rr_delta < 0 {
                            egui::Color32::from_rgb(220, 80, 80)
                        } else {
                            colors::DIM
                        };

                        let row_click = |ui: &mut egui::Ui, text: &str, col: egui::Color32| {
                            ui.add_sized(
                                [0.0, 20.0], // width set by grid
                                egui::SelectableLabel::new(
                                    sel,
                                    egui::RichText::new(text).color(col),
                                ),
                            )
                            .clicked()
                        };

                        if row_click(ui, &date_str, colors::DIM) {
                            *history_sel = Some(i);
                        }
                        if row_click(ui, &m.map, egui::Color32::from_rgb(215, 215, 215)) {
                            *history_sel = Some(i);
                        }
                        if row_click(ui, &m.queue, egui::Color32::from_rgb(180, 180, 180)) {
                            *history_sel = Some(i);
                        }

                        ui.add_sized(
                            [CW[3], 20.0],
                            egui::Label::new(egui::RichText::new(wl_str).color(wl_col).strong()),
                        );
                        ui.add_sized(
                            [CW[4], 20.0],
                            egui::Label::new(
                                egui::RichText::new(tier_to_short(m.my_rank_tier))
                                    .color(colors::rank_color(m.my_rank_tier)),
                            ),
                        );
                        ui.add_sized(
                            [CW[5], 20.0],
                            egui::Label::new(
                                egui::RichText::new(&rr_str).monospace().color(rr_col),
                            ),
                        );

                        if ui.small_button("🗑").on_hover_text("Delete match").clicked() {
                            idx_to_delete = Some(i);
                        }
                        ui.end_row();
                    }

                    if let Some(idx) = idx_to_delete {
                        *delete_id = Some(history[idx].id.clone());
                    }
                });
        });
}

// ── Encounter side panel ──────────────────────────────────────────────────────

fn draw_encounter_panel(
    ui: &mut egui::Ui,
    name: &str,
    encounters: &[PlayerEncounter],
    close: &mut bool,
) {
    // Title row
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(name)
                .strong()
                .size(15.0)
                .color(egui::Color32::from_rgb(215, 215, 215)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✕").clicked() {
                *close = true;
            }
        });
    });

    ui.label(
        egui::RichText::new(format!("{} encounter(s)", encounters.len()))
            .color(colors::DIM)
            .small(),
    );
    ui.separator();

    if encounters.is_empty() {
        ui.label(egui::RichText::new("No encounter data yet.").color(colors::DIM));
        return;
    }

    // Summary
    let summary = valotracker_core::history::summarize_encounters(encounters);
    let taunt = summary
        .worst_game
        .as_ref()
        .map(|g| g.deaths >= 15 && g.kills <= 8)
        .unwrap_or(false);
    let icon = if taunt { "💀" } else { "👀" };
    let sum_col = if taunt {
        egui::Color32::from_rgb(220, 80, 80)
    } else {
        egui::Color32::from_rgb(180, 180, 180)
    };

    ui.label(
        egui::RichText::new(format!(
            "{icon}  {}-{} W/L  ·  Avg {:.1}K/{:.1}D  ·  HS {:.0}%  ·  Usually {}",
            summary.wins_against,
            summary.losses_against,
            summary.avg_kills,
            summary.avg_deaths,
            summary.avg_hs_pct * 100.0,
            summary.most_played_agent,
        ))
        .color(sum_col),
    );

    ui.separator();

    // Encounter table
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("enc_grid")
                .num_columns(8)
                .striped(true)
                .spacing([8.0, 3.0])
                .show(ui, |ui| {
                    for h in ["Date", "Map", "Agent", "K", "D", "A", "HS%", "W/L"] {
                        ui.label(
                            egui::RichText::new(h)
                                .strong()
                                .color(colors::HEADER)
                                .small(),
                        );
                    }
                    ui.end_row();

                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;

                    for enc in encounters {
                        let age = (now - enc.saved_at) / 86400;
                        let date_str = match age {
                            0 => "Today".to_owned(),
                            1 => "Yesterday".to_owned(),
                            d => format!("{d}d ago"),
                        };

                        let (wl_str, wl_col) = match enc.won {
                            Some(true) => ("W", egui::Color32::from_rgb(80, 220, 100)),
                            Some(false) => ("L", egui::Color32::from_rgb(220, 80, 80)),
                            None => ("?", colors::DIM),
                        };

                        let side = if enc.was_enemy { "⚔" } else { "✦" };

                        ui.label(egui::RichText::new(&date_str).color(colors::DIM).small());
                        ui.label(egui::RichText::new(format!("{} {}", &enc.map, side)).small());
                        ui.label(egui::RichText::new(&enc.agent).small());
                        ui.label(
                            egui::RichText::new(enc.kills.to_string())
                                .color(egui::Color32::from_rgb(80, 220, 100))
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(enc.deaths.to_string())
                                .color(egui::Color32::from_rgb(220, 80, 80))
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(enc.assists.to_string())
                                .color(colors::DIM)
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.0}%", enc.hs_pct * 100.0))
                                .color(colors::hs_color(enc.hs_pct))
                                .monospace()
                                .small(),
                        );
                        ui.label(egui::RichText::new(wl_str).color(wl_col).strong().small());
                        ui.end_row();
                    }
                });
        });
}
