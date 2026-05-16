use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;

/// Render the inline config editor as a centred overlay popup.
pub fn draw(frame: &mut Frame, app: &App) {
    let area  = centred_rect(58, 18, frame.area());
    frame.render_widget(Clear, area); // clear behind the popup

    let block = Block::default()
        .title(" ⚙  Config  (Esc to close, changes auto-saved) ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(100, 180, 255)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cfg = &app.config.display;

    let on  = Style::default().fg(Color::Rgb(80, 220, 100)).add_modifier(Modifier::BOLD);
    let off = Style::default().fg(Color::DarkGray);
    let key = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    fn toggle_row<'a>(
        k: &'a str,
        label: &'a str,
        value: bool,
        key_style: Style,
        on: Style,
        off: Style,
    ) -> Row<'a> {
        let (state_str, state_style) = if value {
            ("✓  ON ", on)
        } else {
            ("   OFF", off)
        };
        Row::new(vec![
            Cell::from(Line::from(vec![
                Span::styled(format!("[{k}]"), key_style),
                Span::raw(format!("  {label}")),
            ])),
            Cell::from(Span::styled(state_str, state_style)),
        ])
    }

    let rows = vec![
        toggle_row("s", "Show streamer [S] tag",          cfg.show_streamer_tag,        key, on, off),
        toggle_row("p", "Show party size (3)",             cfg.show_party_size,           key, on, off),
        toggle_row("e", "Highlight enemy premades",        cfg.highlight_enemy_parties,   key, on, off),
        toggle_row("R", "Short rank names  (D2 vs Diamond 2)", cfg.short_ranks,           key, on, off),
        toggle_row("l", "Show account level",              cfg.show_level,                key, on, off),
        toggle_row("k", "Show K/D column",                 cfg.show_kd,                   key, on, off),
        toggle_row("H", "Show HS% column",                 cfg.show_hs,                   key, on, off),
        toggle_row("w", "Show WR% column",                 cfg.show_wr,                   key, on, off),
        toggle_row("d", "Show ΔRR column",                 cfg.show_rr_delta,             key, on, off),
    ];

    let widths = [Constraint::Min(36), Constraint::Length(8)];

    let table = Table::new(rows, widths)
        .block(Block::default().borders(Borders::NONE))
        .column_spacing(2);

    // Split inner area: table rows + bottom hint
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    frame.render_widget(table, chunks[0]);

    let hint = Paragraph::new(
        Line::from(vec![
            Span::styled("Press the key in ", dim),
            Span::styled("[brackets]", Style::default().fg(Color::White)),
            Span::styled(" to toggle  ·  changes are saved instantly", dim),
        ])
    );
    frame.render_widget(hint, chunks[1]);
}

/// Returns a centred Rect of the given percentage of the parent.
fn centred_rect(percent_x: u16, lines: u16, r: Rect) -> Rect {
    let popup_width  = (r.width  * percent_x / 100).min(r.width);
    let popup_height = lines.min(r.height);
    let x = r.x + (r.width.saturating_sub(popup_width))  / 2;
    let y = r.y + (r.height.saturating_sub(popup_height)) / 2;
    Rect::new(x, y, popup_width, popup_height)
}
