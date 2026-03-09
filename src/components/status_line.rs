use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::components::Component;
use crate::context::NotificationLevel;

#[derive(Debug, Clone)]
pub struct StatusNotification {
    pub level: NotificationLevel,
    pub message: String,
}

pub struct StatusLine {
    pub branch: String,
    pub copy_hint: Option<String>,
    pub dir: String,
    pub notification: Option<StatusNotification>,
    pub qa_mode: Option<String>,
    pub repo: String,
    pub status: String,
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

        if let Some(hint) = &self.copy_hint {
            spans.push(Span::styled(
                format!(" {hint} "),
                Style::default().fg(Color::Magenta),
            ));
        }

        if let Some(notif) = &self.notification {
            let color = match notif.level {
                NotificationLevel::Error => Color::Red,
                NotificationLevel::Info => Color::Green,
            };
            spans.push(Span::styled(
                format!(" {} ", notif.message),
                Style::default().fg(color),
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
            copy_hint: None,
            dir: "/home/user/project".to_owned(),
            notification: None,
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

    fn render_status_line(status: &StatusLine, width: u16) -> String {
        let backend = TestBackend::new(width, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                status.render(frame, frame.area());
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..width)
            .map(|x| buffer[(x, 0)].symbol().to_string())
            .collect()
    }

    #[test]
    fn renders_all_fields() {
        let status = StatusLine {
            branch: "feat".to_owned(),
            copy_hint: Some("[v] select".to_owned()),
            dir: "/home".to_owned(),
            notification: None,
            qa_mode: Some("Fork".to_owned()),
            repo: "myrepo".to_owned(),
            status: "running".to_owned(),
        };
        let text = render_status_line(&status, 120);
        assert!(text.contains("myrepo"), "got: {text}");
        assert!(text.contains("feat"), "got: {text}");
        assert!(text.contains("/home"), "got: {text}");
        assert!(text.contains("running"), "got: {text}");
        assert!(text.contains("Q&A: Fork"), "got: {text}");
        assert!(text.contains("[v] select"), "got: {text}");
    }

    #[test]
    fn renders_copy_hint_when_present() {
        let status = StatusLine {
            branch: "main".to_owned(),
            copy_hint: Some("[v] select".to_owned()),
            dir: String::new(),
            notification: None,
            qa_mode: None,
            repo: "r".to_owned(),
            status: String::new(),
        };
        let text = render_status_line(&status, 80);
        assert!(
            text.contains("[v] select"),
            "Should contain copy hint, got: {text}"
        );
    }

    #[test]
    fn renders_qa_mode_when_present() {
        let status = StatusLine {
            branch: "main".to_owned(),
            copy_hint: None,
            dir: String::new(),
            notification: None,
            qa_mode: Some("Fork".to_owned()),
            repo: "r".to_owned(),
            status: String::new(),
        };
        let text = render_status_line(&status, 80);
        assert!(
            text.contains("Q&A: Fork"),
            "Should contain Q&A mode, got: {text}"
        );
    }

    #[test]
    fn renders_status_and_dir() {
        let status = StatusLine {
            branch: "main".to_owned(),
            copy_hint: None,
            dir: "/home/user/project".to_owned(),
            notification: None,
            qa_mode: None,
            repo: "r".to_owned(),
            status: "processing".to_owned(),
        };
        let text = render_status_line(&status, 80);
        assert!(
            text.contains("/home/user/project"),
            "Should contain dir, got: {text}"
        );
        assert!(
            text.contains("processing"),
            "Should contain status, got: {text}"
        );
    }
}
