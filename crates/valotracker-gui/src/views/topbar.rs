//! Top bar and status bar draw functions.

use std::time::Instant;

use eframe::egui;

use crate::{
    app::{BgState, Tab},
    colors,
};

/// Draw the horizontal top bar: logo, match info, tab selector, and action buttons.
pub fn draw_topbar(
    ui: &mut egui::Ui,
    bg: &BgState,
    tab: &mut Tab,
    do_refresh: &mut bool,
    do_save: &mut bool,
    do_history: &mut bool,
    do_settings: &mut bool,
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
                egui::RichText::new(format_queue(&snap.queue_id))
                    .color(egui::Color32::from_rgb(180, 180, 180)),
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

            // Settings gear (far right)
            if ui.button("⚙").on_hover_text("Settings").clicked() {
                *do_settings = true;
            }
            ui.separator();

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

fn format_queue(queue_id: &str) -> &str {
    match queue_id {
        "competitive" => "Competitive",
        "unrated" => "Unrated",
        "spikerush" => "Spike Rush",
        "deathmatch" => "Deathmatch",
        "ggteam" => "Escalation",
        "onefa" => "Replication",
        "snowball" => "Snowball Fight",
        "swiftplay" => "Swiftplay",
        "custom" => "Custom Game",
        "unknown" => "—",
        other => other,
    }
}

/// Draw the slim status bar at the bottom of the window.
pub fn draw_statusbar(ui: &mut egui::Ui, bg: &BgState, status_msg: &Option<(String, Instant)>) {
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
