use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::action::Action;
use crate::components::Component;

pub struct ConfirmDialog {
    pub visible: bool,
    message: String,
    confirmed: Option<bool>,
}

impl ConfirmDialog {
    pub fn new() -> Self {
        Self {
            confirmed: None,
            message: String::new(),
            visible: false,
        }
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn open(&mut self, message: impl Into<String>) {
        self.visible = true;
        self.message = message.into();
        self.confirmed = None;
    }

    pub fn take_result(&mut self) -> Option<bool> {
        self.confirmed.take()
    }
}

impl Component for ConfirmDialog {
    fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if !self.visible {
            return Action::None;
        }

        match key.code {
            KeyCode::Char('y' | 'Y') => {
                self.confirmed = Some(true);
                self.close();
            }
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                self.confirmed = Some(false);
                self.close();
            }
            _ => {}
        }
        Action::None
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let popup_area = centered_rect(50, 5, area);
        frame.render_widget(Clear, popup_area);

        let text = vec![
            Line::from(self.message.as_str()),
            Line::from(""),
            Line::from(vec![
                Span::styled("[y]", Style::default().fg(Color::Green)),
                Span::raw(" Yes  "),
                Span::styled("[n]", Style::default().fg(Color::Red)),
                Span::raw(" No"),
            ]),
        ];

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .title(" Confirm ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(paragraph, popup_area);
    }
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
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
    fn new_dialog_is_not_visible() {
        let dialog = ConfirmDialog::new();
        assert!(!dialog.visible);
        assert!(dialog.confirmed.is_none());
    }

    #[test]
    fn open_makes_visible_with_message() {
        let mut dialog = ConfirmDialog::new();
        dialog.open("Delete session?");
        assert!(dialog.visible);
        assert_eq!(dialog.message, "Delete session?");
        assert!(dialog.confirmed.is_none());
    }

    #[test]
    fn y_confirms() {
        let mut dialog = ConfirmDialog::new();
        dialog.open("test");
        let key = KeyEvent::new(KeyCode::Char('y'), crossterm::event::KeyModifiers::NONE);
        dialog.handle_key_event(key);
        assert!(!dialog.visible);
        assert_eq!(dialog.take_result(), Some(true));
    }

    #[test]
    fn n_denies() {
        let mut dialog = ConfirmDialog::new();
        dialog.open("test");
        let key = KeyEvent::new(KeyCode::Char('n'), crossterm::event::KeyModifiers::NONE);
        dialog.handle_key_event(key);
        assert!(!dialog.visible);
        assert_eq!(dialog.take_result(), Some(false));
    }

    #[test]
    fn esc_denies() {
        let mut dialog = ConfirmDialog::new();
        dialog.open("test");
        let key = KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::NONE);
        dialog.handle_key_event(key);
        assert!(!dialog.visible);
        assert_eq!(dialog.take_result(), Some(false));
    }

    #[test]
    fn take_result_consumes_result() {
        let mut dialog = ConfirmDialog::new();
        dialog.confirmed = Some(true);
        let result = dialog.take_result();
        assert_eq!(result, Some(true));
        assert!(dialog.confirmed.is_none());
    }

    #[test]
    fn other_keys_are_ignored() {
        let mut dialog = ConfirmDialog::new();
        dialog.open("test");
        let key = KeyEvent::new(KeyCode::Char('x'), crossterm::event::KeyModifiers::NONE);
        dialog.handle_key_event(key);
        assert!(dialog.visible);
        assert!(dialog.confirmed.is_none());
    }
}
