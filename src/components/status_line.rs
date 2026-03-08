use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::components::Component;

pub struct StatusLine {
    pub branch: String,
    pub dir: String,
    pub qa_mode: Option<String>,
    pub repo: String,
    pub status: String,
}

impl StatusLine {
    pub fn new() -> Self {
        Self {
            branch: String::new(),
            dir: String::new(),
            qa_mode: None,
            repo: String::new(),
            status: String::new(),
        }
    }
}

impl Component for StatusLine {
    fn render(&self, frame: &mut Frame, area: Rect) {
        let mut spans = vec![
            Span::styled(
                format!(" {} ", self.repo),
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ),
            Span::styled(
                format!(" {} ", self.branch),
                Style::default().fg(Color::Black).bg(Color::Green),
            ),
            Span::styled(format!(" {} ", self.dir), Style::default().fg(Color::White)),
            Span::styled(
                format!(" {} ", self.status),
                Style::default().fg(Color::Yellow),
            ),
        ];

        if let Some(qa) = &self.qa_mode {
            spans.push(Span::styled(
                format!(" Q&A: {qa} "),
                Style::default().fg(Color::Magenta),
            ));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    #[test]
    fn renders_repo_and_branch() {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        let status = StatusLine {
            branch: "main".to_owned(),
            dir: "/home/user/project".to_owned(),
            qa_mode: None,
            repo: "miya10kei/ccargus".to_owned(),
            status: "running".to_owned(),
        };

        terminal
            .draw(|frame| {
                status.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text: String = (0..80)
            .map(|x| buffer[(x, 0)].symbol().to_string())
            .collect();
        assert!(
            text.contains("miya10kei/ccargus"),
            "Should contain repo name, got: {text}"
        );
        assert!(
            text.contains("main"),
            "Should contain branch name, got: {text}"
        );
    }
}
