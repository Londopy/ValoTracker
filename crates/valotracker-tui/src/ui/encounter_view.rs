use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use valotracker_core::history::{summarize_encounters, PlayerEncounter};

use crate::app::App;

/// Render the per-player encounter drill-down panel.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Title
            Constraint::Min(0),    // Table
            Constraint::Length(3), // Summary
            Constraint::Length(1), // Footer
        ])
        .split(area);

    let encounters = app.encounter_data.as_deref().unwrap_or(&[]);
    let name = &app.encounter_name;

    draw_title(frame, chunks[0], name, encounters.len());
    draw_table(frame, chunks[1], encounters);
    draw_summary(frame, chunks[2], encounters);
    draw_footer(frame, chunks[3]);
}

fn draw_title(frame: &mut Frame, area: Rect, name: &str, count: usize) {
    let line = Line::from(vec![
        Span::styled(
            format!("  {name}"),
            Style::default()
                .fg(Color::Rgb(100, 180, 255))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  —  {count} previous encounters"),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_table(frame: &mut Frame, area: Rect, encounters: &[PlayerEncounter]) {
    let widths = [
        Constraint::Length(12), // Date
        Constraint::Length(10), // Map
        Constraint::Length(10), // Agent
        Constraint::Length(4),  // K
        Constraint::Length(4),  // D
        Constraint::Length(4),  // A
        Constraint::Length(5),  // W/L
    ];

    let header_style = Style::default()
        .fg(Color::Rgb(180, 180, 180))
        .add_modifier(Modifier::BOLD);

    let header = Row::new(vec![
        Cell::from("Date").style(header_style),
        Cell::from("Map").style(header_style),
        Cell::from("Agent").style(header_style),
        Cell::from("K").style(header_style),
        Cell::from("D").style(header_style),
        Cell::from("A").style(header_style),
        Cell::from("W/L").style(header_style),
    ]);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let rows: Vec<Row> = encounters
        .iter()
        .map(|e| {
            let age_days = (now - e.saved_at) / 86400;
            let date_str = if age_days == 0 {
                "Today".to_owned()
            } else if age_days == 1 {
                "Yesterday".to_owned()
            } else {
                format!("{age_days} days ago")
            };

            let (wl, wl_color) = match e.won {
                Some(true) => ("W", Color::Rgb(80, 220, 100)),
                Some(false) => ("L", Color::Rgb(220, 80, 80)),
                None => ("?", Color::DarkGray),
            };

            Row::new(vec![
                Cell::from(date_str).style(Style::default().fg(Color::DarkGray)),
                Cell::from(e.map.clone()),
                Cell::from(e.agent.clone()).style(Style::default().fg(Color::Rgb(200, 200, 200))),
                Cell::from(e.kills.to_string()).style(Style::default().fg(Color::Rgb(80, 220, 100))),
                Cell::from(e.deaths.to_string()).style(Style::default().fg(Color::Rgb(220, 80, 80))),
                Cell::from(e.assists.to_string()).style(Style::default().fg(Color::Rgb(200, 200, 100))),
                Cell::from(wl).style(Style::default().fg(wl_color)),
            ])
        })
        .collect();

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::NONE));

    frame.render_widget(table, area);
}

fn draw_summary(frame: &mut Frame, area: Rect, encounters: &[PlayerEncounter]) {
    if encounters.is_empty() {
        return;
    }

    let summary = summarize_encounters(encounters);

    // "taunt threshold" easter egg
    let emoji = if let Some(worst) = &summary.worst_game {
        if worst.deaths >= 15 && worst.kills <= 8 {
            "💀"
        } else {
            "👀"
        }
    } else {
        "👀"
    };

    let w_l = format!("{}-{}", summary.wins_against, summary.losses_against);
    let text = format!(
        "  Avg: {:.0}K / {:.0}D  HS: {:.0}%  W-L: {}  Usually plays: {}  {}",
        summary.avg_kills,
        summary.avg_deaths,
        summary.avg_hs_pct * 100.0,
        w_l,
        summary.most_played_agent,
        emoji,
    );

    let style = if emoji == "💀" {
        Style::default().fg(Color::Rgb(220, 80, 80))
    } else {
        Style::default().fg(Color::Rgb(180, 180, 180))
    };

    frame.render_widget(
        Paragraph::new(text)
            .style(style)
            .block(Block::default().borders(Borders::TOP)),
        area,
    );
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    use ratatui::widgets::Paragraph;
    frame.render_widget(
        Paragraph::new("  [Esc] back").style(Style::default().fg(Color::DarkGray)),
        area,
    );
}
