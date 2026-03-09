use color_eyre::Result;
use crossterm::event::KeyEventKind;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::components::Component;
use crate::context::{AppContext, UiContext};
use crate::domain::claude_status::start_socket_listener;
use crate::layout::calculate_pty_sizes;

mod action;
mod app;
mod components;
mod config;
mod context;
mod copy_mode;
mod domain;
mod event;
mod handler;
mod keys;
mod layout;
mod renderer;
mod tui;

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let mut tui = tui::Tui::new()?;

    let mut app = app::App::new();
    let config = config::Config::load()?;
    let worktree_manager = domain::WorktreeManager::new(config.worktree.base_dir.clone())?;
    let status_cache = domain::claude_status::StatusCache::new();

    let status_rx = start_socket_listener(status_cache.socket_path());
    let mut events = event::EventHandler::new(4.0, 60.0, status_rx);

    let entries = worktree_manager.scan()?;
    app.worktree_pool.sync_with_worktrees(&entries);

    let mut ctx = AppContext {
        app,
        config,
        status_cache,
        worktree_manager,
    };

    let mut ui = UiContext {
        confirm_dialog: components::confirm_dialog::ConfirmDialog::new(),
        editor_float: components::editor_float::EditorFloat::new(),
        qa_selector: components::qa_selector::QaSelector::new(),
        repo_selector: components::repo_selector::RepoSelector::new(),
        terminal_pane: components::terminal_pane::TerminalPane::new(),
        worktree_tree: components::worktree_tree::WorktreeTree::new(),
    };

    let mut needs_render = true;

    while ctx.app.is_running() {
        let event = events.next().await?;
        match event {
            event::Event::Key(key) if key.kind == KeyEventKind::Press => {
                handler::handle_key_press(&mut ctx, &mut ui, key);
                needs_render = true;
            }
            event::Event::Mouse(mouse) => {
                handler::mouse::handle_mouse_event(&mut ctx, &mut ui, mouse);
                needs_render = true;
            }
            event::Event::Resize(cols, rows) => {
                let sizes = calculate_pty_sizes(cols, rows);
                for wt in ctx.app.worktree_pool.all_mut() {
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
                ctx.status_cache.update(&cwd, &status);
            }
            event::Event::Render => {
                let pty_dirty = ctx
                    .app
                    .worktree_pool
                    .get(ctx.app.selected_worktree)
                    .is_some_and(domain::worktree::Worktree::any_pty_dirty);
                let editor_dirty = ui.editor_float.visible && ui.editor_float.is_dirty();

                if !needs_render && !pty_dirty && !editor_dirty {
                    continue;
                }

                if let Some(wt) = ctx.app.worktree_pool.get(ctx.app.selected_worktree) {
                    wt.clear_pty_dirty();
                }
                ui.editor_float.clear_dirty();
                needs_render = false;

                renderer::update_components(&ctx, &mut ui);
                let status_line = renderer::build_status_line(&ctx, &ui);

                tui.draw(|frame| {
                    let vertical = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(3), Constraint::Length(1)])
                        .split(frame.area());

                    let horizontal = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
                        .split(vertical[0]);

                    ui.worktree_tree.render(frame, horizontal[0]);
                    ui.terminal_pane.render(frame, horizontal[1]);
                    status_line.render(frame, vertical[1]);

                    ui.repo_selector.render(frame, frame.area());
                    ui.qa_selector.render(frame, frame.area());
                    ui.confirm_dialog.render(frame, frame.area());
                    ui.editor_float.render(frame, frame.area());

                    // Show cursor at IME position when no overlay is active
                    if !ui.editor_float.visible
                        && !ui.repo_selector.visible
                        && !ui.qa_selector.visible
                        && !ui.confirm_dialog.visible
                        && let Some((x, y)) =
                            ui.terminal_pane.cursor_position_for_ime(horizontal[1])
                    {
                        frame.set_cursor_position(ratatui::layout::Position::new(x, y));
                    }
                })?;
            }
            _ => {}
        }
    }

    tui.exit()?;

    let _ = std::fs::remove_file(ctx.status_cache.socket_path());

    Ok(())
}