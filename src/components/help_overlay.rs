use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::components::Component;
use crate::components::utils::centered_rect_percent;

pub struct HelpOverlay {
    pub visible: bool,
}

impl HelpOverlay {
    pub fn new() -> Self {
        Self { visible: false }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
}

fn help_entry<'a>(key: &'a str, desc: &'a str, style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {key:<10}"), style),
        Span::raw(desc),
    ])
}

fn build_help_lines() -> Vec<Line<'static>> {
    let header = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let key = Style::default().fg(Color::Green);

    vec![
        Line::from(Span::styled("Worktree Pane", header)),
        help_entry("n", "New worktree", key),
        help_entry("d", "Delete worktree", key),
        help_entry("e", "Open editor", key),
        help_entry("s", "Start Q&A session", key),
        help_entry("x", "Stop worktree", key),
        help_entry("j/k", "Navigate worktrees", key),
        help_entry("Enter", "Focus / Start worktree", key),
        help_entry("q", "Quit", key),
        Line::from(""),
        Line::from(Span::styled("Terminal Pane", header)),
        help_entry("Tab", "Toggle focus", key),
        help_entry("Ctrl+w", "Switch main/Q&A terminal", key),
        help_entry("Ctrl+b", "Enter scroll mode", key),
        help_entry("Ctrl+d", "Close Q&A session", key),
        Line::from(""),
        Line::from(Span::styled("Scroll Mode", header)),
        help_entry("j/k", "Scroll down/up", key),
        help_entry("Ctrl+d/u", "Half page down/up", key),
        help_entry("v", "Enter copy mode", key),
        help_entry("q/Esc", "Exit scroll mode", key),
        Line::from(""),
        Line::from(Span::styled("Copy Mode", header)),
        help_entry("v", "Toggle selection", key),
        help_entry("y", "Yank (copy) selection", key),
        help_entry("q/Esc", "Exit copy mode", key),
        Line::from(""),
        Line::from(Span::styled(
            "Press ? or Esc to close",
            Style::default().fg(Color::DarkGray),
        )),
    ]
}

impl Component for HelpOverlay {
    fn handle_key_event(&mut self, key: KeyEvent) {
        if !self.visible {
            return;
        }

        if matches!(key.code, KeyCode::Esc | KeyCode::Char('?' | 'q')) {
            self.visible = false;
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let popup_area = centered_rect_percent(60, 70, area);
        frame.render_widget(Clear, popup_area);

        let paragraph = Paragraph::new(build_help_lines()).block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );

        frame.render_widget(paragraph, popup_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_overlay_is_not_visible() {
        let overlay = HelpOverlay::new();
        assert!(!overlay.visible);
    }

    #[test]
    fn toggle_changes_visibility() {
        let mut overlay = HelpOverlay::new();
        overlay.toggle();
        assert!(overlay.visible);
        overlay.toggle();
        assert!(!overlay.visible);
    }

    #[test]
    fn esc_closes_overlay() {
        let mut overlay = HelpOverlay::new();
        overlay.visible = true;
        let key = KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::NONE);
        overlay.handle_key_event(key);
        assert!(!overlay.visible);
    }

    #[test]
    fn question_mark_closes_overlay() {
        let mut overlay = HelpOverlay::new();
        overlay.visible = true;
        let key = KeyEvent::new(KeyCode::Char('?'), crossterm::event::KeyModifiers::NONE);
        overlay.handle_key_event(key);
        assert!(!overlay.visible);
    }

    #[test]
    fn other_keys_ignored_when_visible() {
        let mut overlay = HelpOverlay::new();
        overlay.visible = true;
        let key = KeyEvent::new(KeyCode::Char('x'), crossterm::event::KeyModifiers::NONE);
        overlay.handle_key_event(key);
        assert!(overlay.visible);
    }
}
