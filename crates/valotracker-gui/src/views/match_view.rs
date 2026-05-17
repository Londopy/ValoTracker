//! Live match view — the main player table shown while in a match.

use eframe::egui;
use valotracker_core::{tier_to_name, tier_to_short, Config};

use crate::{
    app::{sorted_players, BgState},
    colors,
};

/// Draw the live match view, including idle screens when not in a game.
///
/// `open_enc` is set to `(puuid, display_name)` when the user clicks a player
/// name to open the encounter side panel.
pub fn draw_match_view(
    ui: &mut egui::Ui,
    bg: &BgState,
    config: &Config,
    open_enc: &mut Option<(String, String)>,
) {
    // ── Idle screen: VALORANT not running ────────────────────────────────────
    if !bg.valorant_detected {
        super::idle::draw_idle_screen(ui, false);
        return;
    }

    // ── Loading spinner: VALORANT running, fetching data ─────────────────────
    if bg.loading && bg.snapshot.is_none() {
        ui.centered_and_justified(|ui| {
            ui.add(egui::Spinner::new().size(36.0));
        });
        return;
    }

    // ── Idle screen: VALORANT running, not in a match ────────────────────────
    if bg.snapshot.is_none() {
        super::idle::draw_idle_screen(ui, true);
        return;
    }

    let snap = bg.snapshot.as_ref().unwrap().clone();
    let players = sorted_players(&snap);
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
                        "PTY", "AGENT", "NAME", "RANK", "RR", "PEAK", "HS%", "WR%", "K/D",
                        "LVL", "ΔRR", "MET",
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

                        draw_player_row(ui, player, config, &W, short, open_enc);
                        ui.end_row();
                    }
                });
        });
}

/// Draw a single player row inside the match grid.
fn draw_player_row(
    ui: &mut egui::Ui,
    player: &valotracker_core::ResolvedPlayer,
    config: &Config,
    w: &[f32; 12],
    short: bool,
    open_enc: &mut Option<(String, String)>,
) {
    // ── Party ─────────────────────────────────────────────────────────────
    let p_col = if player.is_enemy_party && config.display.highlight_enemy_parties {
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
        [w[0], 20.0],
        egui::Label::new(egui::RichText::new(p_str).color(p_col).monospace()),
    );

    // ── Agent ─────────────────────────────────────────────────────────────
    ui.add_sized(
        [w[1], 20.0],
        egui::Label::new(
            egui::RichText::new(&player.agent_name)
                .color(egui::Color32::from_rgb(200, 200, 200)),
        ),
    );

    // ── Name (clickable if seen before) ───────────────────────────────────
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
                [w[2], 20.0],
                egui::Button::new(egui::RichText::new(&lbl).color(name_col)).frame(false),
            )
            .on_hover_text(format!(
                "Seen {} time(s) before — click for history",
                player.times_seen
            ));
        if resp.clicked() {
            *open_enc = Some((player.puuid.clone(), player.display_name().to_owned()));
        }
    } else {
        ui.add_sized(
            [w[2], 20.0],
            egui::Label::new(egui::RichText::new(&name_str).color(name_col)),
        );
    }

    // ── Rank ──────────────────────────────────────────────────────────────
    let rank_str = if short {
        tier_to_short(player.rank.tier).to_owned()
    } else {
        tier_to_name(player.rank.tier).to_owned()
    };
    ui.add_sized(
        [w[3], 20.0],
        egui::Label::new(
            egui::RichText::new(rank_str)
                .color(colors::rank_color(player.rank.tier))
                .strong(),
        ),
    );

    // ── RR ────────────────────────────────────────────────────────────────
    ui.add_sized(
        [w[4], 20.0],
        egui::Label::new(
            egui::RichText::new(player.rank.rr.to_string())
                .monospace()
                .color(egui::Color32::from_rgb(200, 200, 200)),
        ),
    );

    // ── Peak ──────────────────────────────────────────────────────────────
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
        [w[5], 20.0],
        egui::Label::new(
            egui::RichText::new(peak_str)
                .color(colors::rank_color(player.rank.peak_tier))
                .weak(),
        ),
    );

    // ── HS% ───────────────────────────────────────────────────────────────
    let hs = player.stats.headshot_pct;
    let hs_str = if config.display.show_hs {
        format!("{:.0}%", hs * 100.0)
    } else {
        "—".into()
    };
    ui.add_sized(
        [w[6], 20.0],
        egui::Label::new(egui::RichText::new(hs_str).monospace().color(colors::hs_color(hs))),
    );

    // ── WR% ───────────────────────────────────────────────────────────────
    let wr = player.stats.win_rate;
    let wr_str = if config.display.show_wr {
        format!("{:.0}%", wr * 100.0)
    } else {
        "—".into()
    };
    ui.add_sized(
        [w[7], 20.0],
        egui::Label::new(egui::RichText::new(wr_str).monospace().color(colors::wr_color(wr))),
    );

    // ── K/D ───────────────────────────────────────────────────────────────
    let kd = player.stats.kd_ratio;
    let kd_str = if config.display.show_kd {
        format!("{:.2}", kd)
    } else {
        "—".into()
    };
    ui.add_sized(
        [w[8], 20.0],
        egui::Label::new(egui::RichText::new(kd_str).monospace().color(colors::kd_color(kd))),
    );

    // ── Level ─────────────────────────────────────────────────────────────
    let lvl_str = if config.display.show_level && !player.hide_account_level {
        player.account_level.to_string()
    } else {
        "—".into()
    };
    ui.add_sized(
        [w[9], 20.0],
        egui::Label::new(egui::RichText::new(lvl_str).monospace().color(colors::DIM)),
    );

    // ── ΔRR ──────────────────────────────────────────────────────────────
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
        [w[10], 20.0],
        egui::Label::new(
            egui::RichText::new(rr_str)
                .monospace()
                .color(colors::rr_delta_color(rr_d)),
        ),
    );

    // ── MET ───────────────────────────────────────────────────────────────
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
        [w[11], 20.0],
        egui::Label::new(egui::RichText::new(met_str).monospace().color(met_col)),
    );
}
