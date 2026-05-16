use ratatui::style::Color;

/// Convert an RGB tuple to a ratatui Color.
#[allow(dead_code)]
pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

/// Color for a rank tier (delegates to valotracker-core).
pub fn rank_color(tier: u8) -> Color {
    let (r, g, b) = valotracker_core::tier_to_color(tier);
    Color::Rgb(r, g, b)
}

/// Color for a headshot percentage value.
///
/// - Green  : hs ≥ 0.35
/// - Yellow : hs ≥ 0.20
/// - Red    : hs < 0.20
pub fn hs_color(hs: f32) -> Color {
    if hs >= 0.35 {
        Color::Rgb(80, 220, 100)
    } else if hs >= 0.20 {
        Color::Rgb(220, 200, 60)
    } else {
        Color::Rgb(220, 80, 80)
    }
}

/// Color for a win-rate value.
///
/// - Green  : wr ≥ 0.55
/// - Yellow : wr ≥ 0.45
/// - Red    : wr < 0.45
pub fn wr_color(wr: f32) -> Color {
    if wr >= 0.55 {
        Color::Rgb(80, 220, 100)
    } else if wr >= 0.45 {
        Color::Rgb(220, 200, 60)
    } else {
        Color::Rgb(220, 80, 80)
    }
}

/// Color for a K/D ratio.
///
/// - Green  : kd ≥ 1.2
/// - Yellow : kd ≥ 0.85
/// - Red    : kd < 0.85
pub fn kd_color(kd: f32) -> Color {
    if kd >= 1.2 {
        Color::Rgb(80, 220, 100)
    } else if kd >= 0.85 {
        Color::Rgb(220, 200, 60)
    } else {
        Color::Rgb(220, 80, 80)
    }
}

/// Color for an RR delta.
pub fn rr_delta_color(delta: f32) -> Color {
    if delta > 0.0 {
        Color::Rgb(80, 220, 100)
    } else if delta == 0.0 {
        Color::DarkGray
    } else {
        Color::Rgb(220, 80, 80)
    }
}

pub const ALLY_TEAM_COLOR: Color = Color::Rgb(80, 150, 240);
pub const ENEMY_TEAM_COLOR: Color = Color::Rgb(240, 80, 80);
pub const STREAMER_TAG_COLOR: Color = Color::Rgb(255, 165, 0);
pub const PARTY_ENEMY_COLOR: Color = Color::Rgb(240, 80, 80);
pub const SEPARATOR_COLOR: Color = Color::DarkGray;
pub const HEADER_COLOR: Color = Color::Rgb(180, 180, 180);
pub const DIM_COLOR: Color = Color::DarkGray;
 = Color::DarkGray;
