use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

use valotracker_core::{tier_to_name, tier_to_short, ResolvedPlayer};

use crate::{app::App, ui::colors::*};

/// Render the main player table.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    if app.snapshot.is_none() && app.load_error.is_some() {
        draw_waiting(frame, area, app);
        return;
    }

    let players = app.display_players();
    if players.is_empty() {
        draw_waiting(frame, area, app);
        return;
    }

    let cfg = &app.config.display;
    let use_short = cfg.short_ranks;

    // ── Column widths ──────────────────────────────────────────────────────
    // PARTY  AGENT   NAME           RANK      RR    PEAK    HS%   WR%   K/D   LVL   ΔRR  MET
    let widths = [
        ratatui::layout::Constraint::Length(4),  // party icon
        ratatui::layout::Constraint::Length(12), // agent
        ratatui::layout::Constraint::Min(18),    // name
        ratatui::layout::Constraint::Length(12), // rank
        ratatui::layout::Constraint::Length(5),  // RR
        ratatui::layout::Constraint::Length(10), // peak
        ratatui::layout::Constraint::Length(5),  // HS%
        ratatui::layout::Constraint::Length(5),  // WR%
        ratatui::layout::Constraint::Length(5),  // K/D
        ratatui::layout::Constraint::Length(4),  // LVL
        ratatui::layout::Constraint::Length(5),  // ΔRR
        ratatui::layout::Constraint::Length(3),  // MET
    ];

    // ── Header row ─────────────────────────────────────────────────────────
    let header_style = Style::default()
        .fg(HEADER_COLOR)
        .add_modifier(Modifier::BOLD);
    let header = Row::new(vec![
        Cell::from("PTY").style(header_style),
        Cell::from("AGENT").style(header_style),
        Cell::from("NAME").style(header_style),
        Cell::from("RANK").style(header_style),
        Cell::from("RR").style(header_style),
        Cell::from("PEAK").style(header_style),
        Cell::from("HS%").style(header_style),
        Cell::from("WR%").style(header_style),
        Cell::from("K/D").style(header_style),
        Cell::from("LVL").style(header_style),
        Cell::from("ΔRR").style(header_style),
        Cell::from("MET").style(header_style),
    ]);

    // ── Player rows ────────────────────────────────────────────────────────
    let mut rows: Vec<Row> = Vec::new();
    let mut last_team: Option<bool> = None; // true = ally, false = enemy

    for (i, player) in players.iter().enumerate() {
        // Insert a team separator between ally and enemy sides
        if let Some(lt) = last_team {
            if lt != player.is_ally {
                let sep_row = Row::new(vec![Cell::from(
                    "─".repeat(4) + "  ─────────────  ─────────────────  ──────────  ─────  ──────────  ─────  ─────  ─────  ────  ─────  ───",
                )])
                .style(Style::default().fg(SEPARATOR_COLOR));
                rows.push(sep_row);
            }
        }
        last_team = Some(player.is_ally);

        let row = build_player_row(player, i, app, use_short);
        rows.push(row);
    }

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = TableState::default();
    state.select(app.selected_row);

    frame.render_stateful_widget(table, area, &mut state);
}

fn build_player_row<'a>(
    player: &'a ResolvedPlayer,
    _idx: usize,
    app: &App,
    use_short: bool,
) -> Row<'a> {
    let cfg = &app.config.display;

    // ── Party cell ─────────────────────────────────────────────────────────
    let party_color = if player.is_enemy_party && cfg.highlight_enemy_parties {
        PARTY_ENEMY_COLOR
    } else {
        Color::Rgb(180, 180, 220)
    };
    let party_str = if player.party_size > 1 && cfg.show_party_size {
        format!("{} ({})", player.party_icon, player.party_size)
    } else {
        player.party_icon.to_string()
    };
    let party_cell = Cell::from(party_str).style(Style::default().fg(party_color));

    // ── Agent cell ─────────────────────────────────────────────────────────
    let agent_cell =
        Cell::from(player.agent_name.clone()).style(Style::default().fg(Color::Rgb(200, 200, 200)));

    // ── Name cell ──────────────────────────────────────────────────────────
    let name_color = if player.is_ally {
        ALLY_TEAM_COLOR
    } else {
        ENEMY_TEAM_COLOR
    };
    let mut name_spans = vec![Span::styled(
        player.display_name().to_owned(),
        Style::default().fg(name_color),
    )];
    if player.incognito && cfg.show_streamer_tag {
        name_spans.push(Span::styled(
            " [S]",
            Style::default().fg(STREAMER_TAG_COLOR),
        ));
    }
    if player.times_seen > 0 {
        name_spans.push(Span::styled(
            " 👁",
            Style::default().fg(Color::Rgb(200, 200, 100)),
        ));
    }
    let name_cell = Cell::from(Line::from(name_spans));

    // ── Rank cell ──────────────────────────────────────────────────────────
    let rank_name = if use_short {
        tier_to_short(player.rank.tier).to_owned()
    } else {
        tier_to_name(player.rank.tier).to_owned()
    };
    let rank_cell = Cell::from(rank_name).style(Style::default().fg(rank_color(player.rank.tier)));

    // ── RR cell ────────────────────────────────────────────────────────────
    let rr_cell = Cell::from(player.rank.rr.to_string())
        .style(Style::default().fg(Color::Rgb(200, 200, 200)));

    // ── Peak rank cell ────────────────────────────────────────────────────
    let peak_name = if use_short {
        tier_to_short(player.rank.peak_tier).to_owned()
    } else {
        let name = tier_to_name(player.rank.peak_tier);
        // Truncate to 8 chars to save space
        if name.len() > 8 {
            name[..8].to_owned()
        } else {
            name.to_owned()
        }
    };
    let peak_cell = Cell::from(peak_name).style(
        Style::default()
            .fg(rank_color(player.rank.peak_tier))
            .add_modifier(Modifier::DIM),
    );

    // ── HS% cell ───────────────────────────────────────────────────────────
    let hs_val = player.stats.headshot_pct;
    let hs_cell = if cfg.show_hs {
        Cell::from(format!("{:.0}%", hs_val * 100.0)).style(Style::default().fg(hs_color(hs_val)))
    } else {
        Cell::from("—")
    };

    // ── WR% cell ───────────────────────────────────────────────────────────
    let wr_val = player.stats.win_rate;
    let wr_cell = if cfg.show_wr {
        Cell::from(format!("{:.0}%", wr_val * 100.0)).style(Style::default().fg(wr_color(wr_val)))
    } else {
        Cell::from("—")
    };

    // ── K/D cell ───────────────────────────────────────────────────────────
    let kd_val = player.stats.kd_ratio;
    let kd_cell = if cfg.show_kd {
        Cell::from(format!("{:.2}", kd_val)).style(Style::default().fg(kd_color(kd_val)))
    } else {
        Cell::from("—")
    };

    // ── Level cell ────────────────────────────────────────────────────────
    let lvl_cell = if cfg.show_level && !player.hide_account_level {
        Cell::from(player.account_level.to_string()).style(Style::default().fg(DIM_COLOR))
    } else {
        Cell::from("—").style(Style::default().fg(DIM_COLOR))
    };

    // ── ΔRR cell ──────────────────────────────────────────────────────────
    let rr_delta = player.stats.avg_rr_delta;
    let rr_delta_cell = if cfg.show_rr_delta {
        let text = if rr_delta > 0.0 {
            format!("+{:.0}", rr_delta)
        } else {
            format!("{:.0}", rr_delta)
        };
        Cell::from(text).style(Style::default().fg(rr_delta_color(rr_delta)))
    } else {
        Cell::from("—")
    };

    // ── MET (times seen) cell ─────────────────────────────────────────────
    let met_cell = if player.times_seen > 0 {
        Cell::from(player.times_seen.to_string())
            .style(Style::default().fg(Color::Rgb(200, 200, 100)))
    } else {
        Cell::from("—").style(Style::default().fg(DIM_COLOR))
    };

    // ── Row style ─────────────────────────────────────────────────────────
    let row_style = if player.stats.afk_penalty {
        Style::default().add_modifier(Modifier::DIM)
    } else {
        Style::default()
    };

    Row::new(vec![
        party_cell,
        agent_cell,
        name_cell,
        rank_cell,
        rr_cell,
        peak_cell,
        hs_cell,
        wr_cell,
        kd_cell,
        lvl_cell,
        rr_delta_cell,
        met_cell,
    ])
    .style(row_style)
}

fn draw_waiting(frame: &mut Frame, area: Rect, app: &App) {
    use ratatui::widgets::Paragraph;

    let msg = if app.is_loading {
        "⟳  Loading player data…"
    } else if let Some(err) = &app.load_error {
        err.as_str()
    } else {
        "Waiting for VALORANT to launch…"
    };

    let para = Paragraph::new(msg)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::NONE));

    frame.render_widget(para, area);
}
