use color_eyre::Result;
use crossterm::event::KeyEventKind;

use crate::context::{AppContext, UiContext};
use crate::domain::claude_status::start_socket_listener;
use crate::layout::calculate_pty_sizes;

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
async fn main() -> Result<()> {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("ccargus {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    color_eyre::install()?;

    let mut tui = tui::Tui::new()?;

    let app = app::App::new();
    let config = config::Config::load()?;
    config.validate()?;
    let worktree_manager = domain::WorktreeManager::new(
        config.worktree.base_dir.clone(),
        config.worktree.protected_branches.clone(),
    )?;
    let status_cache = domain::claude_status::StatusCache::new();

    let status_rx = start_socket_listener(status_cache.socket_path());
    let mut events = event::EventHandler::new(4.0, 60.0, status_rx);

    let mut worktree_pool = domain::worktree::WorktreePool::new();
    let entries = worktree_manager.scan()?;
    worktree_pool.sync_with_worktrees(&entries);

    let qa_split_percent = config.layout.qa_split_percent;

    let mut ctx = AppContext {
        app,
        config,
        notification: None,
        status_cache,
        worktree_manager,
        worktree_pool,
    };

    let mut ui = UiContext {
        confirm_dialog: components::confirm_dialog::ConfirmDialog::new(),
        editor_float: components::editor_float::EditorFloat::new(),
        help_overlay: components::help_overlay::HelpOverlay::new(),
        last_worktree_area: None,
        last_terminal_area: None,
        qa_selector: components::qa_selector::QaSelector::new(),
        repo_selector: components::repo_selector::RepoSelector::new(),
        terminal_pane: {
            let mut tp = components::terminal_pane::TerminalPane::new();
            tp.qa_split_percent = qa_split_percent;
            tp
        },
        worktree_tree: components::worktree_tree::WorktreeTree::new(),
    };

    let mut needs_render = true;

    while ctx.app.is_running() {
        let event = events.next().await?;
        needs_render = handle_event(event, &mut ctx, &mut ui, &mut tui, needs_render)?;
    }

    tui.exit()?;

    let _ = std::fs::remove_file(ctx.status_cache.socket_path());

    Ok(())
}

fn handle_event(
    event: event::Event,
    ctx: &mut AppContext,
    ui: &mut UiContext,
    tui: &mut tui::Tui,
    mut needs_render: bool,
) -> Result<bool> {
    match event {
        event::Event::Key(key) if key.kind == KeyEventKind::Press => {
            handler::handle_key_press(ctx, ui, key);
            needs_render = true;
        }
        event::Event::Mouse(mouse) => {
            handler::mouse::handle_mouse_event(ctx, ui, mouse);
            needs_render = true;
        }
        event::Event::Resize(cols, rows) => {
            let sizes = calculate_pty_sizes(
                cols,
                rows,
                ctx.config.layout.worktree_pane_percent,
                ctx.config.layout.qa_split_percent,
            );
            for wt in ctx.worktree_pool.all_mut() {
                let (main_rows, main_cols) = sizes.main_size(wt.has_qa());
                wt.resize_pty(
                    main_rows,
                    main_cols,
                    sizes.split_qa_rows,
                    sizes.split_qa_cols,
                );
            }
            ui.editor_float.resize(rows, cols);
            needs_render = true;
        }
        event::Event::StatusChanged { cwd, status } => {
            ctx.status_cache.update(&cwd, &status);
        }
        event::Event::Render => {
            let pty_dirty = ctx
                .worktree_pool
                .get(ctx.app.selected_worktree)
                .is_some_and(domain::worktree::Worktree::any_pty_dirty);
            let editor_dirty = ui.editor_float.visible && ui.editor_float.is_dirty();

            if needs_render || pty_dirty || editor_dirty {
                if let Some(wt) = ctx.worktree_pool.get(ctx.app.selected_worktree) {
                    wt.clear_pty_dirty();
                }
                ui.editor_float.clear_dirty();
                needs_render = false;

                tui.draw(|frame| {
                    renderer::render(frame, ui, ctx);
                })?;
            }
        }
        event::Event::Tick => {
            for wt in ctx.worktree_pool.all_mut() {
                if let Some(pty) = &mut wt.pty
                    && !pty.is_alive()
                {
                    ctx.status_cache.cleanup(&wt.working_dir());
                    wt.pty = None;
                    needs_render = true;
                }
                if let Some(qa) = &mut wt.qa_pty
                    && !qa.is_alive()
                {
                    wt.qa_pty = None;
                    needs_render = true;
                }
            }
        }
        _ => {}
    }
    Ok(needs_render)
}
