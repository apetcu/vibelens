use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;

use crate::theme::save_theme;
use crate::tui_app::{App, InputMode, View};

pub fn handle_events(app: &mut App) -> Result<()> {
    if event::poll(Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            // Ctrl+C always quits
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                app.should_quit = true;
                return Ok(());
            }

            match app.input_mode {
                InputMode::Search => handle_search_input(app, key.code),
                InputMode::Normal => handle_normal_input(app, key.code),
            }
        }
    }
    Ok(())
}

fn handle_search_input(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.search_query.clear();
            app.apply_filter();
        }
        KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.apply_filter();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.apply_filter();
        }
        _ => {}
    }
}

fn handle_normal_input(app: &mut App, code: KeyCode) {
    // In SessionDetail, j/k scroll through messages
    if app.view == View::SessionDetail {
        match code {
            KeyCode::Char('q') => {
                app.should_quit = true;
                return;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let msg_count = app
                    .current_project()
                    .and_then(|p| p.sessions.get(app.selected_session))
                    .map(|s| s.messages.len())
                    .unwrap_or(0);
                app.scroll_messages_down(msg_count);
                return;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.scroll_messages_up();
                return;
            }
            KeyCode::Char('d') => {
                let msg_count = app
                    .current_project()
                    .and_then(|p| p.sessions.get(app.selected_session))
                    .map(|s| s.messages.len())
                    .unwrap_or(0);
                for _ in 0..10 {
                    app.scroll_messages_down(msg_count);
                }
                return;
            }
            KeyCode::Char('u') => {
                for _ in 0..10 {
                    app.scroll_messages_up();
                }
                return;
            }
            KeyCode::Char('g') => {
                app.message_scroll = 0;
                return;
            }
            KeyCode::Char('G') => {
                let msg_count = app
                    .current_project()
                    .and_then(|p| p.sessions.get(app.selected_session))
                    .map(|s| s.messages.len())
                    .unwrap_or(0);
                app.message_scroll = msg_count.saturating_sub(1);
                return;
            }
            KeyCode::Esc | KeyCode::Left | KeyCode::Backspace | KeyCode::Char('h') => {
                app.go_back();
                return;
            }
            KeyCode::Char('t') => {
                app.theme = app.theme.next();
                save_theme(app.theme);
                return;
            }
            _ => return,
        }
    }

    match code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.move_down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.move_up();
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            app.enter_selection();
        }
        KeyCode::Esc | KeyCode::Left | KeyCode::Backspace | KeyCode::Char('h') => {
            app.go_back();
        }
        KeyCode::Char('/') => {
            if app.view == View::ProjectList {
                app.input_mode = InputMode::Search;
                app.search_query.clear();
            }
        }
        KeyCode::Char('s') => {
            app.cycle_sort();
        }
        KeyCode::Char('t') => {
            app.theme = app.theme.next();
            save_theme(app.theme);
        }
        KeyCode::Char('u') => {
            app.page_up();
        }
        KeyCode::Char('d') => {
            app.page_down();
        }
        KeyCode::Char('g') => {
            app.go_home();
        }
        KeyCode::Char('G') => {
            app.go_end();
        }
        KeyCode::Char('1') => {
            app.view = View::Dashboard;
            app.view_stack.clear();
        }
        KeyCode::Char('2') => {
            app.view = View::ProjectList;
            app.view_stack.clear();
        }
        KeyCode::Char('3') => {
            if app.current_project().is_some() {
                app.view = View::ProjectDetail;
                app.view_stack.clear();
            }
        }
        KeyCode::Char('4') => {
            if let Some(proj) = app.current_project() {
                if !proj.sessions.is_empty() {
                    app.view = View::SessionDetail;
                    app.view_stack.clear();
                }
            }
        }
        _ => {}
    }
}
