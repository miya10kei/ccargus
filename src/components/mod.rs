pub mod qa_selector;
pub mod repo_selector;
pub mod session_tree;
pub mod status_line;
pub mod terminal_pane;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::action::Action;

pub trait Component {
    /// Initialize the component
    fn init(&mut self) -> color_eyre::Result<()> {
        Ok(())
    }

    /// Handle a key event, returning an Action
    fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        let _ = key;
        Action::None
    }

    /// Update component state based on an action
    fn update(&mut self, action: &Action) -> color_eyre::Result<()> {
        let _ = action;
        Ok(())
    }

    /// Render the component
    fn render(&self, frame: &mut Frame, area: Rect);
}
