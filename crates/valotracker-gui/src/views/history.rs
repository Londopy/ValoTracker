//! Match history list view.

use eframe::egui;
use valotracker_core::{history::SavedMatch, tier_to_short};

use crate::colors;

/// Draw the history list.
///
/// * `history`     — The list of saved matches (may be mutated for deletion).
/// * `history_sel` — Currently selected index.
/// * `delete_id`   — Set to the match ID to delete when the user clicks 🗑.
pub fn draw_history_view(
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
                                [0.0, 20.0],
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
                            egui::Label::new(
                                egui::RichText::new(wl_str).color(wl_col).strong(),
                            ),
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
