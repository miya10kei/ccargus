use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};

use crate::action::Action;
use crate::components::Component;
use crate::domain::repo::{Repository, filter_repositories, list_repositories};
use crate::domain::worktree::{branch_exists, list_local_branches};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorStep {
    InputBranchName,
    SelectBaseBranch,
    SelectRepo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionResult {
    pub base_branch: Option<String>,
    pub branch: String,
    pub repo: Repository,
}

pub struct RepoSelector {
    pub result: Option<SelectionResult>,
    pub visible: bool,
    base_branch_filter: String,
    base_branch_list_state: ListState,
    branch_input: String,
    filter_query: String,
    local_branches: Vec<String>,
    repo_list_state: ListState,
    repositories: Vec<Repository>,
    selected_repo: Option<Repository>,
    step: SelectorStep,
}

impl RepoSelector {
    pub fn new() -> Self {
        Self {
            base_branch_filter: String::new(),
            base_branch_list_state: ListState::default(),
            branch_input: String::new(),
            filter_query: String::new(),
            local_branches: Vec::new(),
            repo_list_state: ListState::default(),
            repositories: Vec::new(),
            result: None,
            selected_repo: None,
            step: SelectorStep::SelectRepo,
            visible: false,
        }
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn open(&mut self) {
        self.visible = true;
        self.step = SelectorStep::SelectRepo;
        self.branch_input.clear();
        self.filter_query.clear();
        self.result = None;
        self.selected_repo = None;

        match list_repositories() {
            Ok(repos) => {
                self.repositories = repos;
                self.repo_list_state.select(Some(0));
            }
            Err(_) => {
                self.repositories.clear();
            }
        }
    }

    pub fn take_result(&mut self) -> Option<SelectionResult> {
        self.result.take()
    }

    fn confirm_base_branch(&mut self) {
        let filtered = self.filtered_local_branches();
        if let Some(idx) = self.base_branch_list_state.selected()
            && let Some(base) = filtered.get(idx)
        {
            let base = base.clone();
            if let Some(repo) = &self.selected_repo {
                self.result = Some(SelectionResult {
                    base_branch: Some(base),
                    branch: self.branch_input.trim().to_string(),
                    repo: repo.clone(),
                });
                self.visible = false;
            }
        }
    }

    fn confirm_branch(&mut self) {
        let branch = self.branch_input.trim().to_string();
        if branch.is_empty() {
            return;
        }
        if let Some(repo) = &self.selected_repo {
            if branch_exists(&repo.path, &branch).unwrap_or(false) {
                // Existing branch: skip base branch selection
                self.result = Some(SelectionResult {
                    base_branch: None,
                    branch,
                    repo: repo.clone(),
                });
                self.visible = false;
            } else {
                // New branch: transition to base branch selection
                self.local_branches = list_local_branches(&repo.path).unwrap_or_default();
                self.base_branch_filter.clear();
                self.base_branch_list_state.select(Some(0));
                self.step = SelectorStep::SelectBaseBranch;
            }
        }
    }

    fn filtered_local_branches(&self) -> Vec<String> {
        if self.base_branch_filter.is_empty() {
            return self.local_branches.clone();
        }
        let query = self.base_branch_filter.to_lowercase();
        self.local_branches
            .iter()
            .filter(|b| b.to_lowercase().contains(&query))
            .cloned()
            .collect()
    }

    fn filtered_repos(&self) -> Vec<&Repository> {
        filter_repositories(&self.repositories, &self.filter_query)
    }

    fn move_down(&mut self) {
        match self.step {
            SelectorStep::SelectBaseBranch => {
                let max = self.filtered_local_branches().len();
                let current = self.base_branch_list_state.selected().unwrap_or(0);
                if max > 0 {
                    self.base_branch_list_state
                        .select(Some((current + 1).min(max - 1)));
                }
            }
            SelectorStep::SelectRepo => {
                let max = self.filtered_repos().len();
                let current = self.repo_list_state.selected().unwrap_or(0);
                if max > 0 {
                    self.repo_list_state
                        .select(Some((current + 1).min(max - 1)));
                }
            }
            SelectorStep::InputBranchName => {}
        }
    }

    fn move_up(&mut self) {
        match self.step {
            SelectorStep::SelectBaseBranch => {
                let current = self.base_branch_list_state.selected().unwrap_or(0);
                self.base_branch_list_state
                    .select(Some(current.saturating_sub(1)));
            }
            SelectorStep::SelectRepo => {
                let current = self.repo_list_state.selected().unwrap_or(0);
                self.repo_list_state.select(Some(current.saturating_sub(1)));
            }
            SelectorStep::InputBranchName => {}
        }
    }

    fn render_base_branch_selector(&self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);

        let input = Paragraph::new(format!("  {}", self.base_branch_filter)).block(
            Block::default()
                .title(format!(" Base Branch for '{}' ", self.branch_input.trim()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        frame.render_widget(input, layout[0]);

        let filtered = self.filtered_local_branches();
        let items: Vec<ListItem> = filtered
            .iter()
            .map(|b| ListItem::new(format!("  {b}")))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!(" Branches ({}) ", filtered.len()))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut state = self.base_branch_list_state;
        frame.render_stateful_widget(list, layout[1], &mut state);
    }

    fn render_branch_input(&self, frame: &mut Frame, area: Rect) {
        let repo_name = self.selected_repo.as_ref().map_or("", |r| &r.name);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);

        let input = Paragraph::new(format!("  {}", self.branch_input)).block(
            Block::default()
                .title(format!(" New Branch: {repo_name} "))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        frame.render_widget(input, layout[0]);

        let help = Paragraph::new("  Enter: create  |  Esc: back").block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(help, layout[1]);
    }

    fn render_repo_list(&self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);

        let input = Paragraph::new(format!("  {}", self.filter_query)).block(
            Block::default()
                .title(" Filter Repository ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        frame.render_widget(input, layout[0]);

        let filtered = self.filtered_repos();
        let items: Vec<ListItem> = filtered
            .iter()
            .map(|r| ListItem::new(format!("  {}", r.name)))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!(" Repositories ({}) ", filtered.len()))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut state = self.repo_list_state;
        frame.render_stateful_widget(list, layout[1], &mut state);
    }

    fn active_filter(&self) -> &str {
        match self.step {
            SelectorStep::SelectBaseBranch => &self.base_branch_filter,
            SelectorStep::SelectRepo => &self.filter_query,
            SelectorStep::InputBranchName => "",
        }
    }

    fn select_repo(&mut self) {
        let filtered = self.filtered_repos();
        if let Some(idx) = self.repo_list_state.selected()
            && let Some(repo) = filtered.get(idx)
        {
            let repo = (*repo).clone();
            self.selected_repo = Some(repo);
            self.step = SelectorStep::InputBranchName;
            self.branch_input.clear();
        }
    }
}

impl Component for RepoSelector {
    fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if !self.visible {
            return Action::None;
        }

        match key.code {
            KeyCode::Esc => match self.step {
                SelectorStep::InputBranchName => {
                    self.step = SelectorStep::SelectRepo;
                }
                SelectorStep::SelectBaseBranch => {
                    self.step = SelectorStep::InputBranchName;
                }
                SelectorStep::SelectRepo => {
                    self.close();
                }
            },
            KeyCode::Enter => match self.step {
                SelectorStep::InputBranchName => self.confirm_branch(),
                SelectorStep::SelectBaseBranch => self.confirm_base_branch(),
                SelectorStep::SelectRepo => self.select_repo(),
            },
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            KeyCode::Char('k')
                if matches!(
                    self.step,
                    SelectorStep::SelectRepo | SelectorStep::SelectBaseBranch
                ) && self.active_filter().is_empty() =>
            {
                self.move_up();
            }
            KeyCode::Char('j')
                if matches!(
                    self.step,
                    SelectorStep::SelectRepo | SelectorStep::SelectBaseBranch
                ) && self.active_filter().is_empty() =>
            {
                self.move_down();
            }
            KeyCode::Char(c) => match self.step {
                SelectorStep::InputBranchName => {
                    self.branch_input.push(c);
                }
                SelectorStep::SelectBaseBranch => {
                    self.base_branch_filter.push(c);
                    self.base_branch_list_state.select(Some(0));
                }
                SelectorStep::SelectRepo => {
                    self.filter_query.push(c);
                    self.repo_list_state.select(Some(0));
                }
            },
            KeyCode::Backspace => match self.step {
                SelectorStep::InputBranchName => {
                    self.branch_input.pop();
                }
                SelectorStep::SelectBaseBranch => {
                    self.base_branch_filter.pop();
                    self.base_branch_list_state.select(Some(0));
                }
                SelectorStep::SelectRepo => {
                    self.filter_query.pop();
                    self.repo_list_state.select(Some(0));
                }
            },
            _ => {}
        }
        Action::None
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let popup_area = centered_rect(60, 60, area);

        frame.render_widget(Clear, popup_area);

        match self.step {
            SelectorStep::InputBranchName => self.render_branch_input(frame, popup_area),
            SelectorStep::SelectBaseBranch => self.render_base_branch_selector(frame, popup_area),
            SelectorStep::SelectRepo => self.render_repo_list(frame, popup_area),
        }
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
    fn centered_rect_produces_valid_area() {
        let area = Rect::new(0, 0, 100, 50);
        let popup = centered_rect(60, 60, area);
        assert!(popup.width > 0);
        assert!(popup.height > 0);
        assert!(popup.x > 0);
        assert!(popup.y > 0);
    }

    #[test]
    fn close_hides_selector() {
        let mut selector = RepoSelector::new();
        selector.visible = true;
        selector.close();
        assert!(!selector.visible);
    }

    #[test]
    fn new_selector_is_not_visible() {
        let selector = RepoSelector::new();
        assert!(!selector.visible);
        assert_eq!(selector.step, SelectorStep::SelectRepo);
        assert!(selector.result.is_none());
    }

    #[test]
    fn take_result_consumes() {
        let mut selector = RepoSelector::new();
        selector.result = Some(SelectionResult {
            base_branch: None,
            branch: "main".to_string(),
            repo: Repository {
                path: "/tmp".to_string(),
                name: "test".to_string(),
            },
        });
        let result = selector.take_result();
        assert!(result.is_some());
        assert!(selector.result.is_none());
    }

    #[test]
    fn confirm_branch_requires_non_empty() {
        let mut selector = RepoSelector::new();
        selector.selected_repo = Some(Repository {
            path: "/tmp".to_string(),
            name: "test".to_string(),
        });
        selector.branch_input = "  ".to_string();
        selector.confirm_branch();
        assert!(selector.result.is_none());
    }

    #[test]
    fn confirm_branch_existing_branch_sets_result() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_path).unwrap();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@test.com",
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["branch", "feat-existing"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        let mut selector = RepoSelector::new();
        selector.selected_repo = Some(Repository {
            path: repo_path.to_string_lossy().to_string(),
            name: "test/repo".to_string(),
        });
        selector.branch_input = "feat-existing".to_string();
        selector.confirm_branch();
        let result = selector.result.as_ref().unwrap();
        assert_eq!(result.branch, "feat-existing");
        assert!(result.base_branch.is_none());
    }

    #[test]
    fn confirm_branch_new_branch_transitions_to_select_base() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_path).unwrap();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@test.com",
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        let mut selector = RepoSelector::new();
        selector.selected_repo = Some(Repository {
            path: repo_path.to_string_lossy().to_string(),
            name: "test/repo".to_string(),
        });
        selector.branch_input = "feat-new-branch".to_string();
        selector.confirm_branch();
        assert!(selector.result.is_none());
        assert_eq!(selector.step, SelectorStep::SelectBaseBranch);
        assert!(!selector.local_branches.is_empty());
    }
}
