//! Player encounter side panel.

use eframe::egui;
use valotracker_core::{history::PlayerEncounter, teammates::Teammate, tier_to_short};

use crate::colors;

/// Draw the encounter drill-down side panel for a specific player.
///
/// `close` is set to `true` when the user dismisses the panel.
#[allow(clippy::too_many_arguments)]
pub fn draw_encounter_panel(
    ui: &mut egui::Ui,
    name: &str,
    encounters: &[PlayerEncounter],
    close: &mut bool,
    tm_running: bool,
    tm_progress: Option<(usize, usize)>,
    tm_results: Option<&[Teammate]>,
    tm_error: Option<&str>,
    tm_start: &mut bool,
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
            "{icon}  {}-{} W/L vs you  ·  Avg HS {:.0}%  ·  Usually {}",
            summary.wins_against,
            summary.losses_against,
            summary.avg_hs_pct * 100.0,
            summary.most_played_agent,
        ))
        .color(sum_col),
    );

    ui.separator();

    // who they play with (opt-in scan)
    draw_teammates_section(ui, tm_running, tm_progress, tm_results, tm_error, tm_start);
    ui.separator();

    // Encounter table
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("enc_grid")
                .num_columns(7)
                .striped(true)
                .spacing([8.0, 3.0])
                .show(ui, |ui| {
                    for h in ["Date", "Map", "Agent", "Rank", "K/D", "HS%", "W/L"] {
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
                            egui::RichText::new(tier_to_short(enc.rank_tier))
                                .color(colors::rank_color(enc.rank_tier))
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2}", enc.kd_ratio))
                                .color(colors::kd_color(enc.kd_ratio))
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

// the "who they play with" block at the top of the panel: a button, a live
// progress bar while it trickles, then the recurring-teammates list.
fn draw_teammates_section(
    ui: &mut egui::Ui,
    running: bool,
    progress: Option<(usize, usize)>,
    results: Option<&[Teammate]>,
    error: Option<&str>,
    start: &mut bool,
) {
    ui.label(
        egui::RichText::new("Who they play with")
            .strong()
            .color(egui::Color32::from_rgb(200, 200, 215)),
    );

    if running {
        let (done, total) = progress.unwrap_or((0, 0));
        let frac = if total > 0 {
            done as f32 / total as f32
        } else {
            0.0
        };
        ui.add(
            egui::ProgressBar::new(frac)
                .desired_width(380.0)
                .text(format!("scanning {done}/{total} games…")),
        );
        ui.label(
            egui::RichText::new("trickling slowly so riot doesnt rate-limit the account")
                .color(colors::DIM)
                .small(),
        );
    } else if let Some(list) = results {
        if list.is_empty() {
            ui.label(
                egui::RichText::new("No recurring teammates in their last 15 comp games.")
                    .color(colors::DIM)
                    .small(),
            );
        } else {
            for t in list {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("×{}", t.games))
                            .strong()
                            .monospace()
                            .color(egui::Color32::from_rgb(120, 200, 255)),
                    );
                    ui.label(egui::RichText::new(&t.name).small());
                });
            }
        }
        if ui.button("↻ Re-scan").clicked() {
            *start = true;
        }
    } else if let Some(err) = error {
        ui.label(
            egui::RichText::new(format!("Couldn't scan: {err}"))
                .color(egui::Color32::from_rgb(220, 120, 120))
                .small(),
        );
        if ui.button("Try again").clicked() {
            *start = true;
        }
    } else {
        ui.label(
            egui::RichText::new("Scan their last 15 competitive games for recurring teammates. Takes ~25s (trickled to stay safe).")
                .color(colors::DIM)
                .small(),
        );
        if ui.button("🔍 Find who they play with").clicked() {
            *start = true;
        }
    }
}
