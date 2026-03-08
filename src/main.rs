use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders};

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
use crate::copy_mode::{CopyModeState, ScrollDirection};
use crate::domain::claude_status::{StatusCache, start_socket_listener};
use crate::domain::worktree::WorktreeManager;
use crate::keys::{key_to_bytes, mouse_to_bytes};

mod action;
mod app;
mod components;
mod config;
mod copy_mode;
mod domain;
mod event;
mod keys;
mod tui;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let mut tui = tui::Tui::new()?;
    let mut status_cache = StatusCache::new();

    let status_rx = start_socket_listener(status_cache.socket_path());
    let mut events = event::EventHandler::new(4.0, 60.0, status_rx);

    let mut app = app::App::new();
    let config = Config::load()?;
    let worktree_manager = WorktreeManager::new(config.worktree.base_dir.clone())?;

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
                    &mut status_cache,
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
            event::Event::Resize(cols, rows) => {
                let sizes = calculate_pty_sizes(cols, rows);
                for wt in app.worktree_pool.all_mut() {
                    let (main_rows, main_cols) = sizes.main_size(wt.has_qa());
                    wt.resize_pty(
                        main_rows,
                        main_cols,
                        sizes.split_qa_rows,
                        sizes.split_qa_cols,
                    );
                }
                needs_render = true;
            }
            event::Event::StatusChanged { cwd, status } => {
                status_cache.update(&cwd, &status);
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

                update_components(&app, &status_cache, &mut worktree_tree, &mut terminal_pane);
                let status_line = build_status_line(&app, &status_cache, &terminal_pane);

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
            _ => {}
        }
    }

    tui.exit()?;

    let _ = std::fs::remove_file(status_cache.socket_path());

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_key_press(
    app: &mut app::App,
    confirm_dialog: &mut ConfirmDialog,
    editor_float: &mut EditorFloat,
    config: &Config,
    status_cache: &mut StatusCache,
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
                        status_cache.cleanup(&wt.working_dir());
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
                    let sizes = current_pty_sizes();
                    let _ = wt.start(sizes.single_rows, sizes.single_cols);
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
            let sizes = current_pty_sizes();
            let fork = mode == QaMode::Fork;
            if let Some(wt) = app.worktree_pool.get_mut(app.selected_worktree) {
                let _ = wt.create_qa(fork, sizes.split_qa_rows, sizes.split_qa_cols);
                let (main_rows, main_cols) = sizes.main_size(true);
                wt.resize_pty(
                    main_rows,
                    main_cols,
                    sizes.split_qa_rows,
                    sizes.split_qa_cols,
                );
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
                    status_cache,
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
    status_cache: &StatusCache,
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
            status: status_cache.read_status(&wt.working_dir(), wt.is_running()),
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

    // Reset scroll/copy mode when the screen changes (e.g. worktree switch)
    if terminal_pane.screen.is_some() != new_screen.is_some() || new_screen.is_none() {
        terminal_pane.scroll_offset = 0;
        terminal_pane.copy_mode = None;
    }
    if terminal_pane.qa_screen.is_some() != new_qa_screen.is_some() || new_qa_screen.is_none() {
        terminal_pane.qa_scroll_offset = 0;
        terminal_pane.qa_copy_mode = None;
    }

    terminal_pane.screen = new_screen;
    terminal_pane.qa_screen = new_qa_screen;
}

fn build_status_line(
    app: &app::App,
    status_cache: &StatusCache,
    terminal_pane: &TerminalPane,
) -> StatusLine {
    let is_qa = app.focus == Focus::QaTerminal;
    let copy_hint = if terminal_pane.is_in_copy_mode(is_qa) {
        Some("COPY: v=select y=yank q=exit".to_owned())
    } else {
        None
    };

    app.worktree_pool.get(app.selected_worktree).map_or_else(
        || StatusLine {
            branch: String::new(),
            copy_hint: None,
            dir: String::new(),
            qa_mode: None,
            repo: String::new(),
            status: "no worktree".to_owned(),
        },
        |wt| {
            let qa_mode = wt.has_qa().then(|| "active".to_owned());
            let claude_status = status_cache.read_status(&wt.working_dir(), wt.is_running());
            StatusLine {
                branch: wt.branch.clone(),
                copy_hint,
                dir: wt.working_dir(),
                qa_mode,
                repo: wt.display_name().to_string(),
                status: claude_status.label().to_owned(),
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
        KeyCode::Char('v') => {
            let (_cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
            // Approximate viewport: terminal height minus borders and status line
            let viewport_rows = usize::from(rows).saturating_sub(4);
            let viewport_cols = 80; // Will be refined by actual render area
            terminal_pane.enter_copy_mode(qa, viewport_rows, viewport_cols);
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

/// Returns true if the key was handled as a copy mode action.
#[allow(clippy::too_many_lines)]
fn handle_copy_mode_key(
    app: &app::App,
    terminal_pane: &mut TerminalPane,
    key: crossterm::event::KeyEvent,
    qa: bool,
) -> bool {
    if !terminal_pane.is_in_copy_mode(qa) {
        return false;
    }

    // Clone the screen Arc to avoid borrow conflicts with terminal_pane
    let screen_arc = if qa {
        terminal_pane.qa_screen.clone()
    } else {
        terminal_pane.screen.clone()
    };

    match key.code {
        KeyCode::Char('h') | KeyCode::Left => {
            if let Some(cm) = terminal_pane.copy_mode_mut(qa) {
                cm.move_left();
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let scroll_dir = terminal_pane
                .copy_mode_mut(qa)
                .and_then(CopyModeState::move_down);
            if let Some(ScrollDirection::Down) = scroll_dir {
                terminal_pane.scroll_down(qa, 1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let scroll_dir = terminal_pane
                .copy_mode_mut(qa)
                .and_then(CopyModeState::move_up);
            if let Some(ScrollDirection::Up) = scroll_dir {
                let max = scrollback_max(app, qa);
                terminal_pane.scroll_up(qa, 1, max);
            }
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if let Some(cm) = terminal_pane.copy_mode_mut(qa) {
                cm.move_right();
            }
        }
        KeyCode::Char('w') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let scroll_offset = terminal_pane.scroll_offset_for(qa);
            if let Some(parser_arc) = &screen_arc
                && let Ok(parser) = parser_arc.lock()
                && let Some(cm) = terminal_pane.copy_mode_mut(qa)
            {
                cm.move_word_forward(parser.screen(), scroll_offset);
            }
        }
        KeyCode::Char('b') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let scroll_offset = terminal_pane.scroll_offset_for(qa);
            if let Some(parser_arc) = &screen_arc
                && let Ok(parser) = parser_arc.lock()
                && let Some(cm) = terminal_pane.copy_mode_mut(qa)
            {
                cm.move_word_backward(parser.screen(), scroll_offset);
            }
        }
        KeyCode::Char('^') => {
            if let Some(cm) = terminal_pane.copy_mode_mut(qa) {
                cm.move_line_start();
            }
        }
        KeyCode::Char('$') => {
            if let Some(cm) = terminal_pane.copy_mode_mut(qa) {
                cm.move_line_end();
            }
        }
        KeyCode::Char('g') => {
            if let Some(cm) = terminal_pane.copy_mode_mut(qa) {
                cm.move_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(cm) = terminal_pane.copy_mode_mut(qa) {
                cm.move_bottom();
            }
        }
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let max = scrollback_max(app, qa);
            terminal_pane.scroll_up(qa, terminal_half_page_size(), max);
        }
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            terminal_pane.scroll_down(qa, terminal_half_page_size());
        }
        KeyCode::Char('v' | ' ') => {
            if let Some(cm) = terminal_pane.copy_mode_mut(qa) {
                cm.toggle_selection();
            }
        }
        KeyCode::Char('y') | KeyCode::Enter => {
            let scroll_offset = terminal_pane.scroll_offset_for(qa);
            let text = screen_arc.as_ref().and_then(|parser_arc| {
                parser_arc.lock().ok().and_then(|parser| {
                    terminal_pane
                        .copy_mode_for(qa)
                        .map(|cm| cm.extract_text(parser.screen(), scroll_offset))
                })
            });
            if let Some(text) = text
                && !text.is_empty()
            {
                let _ = CopyModeState::copy_to_clipboard(&text);
            }
            terminal_pane.exit_copy_mode(qa);
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            terminal_pane.exit_copy_mode(qa);
        }
        _ => {}
    }
    true
}

fn handle_qa_terminal_key(
    app: &mut app::App,
    terminal_pane: &mut TerminalPane,
    key: crossterm::event::KeyEvent,
) {
    if handle_copy_mode_key(app, terminal_pane, key, true) {
        return;
    }

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
            let sizes = current_pty_sizes();
            let (main_rows, main_cols) = sizes.main_size(false);
            wt.resize_pty(
                main_rows,
                main_cols,
                sizes.split_qa_rows,
                sizes.split_qa_cols,
            );
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
    status_cache: &mut StatusCache,
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
            if let Some(wt) = app.worktree_pool.get_mut(app.selected_worktree) {
                status_cache.cleanup(&wt.working_dir());
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
                    let sizes = current_pty_sizes();
                    let _ = wt.start(sizes.single_rows, sizes.single_cols);
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

struct PtySizes {
    /// Single pane dimensions (no Q&A active)
    single_cols: u16,
    single_rows: u16,
    /// Split pane dimensions for main terminal (Q&A active)
    split_main_cols: u16,
    split_main_rows: u16,
    /// Split pane dimensions for Q&A terminal
    split_qa_cols: u16,
    split_qa_rows: u16,
}

impl PtySizes {
    fn main_size(&self, has_qa: bool) -> (u16, u16) {
        if has_qa {
            (self.split_main_rows, self.split_main_cols)
        } else {
            (self.single_rows, self.single_cols)
        }
    }
}

fn current_pty_sizes() -> PtySizes {
    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    calculate_pty_sizes(cols, rows)
}

fn calculate_pty_sizes(term_cols: u16, term_rows: u16) -> PtySizes {
    let full = Rect::new(0, 0, term_cols, term_rows);

    // Vertical: content area + status line
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(full);

    // Horizontal: worktree tree (25%) + terminal pane (75%)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(vertical[0]);

    let terminal_area = horizontal[1];

    // Single pane (no Q&A): terminal area with border
    let single_inner = Block::default().borders(Borders::ALL).inner(terminal_area);

    // Split pane (Q&A): 50/50 horizontal split, each with border
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(terminal_area);
    let split_main_inner = Block::default().borders(Borders::ALL).inner(split[0]);
    let split_qa_inner = Block::default().borders(Borders::ALL).inner(split[1]);

    PtySizes {
        single_cols: single_inner.width,
        single_rows: single_inner.height,
        split_main_cols: split_main_inner.width,
        split_main_rows: split_main_inner.height,
        split_qa_cols: split_qa_inner.width,
        split_qa_rows: split_qa_inner.height,
    }
}

fn handle_terminal_key(
    app: &mut app::App,
    terminal_pane: &mut TerminalPane,
    key: crossterm::event::KeyEvent,
) {
    if handle_copy_mode_key(app, terminal_pane, key, false) {
        return;
    }

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
