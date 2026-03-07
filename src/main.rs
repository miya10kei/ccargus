use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::Focus;
use crate::components::Component;
use crate::components::session_tree::SessionTree;
use crate::components::status_line::StatusLine;
use crate::components::terminal_pane::TerminalPane;

mod action;
mod app;
mod components;
mod domain;
mod event;
mod tui;

const SESSION_COUNT: usize = 4;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let mut tui = tui::Tui::new()?;
    let mut events = event::EventHandler::new(4.0, 60.0);
    let mut app = app::App::new();

    let mut session_tree = SessionTree::new();
    let mut terminal_pane = TerminalPane::new();
    let status_line = StatusLine {
        branch: "main".to_owned(),
        dir: "~/dev/ccargus".to_owned(),
        repo: "miya10kei/ccargus".to_owned(),
        status: "idle".to_owned(),
    };

    while app.is_running() {
        let event = events.next().await?;
        match event {
            event::Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => app.quit(),
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.quit();
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.select_next_session(SESSION_COUNT);
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.select_prev_session();
                        }
                        KeyCode::Tab => {
                            app.toggle_focus();
                        }
                        _ => {}
                    }
                }
            }
            event::Event::Render => {
                session_tree.selected = app.selected_session;
                session_tree.focused = app.focus == Focus::Sessions;
                terminal_pane.focused = app.focus == Focus::Terminal;

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
                })?;
            }
            event::Event::Resize(..) | event::Event::Tick | event::Event::Error => {}
        }
    }

    tui.exit()?;
    Ok(())
}
