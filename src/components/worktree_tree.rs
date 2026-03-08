use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::components::Component;
use crate::domain::claude_status::ClaudeStatus;

pub struct WorktreeItem {
    pub branch: String,
    pub repo: String,
    pub status: ClaudeStatus,
}

pub struct WorktreeTree {
    pub focused: bool,
    pub selected: usize,
    pub worktrees: Vec<WorktreeItem>,
}

impl WorktreeTree {
    pub fn new() -> Self {
        Self {
            focused: true,
            selected: 0,
            worktrees: Vec::new(),
        }
    }

    fn border_color(&self) -> Color {
        if self.focused {
            Color::Cyan
        } else {
            Color::DarkGray
        }
    }

    fn build_tree_items(&self) -> Vec<ListItem<'static>> {
        if self.worktrees.is_empty() {
            return vec![ListItem::new("  (no worktrees)")];
        }

        let groups = group_by_repo(&self.worktrees);
        let mut items = Vec::new();
        let mut flat_index = 0usize;

        for (repo, entries) in &groups {
            items.push(ListItem::new(format!("\u{25bc} {repo}")));
            for entry in entries {
                let marker = entry.status.icon();
                let is_selected = flat_index == self.selected;
                let marker_color = entry.status.color();
                let style = if is_selected {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };
                let marker_span =
                    Span::styled(format!("  {marker} "), Style::default().fg(marker_color));
                let branch_span = Span::styled(entry.branch.clone(), style);
                items.push(ListItem::new(Line::from(vec![marker_span, branch_span])));
                flat_index += 1;
            }
        }

        items
    }

    fn selected_list_index(&self) -> Option<usize> {
        if self.worktrees.is_empty() {
            return None;
        }

        let groups = group_by_repo(&self.worktrees);
        let mut list_index = 0usize;
        let mut flat_index = 0usize;

        for (_repo, entries) in &groups {
            list_index += 1; // repo header row
            for _entry in entries {
                if flat_index == self.selected {
                    return Some(list_index);
                }
                list_index += 1;
                flat_index += 1;
            }
        }

        None
    }
}

pub fn group_by_repo(worktrees: &[WorktreeItem]) -> Vec<(String, Vec<&WorktreeItem>)> {
    let mut groups: Vec<(String, Vec<&WorktreeItem>)> = Vec::new();

    for wt in worktrees {
        if let Some(group) = groups.iter_mut().find(|(repo, _)| repo == &wt.repo) {
            group.1.push(wt);
        } else {
            groups.push((wt.repo.clone(), vec![wt]));
        }
    }

    groups
}

impl Component for WorktreeTree {
    fn render(&self, frame: &mut Frame, area: Rect) {
        let items = self.build_tree_items();

        let block = Block::default()
            .title(" Worktrees ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color()));

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut state = ListState::default();
        state.select(self.selected_list_index());

        frame.render_stateful_widget(list, area, &mut state);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;

    use super::*;

    fn make_entry(_name: &str, repo: &str, branch: &str) -> WorktreeItem {
        WorktreeItem {
            branch: branch.to_owned(),
            repo: repo.to_owned(),
            status: ClaudeStatus::Processing,
        }
    }

    #[test]
    fn group_by_repo_empty_input_returns_empty() {
        let result = group_by_repo(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn group_by_repo_same_repo_returns_one_group() {
        let entries = vec![
            make_entry("s1", "myrepo", "main"),
            make_entry("s2", "myrepo", "dev"),
        ];
        let result = group_by_repo(&entries);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "myrepo");
        assert_eq!(result[0].1.len(), 2);
        assert_eq!(result[0].1[0].branch, "main");
        assert_eq!(result[0].1[1].branch, "dev");
    }

    #[test]
    fn group_by_repo_different_repos_returns_separate_groups() {
        let entries = vec![
            make_entry("s1", "repo-a", "main"),
            make_entry("s2", "repo-b", "main"),
        ];
        let result = group_by_repo(&entries);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "repo-a");
        assert_eq!(result[1].0, "repo-b");
    }

    #[test]
    fn group_by_repo_preserves_insertion_order() {
        let entries = vec![
            make_entry("s1", "zzz-repo", "main"),
            make_entry("s2", "aaa-repo", "main"),
            make_entry("s3", "zzz-repo", "dev"),
        ];
        let result = group_by_repo(&entries);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "zzz-repo");
        assert_eq!(result[0].1.len(), 2);
        assert_eq!(result[1].0, "aaa-repo");
        assert_eq!(result[1].1.len(), 1);
    }

    #[test]
    fn renders_with_border_color_when_focused() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = WorktreeTree {
            focused: true,
            selected: 0,
            worktrees: Vec::new(),
        };

        terminal
            .draw(|frame| {
                tree.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let corner = &buffer[(0, 0)];
        assert_eq!(corner.fg, Color::Cyan);
    }

    #[test]
    fn renders_with_border_color_when_unfocused() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = WorktreeTree {
            focused: false,
            selected: 0,
            worktrees: Vec::new(),
        };

        terminal
            .draw(|frame| {
                tree.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let corner = &buffer[(0, 0)];
        assert_eq!(corner.fg, Color::DarkGray);
    }

    #[test]
    fn renders_with_worktrees_title() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = WorktreeTree::new();

        terminal
            .draw(|frame| {
                tree.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text: String = (0..40)
            .map(|x| buffer[(x, 0)].symbol().to_string())
            .collect::<String>();
        assert!(
            text.contains(" Worktrees "),
            "Title should contain ' Worktrees ', got: {text}"
        );
    }

    #[test]
    fn renders_no_worktrees_when_empty() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = WorktreeTree::new();

        terminal
            .draw(|frame| {
                tree.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text: String = (0..40)
            .map(|x| buffer[(x, 1)].symbol().to_string())
            .collect::<String>();
        assert!(
            text.contains("(no worktrees)"),
            "Should show '(no worktrees)' when empty, got: {text}"
        );
    }

    #[test]
    fn renders_tree_with_worktrees() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = WorktreeTree {
            focused: true,
            selected: 0,
            worktrees: vec![
                make_entry("s1", "my/repo", "main"),
                make_entry("s2", "my/repo", "dev"),
            ],
        };

        terminal
            .draw(|frame| {
                tree.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let row1: String = (0..40)
            .map(|x| buffer[(x, 1)].symbol().to_string())
            .collect::<String>();
        let row2: String = (0..40)
            .map(|x| buffer[(x, 2)].symbol().to_string())
            .collect::<String>();
        assert!(
            row1.contains("my/repo"),
            "Should show repo header, got: {row1}"
        );
        assert!(
            row2.contains("main"),
            "Should show branch name, got: {row2}"
        );
    }
}
