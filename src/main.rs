use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::Focus;
use crate::components::Component;
use crate::components::repo_selector::RepoSelector;
use crate::components::session_tree::{SessionEntry, SessionTree};
use crate::components::status_line::StatusLine;
use crate::components::terminal_pane::TerminalPane;
use crate::keys::key_to_bytes;

mod action;
mod app;
mod components;
mod domain;
mod event;
mod keys;
mod tui;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let mut tui = tui::Tui::new()?;
    let mut events = event::EventHandler::new(4.0, 60.0);
    let mut app = app::App::new();

    let mut repo_selector = RepoSelector::new();
    let mut session_tree = SessionTree::new();
    let mut terminal_pane = TerminalPane::new();

    while app.is_running() {
        let event = events.next().await?;
        match event {
            event::Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    if repo_selector.visible {
                        repo_selector.handle_key_event(key);

                        if let Some(result) = repo_selector.take_result() {
                            let size = crossterm::terminal::size().unwrap_or((80, 24));
                            let name = format!("session-{}", app.session_manager.len() + 1);
                            let _ = app.session_manager.create_session(
                                &name,
                                &result.repo_name,
                                &result.branch,
                                &result.working_dir,
                                size.1,
                                size.0,
                            );
                            app.selected_session = app.session_manager.len().saturating_sub(1);
                            app.focus = Focus::Terminal;
                        }
                    } else {
                        match app.focus {
                            Focus::Sessions => {
                                handle_sessions_key(
                                    &mut app,
                                    &mut repo_selector,
                                    key.code,
                                    key.modifiers,
                                );
                            }
                            Focus::Terminal => handle_terminal_key(&mut app, key),
                        }
                    }
                }
            }
            event::Event::Render => {
                session_tree.selected = app.selected_session;
                session_tree.focused = app.focus == Focus::Sessions;
                session_tree.sessions = app
                    .session_manager
                    .sessions()
                    .iter()
                    .map(|s| SessionEntry {
                        branch: s.branch.clone(),
                        name: s.name.clone(),
                        repo: s.repo.clone(),
                    })
                    .collect();
                terminal_pane.focused = app.focus == Focus::Terminal;
                terminal_pane.screen = app
                    .session_manager
                    .get(app.selected_session)
                    .map(|s| s.pty.screen());

                let status_line = build_status_line(&app);

                tui.draw(|frame| {
                    let vertical = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(3), Constraint::Length(1)])
                        .split(frame.area());

                    let horizontal = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
                        .split(vertical[0]);

                    session_tree.render(frame, horizontal[0]);
                    terminal_pane.render(frame, horizontal[1]);
                    status_line.render(frame, vertical[1]);

                    repo_selector.render(frame, frame.area());
                })?;
            }
            event::Event::Resize(..) | event::Event::Tick | event::Event::Error => {}
        }
    }

    tui.exit()?;
    Ok(())
}

fn build_status_line(app: &app::App) -> StatusLine {
    app.session_manager.get(app.selected_session).map_or_else(
        || StatusLine {
            branch: String::new(),
            dir: String::new(),
            repo: String::new(),
            status: "no session".to_owned(),
        },
        |session| StatusLine {
            branch: session.branch.clone(),
            dir: session.pty.working_dir().to_owned(),
            repo: session.repo.clone(),
            status: "running".to_owned(),
        },
    )
}

fn handle_sessions_key(
    app: &mut app::App,
    repo_selector: &mut RepoSelector,
    code: KeyCode,
    modifiers: KeyModifiers,
) {
    match code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.quit();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.select_next_session(app.session_manager.len());
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.select_prev_session();
        }
        KeyCode::Char('n') => {
            repo_selector.open();
        }
        KeyCode::Char('d') => {
            if !app.session_manager.is_empty() {
                app.session_manager.remove_session(app.selected_session);
                if app.selected_session >= app.session_manager.len() && app.selected_session > 0 {
                    app.selected_session -= 1;
                }
            }
        }
        KeyCode::Tab | KeyCode::Enter => {
            if !app.session_manager.is_empty() {
                app.toggle_focus();
            }
        }
        _ => {}
    }
}

fn handle_terminal_key(app: &mut app::App, key: crossterm::event::KeyEvent) {
    if key.code == KeyCode::Tab {
        app.toggle_focus();
        return;
    }

    let bytes = key_to_bytes(key);
    if !bytes.is_empty()
        && let Some(session) = app.session_manager.get_mut(app.selected_session)
    {
        let _ = session.pty.write(&bytes);
    }
}
