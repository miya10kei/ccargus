use crossterm::event::{KeyCode, KeyModifiers};

use crate::components::terminal_pane::TerminalPane;
use crate::domain::worktree::WorktreePool;
use crate::layout::terminal_half_page_size;

pub fn scrollback_max(worktree_pool: &WorktreePool, selected: usize, qa: bool) -> usize {
    worktree_pool
        .get(selected)
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

/// Returns true if the key was handled as a scroll action.
pub fn handle_scroll_key(
    worktree_pool: &WorktreePool,
    selected: usize,
    terminal_pane: &mut TerminalPane,
    key: crossterm::event::KeyEvent,
    qa: bool,
) -> bool {
    // Ctrl+b: enter/continue scroll mode (half page up)
    if key.code == KeyCode::Char('b') && key.modifiers.contains(KeyModifiers::CONTROL) {
        let max = scrollback_max(worktree_pool, selected, qa);
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
            let max = scrollback_max(worktree_pool, selected, qa);
            terminal_pane.scroll_up(qa, 1, max);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            terminal_pane.scroll_down(qa, 1);
        }
        KeyCode::PageUp => {
            let max = scrollback_max(worktree_pool, selected, qa);
            terminal_pane.scroll_up(qa, terminal_half_page_size() * 2, max);
        }
        KeyCode::PageDown => {
            terminal_pane.scroll_down(qa, terminal_half_page_size() * 2);
        }
        KeyCode::Char('v') => {
            let sizes = crate::layout::current_pty_sizes();
            let (rows, cols) = sizes.main_size(qa);
            let viewport_rows = usize::from(rows);
            let viewport_cols = usize::from(cols);
            terminal_pane.enter_copy_mode(qa, viewport_rows, viewport_cols);
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            terminal_pane.exit_scroll(qa);
        }
        _ => {}
    }
    true
}
