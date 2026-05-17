//! Player encounter side panel.

use eframe::egui;
use valotracker_core::history::PlayerEncounter;

use crate::colors;

/// Draw the encounter drill-down side panel for a specific player.
///
/// `close` is set to `true` when the user dismisses the panel.
pub fn draw_encounter_panel(
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
