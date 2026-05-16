fn main() {
    #[cfg(not(feature = "gui"))]
    {
        eprintln!("valotracker-gui: build with --features gui to enable the desktop GUI");
        eprintln!("  cargo run -p valotracker-gui --features gui");
    }

    #[cfg(feature = "gui")]
    run();
}

#[cfg(feature = "gui")]
mod app;
#[cfg(feature = "gui")]
mod colors;

#[cfg(feature = "gui")]
fn run() {
    use eframe::egui;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ValoTracker — Valorant Tracker")
            .with_inner_size([1300.0, 740.0])
            .with_min_inner_size([900.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "ValoTracker — Valorant Tracker",
        options,
        Box::new(|cc| Ok(Box::new(app::GuiApp::new(cc)))),
    )
    .expect("eframe failed");
}
