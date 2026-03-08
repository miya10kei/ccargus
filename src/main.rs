use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::Focus;
use crate::components::Component;
use crate::components::qa_selector::{QaMode, QaSelector};
use crate::components::repo_selector::RepoSelector;
use crate::components::session_tree::{SessionEntry, SessionTree};
use crate::components::status_line::StatusLine;
use crate::components::terminal_pane::TerminalPane;
use crate::keys::key_to_bytes;

mod action;
mod app;
mod components;
mod config;
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

    let mut qa_selector = QaSelector::new();
    let mut repo_selector = RepoSelector::new();
    let mut session_tree = SessionTree::new();
    let mut terminal_pane = TerminalPane::new();

    while app.is_running() {
        let event = events.next().await?;
        match event {
            event::Event::Key(key) if key.kind == KeyEventKind::Press => {
                handle_key_press(&mut app, &mut repo_selector, &mut qa_selector, key);
            }
            event::Event::Render => {
                update_components(&app, &mut session_tree, &mut terminal_pane);
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
                    qa_selector.render(frame, frame.area());
                })?;
            }
            _ => {}
        }
    }

    tui.exit()?;
    Ok(())
}

fn handle_key_press(
    app: &mut app::App,
    repo_selector: &mut RepoSelector,
    qa_selector: &mut QaSelector,
    key: crossterm::event::KeyEvent,
) {
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
    } else if qa_selector.visible {
        qa_selector.handle_key_event(key);

        if let Some(mode) = qa_selector.take_result() {
            let size = crossterm::terminal::size().unwrap_or((80, 24));
            let fork = mode == QaMode::Fork;
            if let Some(session) = app.session_manager.get_mut(app.selected_session) {
                let _ = session.create_qa_session(fork, size.1, size.0);
            }
            app.focus = Focus::QaTerminal;
        }
    } else {
        match app.focus {
            Focus::Sessions => {
                handle_sessions_key(app, repo_selector, qa_selector, key.code, key.modifiers);
            }
            Focus::Terminal => handle_terminal_key(app, key),
            Focus::QaTerminal => handle_qa_terminal_key(app, key),
        }
    }
}

fn update_components(
    app: &app::App,
    session_tree: &mut SessionTree,
    terminal_pane: &mut TerminalPane,
) {
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
    terminal_pane.qa_focused = app.focus == Focus::QaTerminal;
    terminal_pane.screen = app
        .session_manager
        .get(app.selected_session)
        .map(|s| s.pty.screen());
    terminal_pane.qa_screen = app
        .session_manager
        .get(app.selected_session)
        .and_then(|s| s.qa_pty.as_ref().map(domain::pty::PtySession::screen));
}

fn build_status_line(app: &app::App) -> StatusLine {
    app.session_manager.get(app.selected_session).map_or_else(
        || StatusLine {
            branch: String::new(),
            dir: String::new(),
            qa_mode: None,
            repo: String::new(),
            status: "no session".to_owned(),
        },
        |session| {
            let qa_mode = if session.has_qa_session() {
                Some("active".to_owned())
            } else {
                None
            };
            StatusLine {
                branch: session.branch.clone(),
                dir: session.pty.working_dir().to_owned(),
                qa_mode,
                repo: session.repo.clone(),
                status: "running".to_owned(),
            }
        },
    )
}

fn handle_qa_terminal_key(app: &mut app::App, key: crossterm::event::KeyEvent) {
    if key.code == KeyCode::Tab {
        let has_qa = app
            .session_manager
            .get(app.selected_session)
            .is_some_and(domain::session::SessionInfo::has_qa_session);
        app.toggle_focus(has_qa);
        return;
    }

    // Ctrl+w toggles between Terminal and QaTerminal
    if key.code == KeyCode::Char('w') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.toggle_terminal_qa_focus();
        return;
    }

    // Ctrl+d closes Q&A session
    if key.code == KeyCode::Char('d') && key.modifiers.contains(KeyModifiers::CONTROL) {
        if let Some(session) = app.session_manager.get_mut(app.selected_session) {
            session.close_qa_session();
        }
        app.focus = Focus::Terminal;
        return;
    }

    let bytes = key_to_bytes(key);
    if !bytes.is_empty()
        && let Some(session) = app.session_manager.get_mut(app.selected_session)
        && let Some(qa) = &mut session.qa_pty
    {
        let _ = qa.write(&bytes);
    }
}

fn handle_sessions_key(
    app: &mut app::App,
    repo_selector: &mut RepoSelector,
    qa_selector: &mut QaSelector,
    code: KeyCode,
    modifiers: KeyModifiers,
) {
    match code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.quit();
        }
        KeyCode::Char('d') => {
            if !app.session_manager.is_empty() {
                app.session_manager.remove_session(app.selected_session);
                if app.selected_session >= app.session_manager.len() && app.selected_session > 0 {
                    app.selected_session -= 1;
                }
            }
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
        KeyCode::Char('s') => {
            if !app.session_manager.is_empty() {
                qa_selector.open();
            }
        }
        KeyCode::Tab | KeyCode::Enter => {
            if !app.session_manager.is_empty() {
                let has_qa = app
                    .session_manager
                    .get(app.selected_session)
                    .is_some_and(domain::session::SessionInfo::has_qa_session);
                app.toggle_focus(has_qa);
            }
        }
        _ => {}
    }
}

fn handle_terminal_key(app: &mut app::App, key: crossterm::event::KeyEvent) {
    if key.code == KeyCode::Tab {
        let has_qa = app
            .session_manager
            .get(app.selected_session)
            .is_some_and(domain::session::SessionInfo::has_qa_session);
        app.toggle_focus(has_qa);
        return;
    }

    // Ctrl+w toggles between Terminal and QaTerminal
    if key.code == KeyCode::Char('w') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.toggle_terminal_qa_focus();
        return;
    }

    let bytes = key_to_bytes(key);
    if !bytes.is_empty()
        && let Some(session) = app.session_manager.get_mut(app.selected_session)
    {
        let _ = session.pty.write(&bytes);
    }
}
