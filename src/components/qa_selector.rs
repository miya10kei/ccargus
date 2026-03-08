use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

use crate::action::Action;
use crate::components::Component;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QaMode {
    Fork,
    New,
}

pub struct QaSelector {
    pub visible: bool,
    list_state: ListState,
    result: Option<QaMode>,
}

impl QaSelector {
    pub fn new() -> Self {
        Self {
            visible: false,
            list_state: ListState::default(),
            result: None,
        }
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn open(&mut self) {
        self.visible = true;
        self.result = None;
        self.list_state.select(Some(0));
    }

    pub fn take_result(&mut self) -> Option<QaMode> {
        self.result.take()
    }
}

impl Component for QaSelector {
    fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if !self.visible {
            return Action::None;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let current = self.list_state.selected().unwrap_or(0);
                self.list_state.select(Some((current + 1).min(1)));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let current = self.list_state.selected().unwrap_or(0);
                self.list_state.select(Some(current.saturating_sub(1)));
            }
            KeyCode::Enter => {
                let mode = match self.list_state.selected() {
                    Some(0) => QaMode::Fork,
                    Some(1) => QaMode::New,
                    _ => return Action::None,
                };
                self.result = Some(mode);
                self.visible = false;
            }
            KeyCode::Esc => self.close(),
            _ => {}
        }
        Action::None
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let popup_area = centered_rect(40, 20, area);
        frame.render_widget(Clear, popup_area);

        let items = vec![
            ListItem::new("  Fork  - Continue current context"),
            ListItem::new("  New   - Start fresh Q&A"),
        ];

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Q&A Mode ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut state = self.list_state;
        frame.render_stateful_widget(list, popup_area, &mut state);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_selector_is_not_visible() {
        let selector = QaSelector::new();
        assert!(!selector.visible);
        assert!(selector.result.is_none());
    }

    #[test]
    fn open_makes_visible() {
        let mut selector = QaSelector::new();
        selector.open();
        assert!(selector.visible);
        assert_eq!(selector.list_state.selected(), Some(0));
    }

    #[test]
    fn close_hides_selector() {
        let mut selector = QaSelector::new();
        selector.open();
        selector.close();
        assert!(!selector.visible);
    }

    #[test]
    fn take_result_consumes_result() {
        let mut selector = QaSelector::new();
        selector.result = Some(QaMode::Fork);
        let result = selector.take_result();
        assert_eq!(result, Some(QaMode::Fork));
        assert!(selector.result.is_none());
    }

    #[test]
    fn enter_selects_fork_mode() {
        let mut selector = QaSelector::new();
        selector.open();
        // Default selection is 0 (Fork)
        let key = KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::NONE);
        selector.handle_key_event(key);
        assert!(!selector.visible);
        assert_eq!(selector.take_result(), Some(QaMode::Fork));
    }

    #[test]
    fn enter_selects_new_mode() {
        let mut selector = QaSelector::new();
        selector.open();
        // Move down to New
        let down = KeyEvent::new(KeyCode::Down, crossterm::event::KeyModifiers::NONE);
        selector.handle_key_event(down);
        let enter = KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::NONE);
        selector.handle_key_event(enter);
        assert!(!selector.visible);
        assert_eq!(selector.take_result(), Some(QaMode::New));
    }

    #[test]
    fn esc_closes_without_result() {
        let mut selector = QaSelector::new();
        selector.open();
        let esc = KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::NONE);
        selector.handle_key_event(esc);
        assert!(!selector.visible);
        assert!(selector.take_result().is_none());
    }
}
