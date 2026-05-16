use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;

use crate::app::{App, View};

/// Poll for keyboard events and update app state.
/// Returns `true` if the app should quit.
pub async fn handle_events(app: &mut App) -> Result<bool> {
    if !event::poll(Duration::from_millis(50))? {
        return Ok(false);
    }

    let Event::Key(key) = event::read()? else {
        return Ok(false);
    };

    if key.kind != KeyEventKind::Press {
        return Ok(false);
    }

    let quit = match &app.view.clone() {
        View::Match => handle_match_keys(app, key.code).await,
        View::History => {
            handle_history_keys(app, key.code);
            false
        }
        View::Encounter { .. } => {
            handle_encounter_keys(app, key.code);
            false
        }
        View::Config => {
            handle_config_keys(app, key.code);
            false
        }
    };

    Ok(quit)
}

async fn handle_match_keys(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => return true,

        KeyCode::Char('r') => {
            app.refresh().await;
        }

        KeyCode::Char('s') => {
            app.save_current_match().await;
        }

        KeyCode::Char('h') => {
            app.open_history();
        }

        KeyCode::Char('c') => {
            app.view = View::Config;
        }

        KeyCode::Char('t') => {
            app.config.display.show_streamer_tag = !app.config.display.show_streamer_tag;
            let _ = app.config.save();
        }

        KeyCode::Char('p') => {
            app.config.display.show_party_size = !app.config.display.show_party_size;
            let _ = app.config.save();
        }

        KeyCode::Down | KeyCode::Char('j') => {
            let max = app.display_players().len().saturating_sub(1);
            app.selected_row = Some(app.selected_row.map(|r| (r + 1).min(max)).unwrap_or(0));
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.selected_row = Some(app.selected_row.map(|r| r.saturating_sub(1)).unwrap_or(0));
        }

        KeyCode::Enter | KeyCode::Tab => {
            if let Some(idx) = app.selected_row {
                let players = app.display_players();
                if let Some(player) = players.get(idx) {
                    if player.times_seen > 0 {
                        let puuid = player.puuid.clone();
                        let name = player.display_name().to_owned();
                        app.open_encounter(&puuid, &name);
                    }
                }
            }
        }

        _ => {}
    }
    false
}

fn handle_history_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => app.go_back(),
        KeyCode::Down | KeyCode::Char('j') => {
            let max = app
                .history
                .as_ref()
                .map(|h| h.len())
                .unwrap_or(0)
                .saturating_sub(1);
            app.history_selected = (app.history_selected + 1).min(max);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.history_selected = app.history_selected.saturating_sub(1);
        }
        KeyCode::Char('d') => {
            if let Some(history) = &app.history {
                if let Some(m) = history.get(app.history_selected) {
                    let id = m.id.clone();
                    if let Ok(db) = valotracker_core::history::MatchHistory::open() {
                        let _ = db.delete_match(&id);
                    }
                    app.open_history();
                }
            }
        }
        _ => {}
    }
}

fn handle_encounter_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => app.go_back(),
        _ => {}
    }
}

fn handle_config_keys(app: &mut App, code: KeyCode) {
    let cfg = &mut app.config.display;
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.view = View::Match;
        }
        KeyCode::Char('s') => {
            cfg.show_streamer_tag = !cfg.show_streamer_tag;
            let _ = app.config.save();
        }
        KeyCode::Char('p') => {
            cfg.show_party_size = !cfg.show_party_size;
            let _ = app.config.save();
        }
        KeyCode::Char('e') => {
            cfg.highlight_enemy_parties = !cfg.highlight_enemy_parties;
            let _ = app.config.save();
        }
        KeyCode::Char('R') => {
            cfg.short_ranks = !cfg.short_ranks;
            let _ = app.config.save();
        }
        KeyCode::Char('l') => {
            cfg.show_level = !cfg.show_level;
            let _ = app.config.save();
        }
        KeyCode::Char('k') => {
            cfg.show_kd = !cfg.show_kd;
            let _ = app.config.save();
        }
        KeyCode::Char('H') => {
            cfg.show_hs = !cfg.show_hs;
            let _ = app.config.save();
        }
        KeyCode::Char('w') => {
            cfg.show_wr = !cfg.show_wr;
            let _ = app.config.save();
        }
        KeyCode::Char('d') => {
            cfg.show_rr_delta = !cfg.show_rr_delta;
            let _ = app.config.save();
        }
        _ => {}
    }
}
