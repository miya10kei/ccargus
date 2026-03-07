use std::path::Path;

use color_eyre::Result;
use color_eyre::eyre::eyre;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    /// Full path to the repository root
    pub path: String,
    /// Display name (e.g., "github.com/miya10kei/ccargus")
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    /// Full path to the worktree
    pub path: String,
    /// Branch name
    pub branch: String,
    /// Whether this is the main worktree (bare/main)
    pub is_main: bool,
}

/// Create a new worktree for a repository.
/// Runs `git worktree add <path> -b <branch_name>`.
pub fn create_worktree(repo_path: &str, branch_name: &str) -> Result<Worktree> {
    let repo_dir = Path::new(repo_path);
    let repo_name = repo_dir
        .file_name()
        .map_or("repo", |n| n.to_str().unwrap_or("repo"));
    let worktree_path = repo_dir
        .parent()
        .unwrap_or(repo_dir)
        .join(format!("{repo_name}-{branch_name}"));

    let output = std::process::Command::new("git")
        .args([
            "worktree",
            "add",
            &worktree_path.to_string_lossy(),
            "-b",
            branch_name,
        ])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!("git worktree add failed: {stderr}"));
    }

    Ok(Worktree {
        path: worktree_path.to_string_lossy().to_string(),
        branch: branch_name.to_string(),
        is_main: false,
    })
}

/// Filter repositories by query string (case-insensitive substring match).
pub fn filter_repositories<'a>(repos: &'a [Repository], query: &str) -> Vec<&'a Repository> {
    if query.is_empty() {
        return repos.iter().collect();
    }
    let query_lower = query.to_lowercase();
    repos
        .iter()
        .filter(|r| r.name.to_lowercase().contains(&query_lower))
        .collect()
}

/// List all repositories managed by ghq.
/// Runs `ghq list -p` and parses output.
/// Returns sorted list of `Repository`.
pub fn list_repositories() -> Result<Vec<Repository>> {
    let output = std::process::Command::new("ghq")
        .args(["list", "-p"])
        .output()?;

    if !output.status.success() {
        return Err(eyre!("ghq list failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut repos: Vec<Repository> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let path = line.trim().to_string();
            let name = extract_repo_name(&path);
            Repository { path, name }
        })
        .collect();
    repos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(repos)
}

/// List worktrees for a given repository.
/// Runs `git worktree list --porcelain` in the repo directory and parses output.
pub fn list_worktrees(repo_path: &str) -> Result<Vec<Worktree>> {
    let output = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        return Err(eyre!("git worktree list failed"));
    }

    Ok(parse_worktree_output(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

/// Extract repository display name from full path.
/// Looks for patterns like "github.com/owner/repo" in the path.
/// Falls back to the last 2 path components.
fn extract_repo_name(path: &str) -> String {
    for host in &["github.com", "gitlab.com", "bitbucket.org"] {
        if let Some(idx) = path.find(host) {
            return path[idx..].to_string();
        }
    }
    let parts: Vec<&str> = path.rsplit('/').take(2).collect();
    parts.into_iter().rev().collect::<Vec<_>>().join("/")
}

/// Parse `git worktree list --porcelain` output.
///
/// Format:
/// ```text
/// worktree /path/to/worktree
/// HEAD abc123
/// branch refs/heads/main
///
/// worktree /path/to/linked
/// HEAD def456
/// branch refs/heads/feature
/// ```
fn parse_worktree_output(output: &str) -> Vec<Worktree> {
    let mut worktrees = Vec::new();
    let mut current_path = String::new();
    let mut current_branch = String::new();
    let mut is_first = true;

    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            if !current_path.is_empty() {
                worktrees.push(Worktree {
                    path: current_path.clone(),
                    branch: current_branch.clone(),
                    is_main: is_first,
                });
                is_first = false;
            }
            current_path = path.to_string();
            current_branch = String::new();
        } else if let Some(branch) = line.strip_prefix("branch refs/heads/") {
            current_branch = branch.to_string();
        }
    }

    if !current_path.is_empty() {
        worktrees.push(Worktree {
            path: current_path,
            branch: current_branch,
            is_main: is_first,
        });
    }

    worktrees
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_repo_name_bitbucket() {
        let name = extract_repo_name("/home/user/dev/ghq/bitbucket.org/team/project");
        assert_eq!(name, "bitbucket.org/team/project");
    }

    #[test]
    fn extract_repo_name_fallback() {
        let name = extract_repo_name("/some/unknown/host/repo");
        assert_eq!(name, "host/repo");
    }

    #[test]
    fn extract_repo_name_github() {
        let name = extract_repo_name("/home/user/dev/ghq/github.com/owner/repo");
        assert_eq!(name, "github.com/owner/repo");
    }

    #[test]
    fn extract_repo_name_gitlab() {
        let name = extract_repo_name("/home/user/dev/ghq/gitlab.com/owner/repo");
        assert_eq!(name, "gitlab.com/owner/repo");
    }

    #[test]
    fn filter_repositories_case_insensitive() {
        let repos = vec![Repository {
            path: "/a".to_string(),
            name: "github.com/Owner/Repo".to_string(),
        }];
        let filtered = filter_repositories(&repos, "owner");
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn filter_repositories_empty_query() {
        let repos = vec![
            Repository {
                path: "/a".to_string(),
                name: "github.com/a/b".to_string(),
            },
            Repository {
                path: "/c".to_string(),
                name: "github.com/c/d".to_string(),
            },
        ];
        let filtered = filter_repositories(&repos, "");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_repositories_matches() {
        let repos = vec![
            Repository {
                path: "/a".to_string(),
                name: "github.com/miya10kei/ccargus".to_string(),
            },
            Repository {
                path: "/b".to_string(),
                name: "github.com/miya10kei/other".to_string(),
            },
        ];
        let filtered = filter_repositories(&repos, "ccargus");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "github.com/miya10kei/ccargus");
    }

    #[test]
    fn filter_repositories_no_match() {
        let repos = vec![Repository {
            path: "/a".to_string(),
            name: "github.com/a/b".to_string(),
        }];
        let filtered = filter_repositories(&repos, "xyz");
        assert!(filtered.is_empty());
    }

    #[test]
    fn parse_worktree_empty() {
        let worktrees = parse_worktree_output("");
        assert!(worktrees.is_empty());
    }

    #[test]
    fn parse_worktree_multiple() {
        let output = "\
worktree /path/to/repo
HEAD abc
branch refs/heads/main

worktree /path/to/repo-feat
HEAD def
branch refs/heads/feat
";
        let worktrees = parse_worktree_output(output);
        assert_eq!(worktrees.len(), 2);
        assert!(worktrees[0].is_main);
        assert!(!worktrees[1].is_main);
        assert_eq!(worktrees[1].branch, "feat");
    }

    #[test]
    fn parse_worktree_single() {
        let output = "worktree /path/to/repo\nHEAD abc123\nbranch refs/heads/main\n\n";
        let worktrees = parse_worktree_output(output);
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, "/path/to/repo");
        assert_eq!(worktrees[0].branch, "main");
        assert!(worktrees[0].is_main);
    }
}
