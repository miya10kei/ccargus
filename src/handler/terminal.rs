use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::Focus;
use crate::context::{AppContext, UiContext};
use crate::domain;
use crate::handler::copy_mode::handle_copy_mode_key;
use crate::handler::scroll::handle_scroll_key;
use crate::handler::worktrees;
use crate::keys::key_to_bytes;
use crate::layout::current_pty_sizes_with_config;

pub fn handle_qa_terminal_key(
    ctx: &mut AppContext,
    ui: &mut UiContext,
    key: crossterm::event::KeyEvent,
) {
    if handle_copy_mode_key(ctx, &mut ui.terminal_pane, key, true) {
        return;
    }

    if handle_scroll_key(
        &ctx.worktree_pool,
        ctx.app.selected_worktree,
        &mut ui.terminal_pane,
        key,
        true,
        &ctx.config.layout,
    ) {
        return;
    }

    if key.code == KeyCode::Tab {
        let has_qa = ctx
            .worktree_pool
            .get(ctx.app.selected_worktree)
            .is_some_and(domain::worktree::Worktree::has_qa);
        ctx.app.toggle_focus(has_qa);
        return;
    }

    // Ctrl+w toggles between Terminal and QaTerminal
    if key.code == KeyCode::Char('w') && key.modifiers.contains(KeyModifiers::CONTROL) {
        ctx.app.toggle_terminal_qa_focus();
        return;
    }

    if ctx.config.keybindings.terminal_open_editor.matches(&key) {
        worktrees::open_editor(ctx);
        return;
    }

    // Ctrl+d closes Q&A
    if key.code == KeyCode::Char('d') && key.modifiers.contains(KeyModifiers::CONTROL) {
        if let Some(wt) = ctx.worktree_pool.get_mut(ctx.app.selected_worktree) {
            wt.close_qa();
            let sizes = current_pty_sizes_with_config(
                ctx.config.layout.worktree_pane_percent,
                ctx.config.layout.qa_split_percent,
            );
            let (main_rows, main_cols) = sizes.main_size(false);
            wt.resize_pty(
                main_rows,
                main_cols,
                sizes.split_qa_rows,
                sizes.split_qa_cols,
            );
        }
        ctx.app.focus = Focus::Terminal;
        return;
    }

    let bytes = key_to_bytes(key);
    if !bytes.is_empty()
        && let Some(wt) = ctx.worktree_pool.get_mut(ctx.app.selected_worktree)
        && let Some(qa) = &mut wt.qa_pty
    {
        let _ = qa.write(&bytes);
    }
}

pub fn handle_terminal_key(
    ctx: &mut AppContext,
    ui: &mut UiContext,
    key: crossterm::event::KeyEvent,
) {
    if handle_copy_mode_key(ctx, &mut ui.terminal_pane, key, false) {
        return;
    }

    if handle_scroll_key(
        &ctx.worktree_pool,
        ctx.app.selected_worktree,
        &mut ui.terminal_pane,
        key,
        false,
        &ctx.config.layout,
    ) {
        return;
    }

    if key.code == KeyCode::Tab {
        let has_qa = ctx
            .worktree_pool
            .get(ctx.app.selected_worktree)
            .is_some_and(domain::worktree::Worktree::has_qa);
        ctx.app.toggle_focus(has_qa);
        return;
    }

    // Ctrl+w toggles between Terminal and QaTerminal
    if key.code == KeyCode::Char('w') && key.modifiers.contains(KeyModifiers::CONTROL) {
        ctx.app.toggle_terminal_qa_focus();
        return;
    }

    if ctx.config.keybindings.terminal_open_editor.matches(&key) {
        worktrees::open_editor(ctx);
        return;
    }

    let bytes = key_to_bytes(key);
    if !bytes.is_empty()
        && let Some(wt) = ctx.worktree_pool.get_mut(ctx.app.selected_worktree)
        && let Some(pty) = &mut wt.pty
    {
        let _ = pty.write(&bytes);
    }
}
