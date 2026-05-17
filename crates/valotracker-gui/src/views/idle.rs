//! Idle / waiting screen shown when VALORANT is not running or not in a match.

use eframe::egui;

/// Draw the centered idle screen.
///
/// `valorant_running` distinguishes two idle states:
/// * `false` — VALORANT is not even running yet.
/// * `true`  — VALORANT is running but the player is in menus.
pub fn draw_idle_screen(ui: &mut egui::Ui, valorant_running: bool) {
    let t = ui.ctx().input(|i| i.time);

    // Request continuous repaint for the pulse animation.
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(100));

    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);

            // ── Logo / title ──────────────────────────────────────────────────
            ui.label(
                egui::RichText::new("ValoTracker")
                    .size(36.0)
                    .strong()
                    .color(egui::Color32::from_rgb(255, 70, 85)),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Real-time VALORANT match tracker")
                    .size(13.0)
                    .color(egui::Color32::from_rgb(130, 130, 150)),
            );

            ui.add_space(32.0);
            ui.separator();
            ui.add_space(24.0);

            // ── Pulsing status indicator ──────────────────────────────────────
            // Alpha oscillates between 120 and 255 using a sine wave.
            let pulse = ((t * 1.8).sin() * 0.5 + 0.5) as f32;
            let alpha = (120.0 + pulse * 135.0) as u8;

            let (status_text, instruction) = if valorant_running {
                (
                    "● In menu — Waiting for match…",
                    "Queue up in VALORANT to begin tracking",
                )
            } else {
                (
                    "● Waiting for VALORANT…",
                    "Launch VALORANT to begin tracking",
                )
            };

            ui.label(
                egui::RichText::new(status_text)
                    .size(16.0)
                    .color(egui::Color32::from_rgba_unmultiplied(255, 180, 60, alpha)),
            );

            ui.add_space(12.0);

            // ── Current time ──────────────────────────────────────────────────
            let now = chrono::Local::now();
            ui.label(
                egui::RichText::new(now.format("%H:%M:%S").to_string())
                    .size(20.0)
                    .monospace()
                    .color(egui::Color32::from_rgb(180, 180, 200)),
            );

            ui.add_space(12.0);

            // ── Instruction ───────────────────────────────────────────────────
            ui.label(
                egui::RichText::new(instruction)
                    .size(13.0)
                    .color(egui::Color32::from_rgb(100, 100, 120)),
            );

            ui.add_space(24.0);
        });
    });
}
