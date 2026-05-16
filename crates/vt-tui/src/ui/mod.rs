use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use crate::app::{App, View};

pub mod colors;
pub mod config_view;
pub mod encounter_view;
pub mod header;
pub mod history_view;
pub mod table;

/// Main draw entry point — dispatches to the appropriate view.
pub fn draw(frame: &mut Frame, app: &App) {
    match &app.view {
        View::Match => draw_match(frame, app),
        View::History => history_view::draw(frame, app),
        View::Encounter { .. } => encounter_view::draw(frame, app),
        // Config renders as an overlay on top of the match view
        View::Config => {
            draw_match(frame, app);
            config_view::draw(frame, app);
        }
    }
}

fn draw_match(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header bar
            Constraint::Min(0),    // player table
            Constraint::Length(1), // footer / status bar
        ])
        .split(frame.area());

    header::draw(frame, chunks[0], app);
    table::draw(frame, chunks[1], app);
    draw_footer(frame, chunks[2], app);
}

fn draw_footer(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    app: &App,
) {
    use ratatui::{
        style::{Color, Style},
        text::{Line, Span},
        widgets::Paragraph,
    };

    let dim = Style::default().fg(Color::DarkGray);
    let key = Style::default().fg(Color::White);

    let mut spans = vec![
        Span::styled("[r]", key),
        Span::styled(" refresh  ", dim),
        Span::styled("[s]", key),
        Span::styled(" save  ", dim),
        Span::styled("[h]", key),
        Span::styled(" history  ", dim),
        Span::styled("[c]", key),
        Span::styled(" config  ", dim),
        Span::styled("[q]", key),
        Span::styled(" quit", dim),
    ];

    // Show status message or load time on the right
    let right = if let Some((msg, _)) = &app.status_msg {
        Span::styled(format!("  {msg}"), Style::default().fg(Color::Green))
    } else if let Some(dur) = app.load_duration {
        Span::styled(
            format!("  Loaded in {:.1}s", dur.as_secs_f32()),
            Style::default().fg(Color::DarkGray),
        )
    } else if app.is_loading {
        Span::styled("  Loading…", Style::default().fg(Color::Yellow))
    } else {
        Span::raw("")
    };

    spans.push(right);

    let para = Paragraph::new(Line::from(spans));
    frame.render_widget(para, area);
}
