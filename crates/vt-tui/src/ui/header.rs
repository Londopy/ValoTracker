use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;

/// Render the single-line header bar.
///
/// ```text
///  vt v0.1.0  │  Ascent  │  Competitive  │  EU-WEST  │  14:23
/// ```
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);
    let accent = Style::default().fg(Color::Rgb(100, 180, 255));
    let sep = Span::styled("  │  ", dim);

    let version = Span::styled(
        "vt v0.1.0",
        bold.fg(Color::Rgb(255, 215, 0)),
    );

    let (map_str, queue_str, server_str) = if let Some(snap) = &app.snapshot {
        (
            snap.map_name.clone(),
            format_queue(&snap.queue_id),
            snap.server
                .split('.')
                .nth(2)
                .unwrap_or(&snap.server)
                .to_uppercase(),
        )
    } else {
        ("—".to_owned(), "—".to_owned(), "—".to_owned())
    };

    let time_str = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let h = (secs / 3600) % 24;
        let m = (secs / 60) % 60;
        format!("{h:02}:{m:02}")
    };

    // Game state indicator
    let state_span = if app.is_loading {
        Span::styled("⟳ Loading", Style::default().fg(Color::Yellow))
    } else if let Some(snap) = &app.snapshot {
        Span::styled(snap.game_state.label(), accent)
    } else if let Some(err) = &app.load_error {
        Span::styled(err.as_str(), Style::default().fg(Color::Red))
    } else {
        Span::styled("Waiting for VALORANT…", dim)
    };

    let line = Line::from(vec![
        version,
        sep.clone(),
        Span::styled(map_str, accent),
        sep.clone(),
        Span::styled(queue_str, Style::default().fg(Color::Rgb(180, 180, 180))),
        sep.clone(),
        Span::styled(server_str, Style::default().fg(Color::Rgb(150, 150, 150))),
        sep.clone(),
        state_span,
        sep.clone(),
        Span::styled(time_str, dim),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

fn format_queue(queue_id: &str) -> String {
    match queue_id {
        "competitive" => "Competitive".to_owned(),
        "unrated" => "Unrated".to_owned(),
        "spikerush" => "Spike Rush".to_owned(),
        "deathmatch" => "Deathmatch".to_owned(),
        "ggteam" => "Escalation".to_owned(),
        "onefa" => "Replication".to_owned(),
        "snowball" => "Snowball Fight".to_owned(),
        "swiftplay" => "Swiftplay".to_owned(),
        other => {
            let mut s = other.to_owned();
            if let Some(c) = s.get_mut(0..1) {
                c.make_ascii_uppercase();
            }
            s
        }
    }
}
