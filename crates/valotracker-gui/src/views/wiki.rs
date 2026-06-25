//! In-app help / wiki modal explaining every feature.

use eframe::egui;

use crate::colors;

// one help section: a bold heading + body lines
struct Section {
    title: &'static str,
    body: &'static [&'static str],
}

const SECTIONS: &[Section] = &[
    Section {
        title: "The player table",
        body: &[
            "Everyone in your match - allies first (highest rank first), then enemies.",
            "Click a name with the eye marker to see your past games with that player.",
        ],
    },
    Section {
        title: "Columns",
        body: &[
            "PTY - party icon (* ^ o #). Same icon = queued together; enemy premades tint red.",
            "AGENT - their agent. Hover it to see their equipped weapon skin.",
            "NAME - Riot name. An eye means you've met them before (click for history).",
            "RANK / RR / PEAK - current rank, ranked rating, and peak rank.",
            "HS% / WR% / K/D - headshots, win rate and kill/death over recent ranked games.",
            "LVL - account level.   dRR - average RR gained/lost per recent game.",
            "FORM - last 5 ranked results, newest first (green W / red L).",
            "MET - how many past matches you've shared with them.",
        ],
    },
    Section {
        title: "Skins",
        body: &[
            "Hover a player's agent to see their primary rifle skin (Vandal/Phantom).",
            "Skin, agent and map names update themselves when Riot ships new content.",
        ],
    },
    Section {
        title: "Encounter history",
        body: &[
            "Matches you play are saved automatically to a local database.",
            "Click a player you've met before to see every shared game: map, agent,",
            "ally vs enemy, their rank then, and whether you won.",
        ],
    },
    Section {
        title: "Updates",
        body: &[
            "ValoTracker checks for a new version on startup and shows an up-arrow when one's out.",
            "You can also hit \"Check for updates now\" in Settings.",
        ],
    },
    Section {
        title: "Settings (gear icon)",
        body: &["Minimize to tray, launch on Windows startup, and toggle update checks."],
    },
    Section {
        title: "Good to know",
        body: &[
            "Stats come from each player's COMPETITIVE history, even when you're in other modes.",
            "Live data needs VALORANT running and you being in agent select or a match.",
            "Premade detection only covers you + friends - Riot hides strangers' parties.",
        ],
    },
];

/// Draw the help/wiki modal window. `open` is flipped to false on close.
pub fn draw_wiki_modal(ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("ValoTracker - Help")
        .collapsible(false)
        .resizable(true)
        .default_width(480.0)
        .default_height(520.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(open)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for section in SECTIONS {
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(section.title)
                                .strong()
                                .size(14.0)
                                .color(colors::HEADER),
                        );
                        ui.separator();
                        for line in section.body {
                            ui.label(
                                egui::RichText::new(*line)
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(190, 190, 200)),
                            );
                        }
                    }
                    ui.add_space(8.0);
                });
        });
}
