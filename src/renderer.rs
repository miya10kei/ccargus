use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::Focus;
use crate::components::Component;
use crate::components::status_line::StatusLine;
use crate::components::worktree_tree::WorktreeItem;
use crate::context::{AppContext, UiContext};
use crate::domain;

pub fn build_status_line(ctx: &AppContext, ui: &UiContext) -> StatusLine {
    let is_qa = ctx.app.focus == Focus::QaTerminal;
    let copy_hint = if ui.terminal_pane.is_in_copy_mode(is_qa) {
        Some("COPY: v=select y=yank q=exit".to_owned())
    } else {
        None
    };

    let notification =
        ctx.active_notification()
            .map(|n| crate::components::status_line::StatusNotification {
                level: n.level,
                message: n.message.clone(),
            });

    ctx.app
        .worktree_pool
        .get(ctx.app.selected_worktree)
        .map_or_else(
            || StatusLine {
                branch: String::new(),
                copy_hint: None,
                dir: String::new(),
                notification: notification.clone(),
                qa_mode: None,
                repo: String::new(),
                status: "no worktree".to_owned(),
            },
            |wt| {
                let qa_mode = wt.has_qa().then(|| "active".to_owned());
                let claude_status = ctx
                    .status_cache
                    .read_status(&wt.working_dir(), wt.is_running());
                StatusLine {
                    branch: wt.branch.clone(),
                    copy_hint,
                    dir: wt.working_dir(),
                    notification: notification.clone(),
                    qa_mode,
                    repo: wt.display_name().to_string(),
                    status: claude_status.label().to_owned(),
                }
            },
        )
}

pub fn update_components(ctx: &AppContext, ui: &mut UiContext) {
    ui.worktree_tree.selected = ctx.app.selected_worktree;
    ui.worktree_tree.focused = ctx.app.focus == Focus::Worktrees;
    ui.worktree_tree.worktrees = ctx
        .app
        .worktree_pool
        .all()
        .iter()
        .map(|wt| WorktreeItem {
            branch: wt.branch.clone(),
            repo: wt.display_name().to_string(),
            status: ctx
                .status_cache
                .read_status(&wt.working_dir(), wt.is_running()),
        })
        .collect();
    ui.terminal_pane.focused = ctx.app.focus == Focus::Terminal;
    ui.terminal_pane.qa_focused = ctx.app.focus == Focus::QaTerminal;
    let new_screen = ctx
        .app
        .worktree_pool
        .get(ctx.app.selected_worktree)
        .and_then(|wt| wt.pty.as_ref().map(domain::pty::PtySession::screen));
    let new_qa_screen = ctx
        .app
        .worktree_pool
        .get(ctx.app.selected_worktree)
        .and_then(|wt| wt.qa_pty.as_ref().map(domain::pty::PtySession::screen));

    // Reset scroll/copy mode when the screen changes (e.g. worktree switch)
    if ui.terminal_pane.screen.is_some() != new_screen.is_some() || new_screen.is_none() {
        ui.terminal_pane.scroll_offset = 0;
        ui.terminal_pane.copy_mode = None;
    }
    if ui.terminal_pane.qa_screen.is_some() != new_qa_screen.is_some() || new_qa_screen.is_none() {
        ui.terminal_pane.qa_scroll_offset = 0;
        ui.terminal_pane.qa_copy_mode = None;
    }

    ui.terminal_pane.screen = new_screen;
    ui.terminal_pane.qa_screen = new_qa_screen;
}

pub fn render(frame: &mut Frame, ui: &mut UiContext, ctx: &AppContext) {
    update_components(ctx, ui);
    let status_line = build_status_line(ctx, ui);

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(frame.area());

    let wt_percent = ctx.config.layout.worktree_pane_percent;
    let terminal_percent = 100u16.saturating_sub(wt_percent);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(wt_percent),
            Constraint::Percentage(terminal_percent),
        ])
        .split(vertical[0]);

    ui.last_worktree_area = Some(horizontal[0]);
    ui.last_terminal_area = Some(horizontal[1]);

    ui.worktree_tree.render(frame, horizontal[0]);
    ui.terminal_pane.render(frame, horizontal[1]);
    status_line.render(frame, vertical[1]);

    ui.repo_selector.render(frame, frame.area());
    ui.qa_selector.render(frame, frame.area());
    ui.confirm_dialog.render(frame, frame.area());
    ui.editor_float.render(frame, frame.area());
    ui.help_overlay.render(frame, frame.area());

    // Show cursor at IME position when no overlay is active
    if !ui.editor_float.visible
        && !ui.repo_selector.visible
        && !ui.qa_selector.visible
        && !ui.confirm_dialog.visible
        && !ui.help_overlay.visible
        && let Some((x, y)) = ui.terminal_pane.cursor_position_for_ime(horizontal[1])
    {
        frame.set_cursor_position(ratatui::layout::Position::new(x, y));
    }
}
