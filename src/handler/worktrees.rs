use crossterm::event::{KeyCode, KeyModifiers};

use crate::components::confirm_dialog::ConfirmAction;
use crate::context::{AppContext, UiContext};
use crate::domain;
use crate::layout::current_pty_sizes;

pub fn handle_worktrees_key(
    ctx: &mut AppContext,
    ui: &mut UiContext,
    key: crossterm::event::KeyEvent,
) {
    match key.code {
        KeyCode::Char('q' | 'c')
            if key.code == KeyCode::Char('q') || key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            ui.confirm_dialog
                .open("Quit ccargus?", ConfirmAction::QuitApp);
        }
        KeyCode::Char('d') => {
            if let Some(wt) = ctx.app.worktree_pool.get(ctx.app.selected_worktree) {
                let message = format!("Delete worktree '{}/{}'?", wt.repo, wt.branch);
                ui.confirm_dialog
                    .open(message, ConfirmAction::DeleteWorktree);
            }
        }
        KeyCode::Char('e') => {
            if let Some(wt) = ctx.app.worktree_pool.get(ctx.app.selected_worktree) {
                let size = crossterm::terminal::size().unwrap_or((80, 24));
                let _ = ui.editor_float.open(
                    &ctx.config.editor.command,
                    &wt.working_dir(),
                    size.1,
                    size.0,
                );
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            ctx.app.select_next_worktree(ctx.app.worktree_pool.len());
        }
        KeyCode::Char('k') | KeyCode::Up => {
            ctx.app.select_prev_worktree(ctx.app.worktree_pool.len());
        }
        KeyCode::Char('n') => {
            ui.repo_selector.open();
        }
        KeyCode::Char('s') => {
            if let Some(wt) = ctx.app.worktree_pool.get(ctx.app.selected_worktree)
                && wt.is_running()
            {
                ui.qa_selector.open();
            }
        }
        KeyCode::Char('x') => {
            if let Some(wt) = ctx.app.worktree_pool.get_mut(ctx.app.selected_worktree) {
                ctx.status_cache.cleanup(&wt.working_dir());
                wt.stop();
            }
        }
        KeyCode::Enter => {
            if let Some(wt) = ctx.app.worktree_pool.get_mut(ctx.app.selected_worktree) {
                if wt.is_running() {
                    // Focus into running worktree
                    let has_qa = wt.has_qa();
                    ctx.app.toggle_focus(has_qa);
                } else {
                    // Start stopped worktree
                    let sizes = current_pty_sizes();
                    let _ = wt.start(sizes.single_rows, sizes.single_cols, ctx.config.claude.plan);
                    ctx.app.focus = crate::app::Focus::Terminal;
                }
            }
        }
        KeyCode::Tab => {
            if !ctx.app.worktree_pool.is_empty() {
                let has_qa = ctx
                    .app
                    .worktree_pool
                    .get(ctx.app.selected_worktree)
                    .is_some_and(domain::worktree::Worktree::has_qa);
                ctx.app.toggle_focus(has_qa);
            }
        }
        _ => {}
    }
}
