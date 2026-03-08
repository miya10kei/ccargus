use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::Focus;
use crate::components::Component;
use crate::components::confirm_dialog::{ConfirmAction, ConfirmDialog};
use crate::components::editor_float::EditorFloat;
use crate::components::qa_selector::{QaMode, QaSelector};
use crate::components::repo_selector::RepoSelector;
use crate::components::status_line::StatusLine;
use crate::components::terminal_pane::TerminalPane;
use crate::components::worktree_tree::{WorktreeItem, WorktreeTree};
use crate::config::Config;
use crate::domain::worktree::WorktreeManager;
use crate::keys::{key_to_bytes, mouse_to_bytes};

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
    let config = Config::load()?;
    let worktree_manager = WorktreeManager::new(config.worktree.base_dir.clone())?;

    // Scan existing worktrees on startup
    let entries = worktree_manager.scan()?;
    app.worktree_pool.sync_with_worktrees(&entries);

    let mut confirm_dialog = ConfirmDialog::new();
    let mut editor_float = EditorFloat::new();
    let mut qa_selector = QaSelector::new();
    let mut repo_selector = RepoSelector::new();
    let mut worktree_tree = WorktreeTree::new();
    let mut terminal_pane = TerminalPane::new();

    let mut needs_render = true;

    while app.is_running() {
        let event = events.next().await?;
        match event {
            event::Event::Key(key) if key.kind == KeyEventKind::Press => {
                handle_key_press(
                    &mut app,
                    &mut confirm_dialog,
                    &mut editor_float,
                    &config,
                    &worktree_manager,
                    &mut repo_selector,
                    &mut qa_selector,
                    &mut terminal_pane,
                    key,
                );
                needs_render = true;
            }
            event::Event::Mouse(mouse) => {
                handle_mouse_event(&mut app, &mut editor_float, &mut terminal_pane, mouse);
                needs_render = true;
            }
            event::Event::Render => {
                let pty_dirty = app
                    .worktree_pool
                    .get(app.selected_worktree)
                    .is_some_and(domain::worktree::Worktree::any_pty_dirty);
                let editor_dirty = editor_float.visible && editor_float.is_dirty();

                if !needs_render && !pty_dirty && !editor_dirty {
                    continue;
                }

                if let Some(wt) = app.worktree_pool.get(app.selected_worktree) {
                    wt.clear_pty_dirty();
                }
                editor_float.clear_dirty();
                needs_render = false;

                update_components(&app, &mut worktree_tree, &mut terminal_pane);
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

                    worktree_tree.render(frame, horizontal[0]);
                    terminal_pane.render(frame, horizontal[1]);
                    status_line.render(frame, vertical[1]);

                    repo_selector.render(frame, frame.area());
                    qa_selector.render(frame, frame.area());
                    confirm_dialog.render(frame, frame.area());
                    editor_float.render(frame, frame.area());
                })?;
            }
            event::Event::Resize(_, _) => {
                needs_render = true;
            }
            _ => {}
        }
    }

    tui.exit()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_key_press(
    app: &mut app::App,
    confirm_dialog: &mut ConfirmDialog,
    editor_float: &mut EditorFloat,
    config: &Config,
    worktree_manager: &WorktreeManager,
    repo_selector: &mut RepoSelector,
    qa_selector: &mut QaSelector,
    terminal_pane: &mut TerminalPane,
    key: crossterm::event::KeyEvent,
) {
    if editor_float.visible {
        if key.code == KeyCode::Esc {
            editor_float.close();
            return;
        }
        if !editor_float.is_process_alive() {
            editor_float.close();
            return;
        }
        let bytes = key_to_bytes(key);
        if !bytes.is_empty() {
            let _ = editor_float.write(&bytes);
        }
        return;
    }

    if confirm_dialog.visible {
        confirm_dialog.handle_key_event(key);

        if let Some((true, action)) = confirm_dialog.take_result() {
            match action {
                ConfirmAction::DeleteWorktree => {
                    if let Some(wt) = app.worktree_pool.get(app.selected_worktree) {
                        let entry = wt.to_entry();
                        let _ = worktree_manager.remove_worktree(&entry);
                        app.worktree_pool.remove(app.selected_worktree);
                        if app.selected_worktree >= app.worktree_pool.len()
                            && app.selected_worktree > 0
                        {
                            app.selected_worktree -= 1;
                        }
                    }
                }
                ConfirmAction::QuitApp => {
                    app.quit();
                }
            }
        }
        return;
    }

    if repo_selector.visible {
        repo_selector.handle_key_event(key);

        if let Some(result) = repo_selector.take_result() {
            match worktree_manager.add_worktree(
                &result.repo,
                &result.branch,
                result.base_branch.as_deref(),
            ) {
                Ok(entry) => {
                    let mut wt = domain::worktree::Worktree::from_entry(&entry);
                    let size = crossterm::terminal::size().unwrap_or((80, 24));
                    let _ = wt.start(size.1, size.0);
                    app.worktree_pool.add(wt);
                    app.selected_worktree = app.worktree_pool.len().saturating_sub(1);
                    app.focus = Focus::Terminal;
                }
                Err(e) => {
                    let _ = std::fs::write("/tmp/ccargus-debug.log", format!("{e}"));
                }
            }
        }
    } else if qa_selector.visible {
        qa_selector.handle_key_event(key);

        if let Some(mode) = qa_selector.take_result() {
            let size = crossterm::terminal::size().unwrap_or((80, 24));
            let fork = mode == QaMode::Fork;
            if let Some(wt) = app.worktree_pool.get_mut(app.selected_worktree) {
                let _ = wt.create_qa(fork, size.1, size.0);
            }
            app.focus = Focus::QaTerminal;
        }
    } else {
        match app.focus {
            Focus::Worktrees => {
                handle_worktrees_key(
                    app,
                    confirm_dialog,
                    editor_float,
                    config,
                    repo_selector,
                    qa_selector,
                    key,
                );
            }
            Focus::Terminal => handle_terminal_key(app, terminal_pane, key),
            Focus::QaTerminal => handle_qa_terminal_key(app, terminal_pane, key),
        }
    }
}

fn update_components(
    app: &app::App,
    worktree_tree: &mut WorktreeTree,
    terminal_pane: &mut TerminalPane,
) {
    worktree_tree.selected = app.selected_worktree;
    worktree_tree.focused = app.focus == Focus::Worktrees;
    worktree_tree.worktrees = app
        .worktree_pool
        .all()
        .iter()
        .map(|wt| WorktreeItem {
            branch: wt.branch.clone(),
            repo: wt.display_name().to_string(),
            running: wt.is_running(),
        })
        .collect();
    terminal_pane.focused = app.focus == Focus::Terminal;
    terminal_pane.qa_focused = app.focus == Focus::QaTerminal;
    let new_screen = app
        .worktree_pool
        .get(app.selected_worktree)
        .and_then(|wt| wt.pty.as_ref().map(domain::pty::PtySession::screen));
    let new_qa_screen = app
        .worktree_pool
        .get(app.selected_worktree)
        .and_then(|wt| wt.qa_pty.as_ref().map(domain::pty::PtySession::screen));

    // Reset scroll offset when the screen changes (e.g. worktree switch)
    if terminal_pane.screen.is_some() != new_screen.is_some() || new_screen.is_none() {
        terminal_pane.scroll_offset = 0;
    }
    if terminal_pane.qa_screen.is_some() != new_qa_screen.is_some() || new_qa_screen.is_none() {
        terminal_pane.qa_scroll_offset = 0;
    }

    terminal_pane.screen = new_screen;
    terminal_pane.qa_screen = new_qa_screen;
}

fn build_status_line(app: &app::App) -> StatusLine {
    app.worktree_pool.get(app.selected_worktree).map_or_else(
        || StatusLine {
            branch: String::new(),
            dir: String::new(),
            qa_mode: None,
            repo: String::new(),
            status: "no worktree".to_owned(),
        },
        |wt| {
            let qa_mode = if wt.has_qa() {
                Some("active".to_owned())
            } else {
                None
            };
            let status = if wt.is_running() {
                "running"
            } else {
                "stopped"
            };
            StatusLine {
                branch: wt.branch.clone(),
                dir: wt.working_dir(),
                qa_mode,
                repo: wt.display_name().to_string(),
                status: status.to_owned(),
            }
        },
    )
}

fn handle_mouse_event(
    app: &mut app::App,
    editor_float: &mut EditorFloat,
    terminal_pane: &mut TerminalPane,
    mouse: crossterm::event::MouseEvent,
) {
    use crossterm::event::MouseEventKind;

    let is_scroll_wheel = matches!(
        mouse.kind,
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
    );

    if is_scroll_wheel
        && !editor_float.visible
        && matches!(app.focus, Focus::Terminal | Focus::QaTerminal)
    {
        let is_qa = app.focus == Focus::QaTerminal;
        let max = scrollback_max(app, is_qa);
        match mouse.kind {
            MouseEventKind::ScrollUp => terminal_pane.scroll_up(is_qa, 3, max),
            MouseEventKind::ScrollDown => terminal_pane.scroll_down(is_qa, 3),
            _ => {}
        }
        return;
    }

    let bytes = mouse_to_bytes(mouse);
    if bytes.is_empty() {
        return;
    }

    if editor_float.visible {
        let _ = editor_float.write(&bytes);
        return;
    }

    let pty = app
        .worktree_pool
        .get_mut(app.selected_worktree)
        .and_then(|wt| match app.focus {
            Focus::Terminal => wt.pty.as_mut(),
            Focus::QaTerminal => wt.qa_pty.as_mut(),
            Focus::Worktrees => None,
        });
    if let Some(pty) = pty {
        let _ = pty.write(&bytes);
    }
}

fn scrollback_max(app: &app::App, qa: bool) -> usize {
    app.worktree_pool
        .get(app.selected_worktree)
        .and_then(|wt| {
            let pty = if qa {
                wt.qa_pty.as_ref()
            } else {
                wt.pty.as_ref()
            };
            pty.and_then(|p| {
                p.screen().lock().ok().map(|mut parser| {
                    let screen = parser.screen_mut();
                    // set_scrollback clamps to the actual scrollback buffer size
                    screen.set_scrollback(usize::MAX);
                    let max = screen.scrollback();
                    screen.set_scrollback(0);
                    max
                })
            })
        })
        .unwrap_or(0)
}

fn terminal_half_page_size() -> usize {
    let (_cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    usize::from(rows) / 2
}

/// Returns true if the key was handled as a scroll action.
fn handle_scroll_key(
    app: &app::App,
    terminal_pane: &mut TerminalPane,
    key: crossterm::event::KeyEvent,
    qa: bool,
) -> bool {
    // Ctrl+b: enter/continue scroll mode (half page up)
    if key.code == KeyCode::Char('b') && key.modifiers.contains(KeyModifiers::CONTROL) {
        let max = scrollback_max(app, qa);
        terminal_pane.scroll_up(qa, terminal_half_page_size(), max);
        return true;
    }

    // Ctrl+f: half page down
    if key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL) {
        terminal_pane.scroll_down(qa, terminal_half_page_size());
        return true;
    }

    if !terminal_pane.is_scrolling(qa) {
        return false;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            let max = scrollback_max(app, qa);
            terminal_pane.scroll_up(qa, 1, max);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            terminal_pane.scroll_down(qa, 1);
        }
        KeyCode::PageUp => {
            let max = scrollback_max(app, qa);
            terminal_pane.scroll_up(qa, terminal_half_page_size() * 2, max);
        }
        KeyCode::PageDown => {
            terminal_pane.scroll_down(qa, terminal_half_page_size() * 2);
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            terminal_pane.exit_scroll(qa);
        }
        _ => {
            terminal_pane.exit_scroll(qa);
            return false;
        }
    }
    true
}

fn handle_qa_terminal_key(
    app: &mut app::App,
    terminal_pane: &mut TerminalPane,
    key: crossterm::event::KeyEvent,
) {
    if handle_scroll_key(app, terminal_pane, key, true) {
        return;
    }

    if key.code == KeyCode::Tab {
        let has_qa = app
            .worktree_pool
            .get(app.selected_worktree)
            .is_some_and(domain::worktree::Worktree::has_qa);
        app.toggle_focus(has_qa);
        return;
    }

    // Ctrl+w toggles between Terminal and QaTerminal
    if key.code == KeyCode::Char('w') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.toggle_terminal_qa_focus();
        return;
    }

    // Ctrl+d closes Q&A
    if key.code == KeyCode::Char('d') && key.modifiers.contains(KeyModifiers::CONTROL) {
        if let Some(wt) = app.worktree_pool.get_mut(app.selected_worktree) {
            wt.close_qa();
        }
        app.focus = Focus::Terminal;
        return;
    }

    let bytes = key_to_bytes(key);
    if !bytes.is_empty()
        && let Some(wt) = app.worktree_pool.get_mut(app.selected_worktree)
        && let Some(qa) = &mut wt.qa_pty
    {
        let _ = qa.write(&bytes);
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_worktrees_key(
    app: &mut app::App,
    confirm_dialog: &mut ConfirmDialog,
    editor_float: &mut EditorFloat,
    config: &Config,
    repo_selector: &mut RepoSelector,
    qa_selector: &mut QaSelector,
    key: crossterm::event::KeyEvent,
) {
    match key.code {
        KeyCode::Char('q' | 'c')
            if key.code == KeyCode::Char('q') || key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            confirm_dialog.open("Quit ccargus?", ConfirmAction::QuitApp);
        }
        KeyCode::Char('d') => {
            if let Some(wt) = app.worktree_pool.get(app.selected_worktree) {
                let message = format!("Delete worktree '{}/{}'?", wt.repo, wt.branch);
                confirm_dialog.open(message, ConfirmAction::DeleteWorktree);
            }
        }
        KeyCode::Char('e') => {
            if let Some(wt) = app.worktree_pool.get(app.selected_worktree) {
                let size = crossterm::terminal::size().unwrap_or((80, 24));
                let _ =
                    editor_float.open(&config.editor.command, &wt.working_dir(), size.1, size.0);
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.select_next_worktree(app.worktree_pool.len());
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.select_prev_worktree(app.worktree_pool.len());
        }
        KeyCode::Char('n') => {
            repo_selector.open();
        }
        KeyCode::Char('s') => {
            if let Some(wt) = app.worktree_pool.get(app.selected_worktree)
                && wt.is_running()
            {
                qa_selector.open();
            }
        }
        KeyCode::Char('x') => {
            // Stop PTY without removing worktree
            if let Some(wt) = app.worktree_pool.get_mut(app.selected_worktree) {
                wt.stop();
            }
        }
        KeyCode::Enter => {
            if let Some(wt) = app.worktree_pool.get_mut(app.selected_worktree) {
                if wt.is_running() {
                    // Focus into running worktree
                    let has_qa = wt.has_qa();
                    app.toggle_focus(has_qa);
                } else {
                    // Start stopped worktree
                    let size = crossterm::terminal::size().unwrap_or((80, 24));
                    let _ = wt.start(size.1, size.0);
                    app.focus = Focus::Terminal;
                }
            }
        }
        KeyCode::Tab => {
            if !app.worktree_pool.is_empty() {
                let has_qa = app
                    .worktree_pool
                    .get(app.selected_worktree)
                    .is_some_and(domain::worktree::Worktree::has_qa);
                app.toggle_focus(has_qa);
            }
        }
        _ => {}
    }
}

fn handle_terminal_key(
    app: &mut app::App,
    terminal_pane: &mut TerminalPane,
    key: crossterm::event::KeyEvent,
) {
    if handle_scroll_key(app, terminal_pane, key, false) {
        return;
    }

    if key.code == KeyCode::Tab {
        let has_qa = app
            .worktree_pool
            .get(app.selected_worktree)
            .is_some_and(domain::worktree::Worktree::has_qa);
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
        && let Some(wt) = app.worktree_pool.get_mut(app.selected_worktree)
        && let Some(pty) = &mut wt.pty
    {
        let _ = pty.write(&bytes);
    }
}
