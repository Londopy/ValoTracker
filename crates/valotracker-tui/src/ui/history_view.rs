use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

use crate::app::App;

/// Render the match history list view.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    draw_header(frame, chunks[0], app);
    draw_table(frame, chunks[1], app);
    draw_footer(frame, chunks[2]);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    use ratatui::{text::Line, widgets::Paragraph};
    let count = app.history.as_ref().map(|h| h.len()).unwrap_or(0);
    let line = Line::from(format!("  Match History  │  {count} saved matches"));
    frame.render_widget(
        Paragraph::new(line).style(Style::default().fg(Color::Rgb(100, 180, 255))),
        area,
    );
}

fn draw_table(frame: &mut Frame, area: Rect, app: &App) {
    let widths = [
        Constraint::Length(12), // Date
        Constraint::Length(12), // Map
        Constraint::Length(14), // Queue
        Constraint::Length(5),  // W/L
        Constraint::Length(14), // Rank
        Constraint::Length(6),  // ΔRR
    ];

    let header_style = Style::default()
        .fg(Color::Rgb(180, 180, 180))
        .add_modifier(Modifier::BOLD);

    let header = Row::new(vec![
        Cell::from("Date").style(header_style),
        Cell::from("Map").style(header_style),
        Cell::from("Queue").style(header_style),
        Cell::from("W/L").style(header_style),
        Cell::from("Rank").style(header_style),
        Cell::from("ΔRR").style(header_style),
    ]);

    let rows: Vec<Row> = app
        .history
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|m| {
            use valotracker_core::tier_to_short;
            use std::time::{SystemTime, UNIX_EPOCH};

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            let age_days = (now - m.saved_at) / 86400;
            let date_str = if age_days == 0 {
                "Today".to_owned()
            } else if age_days == 1 {
                "Yesterday".to_owned()
            } else {
                format!("{age_days} days ago")
            };

            let (wl_str, wl_color) = match m.won {
                Some(true) => ("W", Color::Rgb(80, 220, 100)),
                Some(false) => ("L", Color::Rgb(220, 80, 80)),
                None => ("?", Color::DarkGray),
            };

            let rr_delta_str = if m.my_rr_delta > 0 {
                format!("+{}", m.my_rr_delta)
            } else {
                m.my_rr_delta.to_string()
            };
            let rr_delta_color = if m.my_rr_delta > 0 {
                Color::Rgb(80, 220, 100)
            } else if m.my_rr_delta < 0 {
                Color::Rgb(220, 80, 80)
            } else {
                Color::DarkGray
            };

            Row::new(vec![
                Cell::from(date_str).style(Style::default().fg(Color::DarkGray)),
                Cell::from(m.map.clone()),
                Cell::from(m.queue.clone()).style(Style::default().fg(Color::Rgb(180, 180, 180))),
                Cell::from(wl_str).style(Style::default().fg(wl_color)),
                Cell::from(tier_to_short(m.my_rank_tier))
                    .style(Style::default().fg(crate::ui::colors::rank_color(m.my_rank_tier))),
                Cell::from(rr_delta_str).style(Style::default().fg(rr_delta_color)),
            ])
        })
        .collect();

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = TableState::default();
    state.select(Some(app.history_selected));
    frame.render_stateful_widget(table, area, &mut state);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    use ratatui::{text::Line, widgets::Paragraph};
    let line = Line::from("  [↑↓] navigate  [d] delete  [Esc] back");
    frame.render_widget(
        Paragraph::new(line).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}
