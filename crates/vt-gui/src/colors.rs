use eframe::egui;

/// Rank tier → Color32. Tiers 0-27 (Unranked → Radiant).
pub fn rank_color(tier: u8) -> egui::Color32 {
    match tier {
        0        => egui::Color32::DARK_GRAY,
        1..=3    => egui::Color32::from_rgb(130, 110, 100), // Iron
        4..=6    => egui::Color32::from_rgb(180, 130,  90), // Bronze
        7..=9    => egui::Color32::from_rgb(190, 190, 200), // Silver
        10..=12  => egui::Color32::from_rgb(220, 190,  80), // Gold
        13..=15  => egui::Color32::from_rgb(100, 210, 195), // Platinum
        16..=18  => egui::Color32::from_rgb( 80, 160, 255), // Diamond
        19..=21  => egui::Color32::from_rgb(100, 230, 140), // Ascendant
        22..=24  => egui::Color32::from_rgb(230,  80, 100), // Immortal
        25..=27  => egui::Color32::from_rgb(255, 220,  60), // Radiant
        _        => egui::Color32::GRAY,
    }
}

/// Headshot % gradient: green ≥30%, yellow ≥20%, red <20%.
pub fn hs_color(hs: f32) -> egui::Color32 {
    if hs >= 0.30 {
        egui::Color32::from_rgb(80, 220, 100)
    } else if hs >= 0.20 {
        egui::Color32::from_rgb(220, 200, 60)
    } else {
        egui::Color32::from_rgb(220, 80, 80)
    }
}

/// Win-rate gradient: green ≥55%, yellow ≥45%, red <45%.
pub fn wr_color(wr: f32) -> egui::Color32 {
    if wr >= 0.55 {
        egui::Color32::from_rgb(80, 220, 100)
    } else if wr >= 0.45 {
        egui::Color32::from_rgb(220, 200, 60)
    } else {
        egui::Color32::from_rgb(220, 80, 80)
    }
}

/// K/D gradient: green ≥1.2, yellow ≥0.85, red <0.85.
pub fn kd_color(kd: f32) -> egui::Color32 {
    if kd >= 1.2 {
        egui::Color32::from_rgb(80, 220, 100)
    } else if kd >= 0.85 {
        egui::Color32::from_rgb(220, 200, 60)
    } else {
        egui::Color32::from_rgb(220, 80, 80)
    }
}

/// RR delta gradient: green >0, red <0, grey = 0.
pub fn rr_delta_color(delta: f32) -> egui::Color32 {
    if delta > 0.0 {
        egui::Color32::from_rgb(80, 220, 100)
    } else if delta < 0.0 {
        egui::Color32::from_rgb(220, 80, 80)
    } else {
        egui::Color32::DARK_GRAY
    }
}

// ── Named palette constants ───────────────────────────────────────────────────

pub const ALLY_COLOR:      egui::Color32 = egui::Color32::from_rgb( 80, 160, 255);
pub const ENEMY_COLOR:     egui::Color32 = egui::Color32::from_rgb(255,  80,  80);
pub const ACCENT:          egui::Color32 = egui::Color32::from_rgb(255,  70,  85);
pub const DIM:             egui::Color32 = egui::Color32::DARK_GRAY;
pub const HEADER:          egui::Color32 = egui::Color32::from_rgb(160, 160, 160);
pub const PARTY_ENEMY:     egui::Color32 = egui::Color32::from_rgb(240,  80,  80);
pub const STREAMER_TAG:    egui::Color32 = egui::Color32::from_rgb(255, 165,   0);
pub const MET_COLOR:       egui::Color32 = egui::Color32::from_rgb(220, 200,  80);
pub const BG_PANEL:        egui::Color32 = egui::Color32::from_rgb( 18,  18,  25);
pub const BG_CENTRAL:      egui::Color32 = egui::Color32::from_rgb( 15,  15,  20);
pub const BG_STATUSBAR:    egui::Color32 = egui::Color32::from_rgb( 12,  12,  18);
pub const BG_ROW_ALT:      egui::Color32 = egui::Color32::from_rgba_premultiplied(255, 255, 255, 4);
