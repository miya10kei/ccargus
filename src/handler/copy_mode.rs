use crossterm::event::{KeyCode, KeyModifiers};

use crate::components::terminal_pane::TerminalPane;
use crate::context::AppContext;
use crate::copy_mode::{CopyModeState, ScrollDirection};
use crate::handler::scroll::scrollback_max;
use crate::layout::terminal_half_page_size;

/// Returns true if the key was handled as a copy mode action.
#[allow(clippy::too_many_lines)]
pub fn handle_copy_mode_key(
    ctx: &mut AppContext,
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
                let max = scrollback_max(&ctx.worktree_pool, ctx.app.selected_worktree, qa);
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
            let max = scrollback_max(&ctx.worktree_pool, ctx.app.selected_worktree, qa);
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
                ctx.notify(
                    format!("Copied {} chars", text.len()),
                    crate::context::NotificationLevel::Info,
                );
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
