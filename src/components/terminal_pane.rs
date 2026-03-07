use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::components::Component;

const PLACEHOLDER_TEXT: &str = "No session selected. Press 'n' to create a new session.";

pub struct TerminalPane {
    pub focused: bool,
}

impl TerminalPane {
    pub fn new() -> Self {
        Self { focused: false }
    }

    fn border_color(&self) -> Color {
        if self.focused {
            Color::Cyan
        } else {
            Color::DarkGray
        }
    }
}

impl Component for TerminalPane {
    fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Terminal ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color()));

        let paragraph = Paragraph::new(PLACEHOLDER_TEXT).block(block);

        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    #[test]
    fn renders_placeholder_when_no_session() {
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let pane = TerminalPane::new();

        terminal
            .draw(|frame| {
                pane.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..10 {
            for x in 0..80 {
                text.push_str(buffer[(x, y)].symbol());
            }
        }
        assert!(
            text.contains("No session selected"),
            "Should contain placeholder text, got: {text}"
        );
    }
}
