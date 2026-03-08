pub mod confirm_dialog;
pub mod editor_float;
pub mod qa_selector;
pub mod repo_selector;
pub mod status_line;
pub mod terminal_pane;
pub mod worktree_tree;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::action::Action;

pub trait Component {
    fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        let _ = key;
        Action::None
    }

    fn render(&self, frame: &mut Frame, area: Rect);
}
