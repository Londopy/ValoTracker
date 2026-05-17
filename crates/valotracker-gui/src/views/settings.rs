//! Settings modal window.

use eframe::egui;
use valotracker_core::Config;

use crate::colors;

/// Draw the settings window.
///
/// Returns a status message string if something noteworthy happened (e.g. a
/// registry write failed).
pub fn draw_settings_modal(
    ctx: &egui::Context,
    config: &mut Config,
    open: &mut bool,
) -> Option<String> {
    let mut status: Option<String> = None;

    egui::Window::new("⚙  Settings")
        .collapsible(false)
        .resizable(false)
        .min_width(340.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(open)
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Window").strong().color(colors::HEADER));
            ui.separator();

            let mut changed = false;

            if ui
                .checkbox(
                    &mut config.features.minimize_to_tray,
                    "Minimize to tray when window is closed",
                )
                .changed()
            {
                changed = true;
            }

            ui.add_space(2.0);

            if ui
                .checkbox(
                    &mut config.features.run_on_startup,
                    "Launch ValoTracker when Windows starts",
                )
                .on_hover_text("Adds an entry to HKCU\\...\\Run in the Windows registry")
                .changed()
            {
                changed = true;
                if let Err(e) = crate::startup::set_run_on_startup(config.features.run_on_startup) {
                    status = Some(format!("Startup registry error: {e}"));
                }
            }

            if changed {
                if let Err(e) = config.save() {
                    status = Some(format!("Config save failed: {e}"));
                }
            }

            ui.add_space(8.0);
            ui.separator();
            ui.label(
                egui::RichText::new("Changes are saved automatically.")
                    .small()
                    .color(colors::DIM),
            );
        });

    status
}
