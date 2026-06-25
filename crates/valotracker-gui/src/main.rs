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
mod startup;
#[cfg(feature = "gui")]
mod views;

#[cfg(feature = "gui")]
fn run() {
    use eframe::egui;

    init_file_logging();

    // Launched from the Windows startup registry entry → start hidden in tray
    let start_minimized = std::env::args().any(|a| a == "--minimized");

    // Load the app icon (compiled into the binary at build time).
    let icon = {
        let bytes = include_bytes!("../assets/icon.png");
        eframe::icon_data::from_png_bytes(bytes).expect("invalid icon PNG")
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ValoTracker — Valorant Tracker")
            .with_inner_size([1300.0, 740.0])
            .with_min_inner_size([900.0, 500.0])
            .with_visible(!start_minimized)
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };

    eframe::run_native(
        "ValoTracker — Valorant Tracker",
        options,
        Box::new(|cc| Ok(Box::new(app::GuiApp::new(cc)))),
    )
    .expect("eframe failed");
}

// logs to a file because theres no console when you just double-click the exe.
// lands in %LOCALAPPDATA%\ValoTracker\valotracker.log so we can actually see
// whats going on (like why ranks arent loading).
#[cfg(feature = "gui")]
fn init_file_logging() {
    use tracing_subscriber::EnvFilter;
    let dir = std::env::var("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("ValoTracker");
    let _ = std::fs::create_dir_all(&dir);
    let appender = tracing_appender::rolling::never(&dir, "valotracker.log");
    let _ = tracing_subscriber::fmt()
        .with_writer(appender)
        .with_ansi(false)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init();
}
