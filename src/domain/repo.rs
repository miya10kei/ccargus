use color_eyre::Result;
use color_eyre::eyre::eyre;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    pub path: String,
    pub name: String,
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

/// Extract repository display name from full path.
/// Detects hostname-like components (containing a dot) in the path
/// and returns everything from that component onwards.
/// Falls back to the last 2 path components.
fn extract_repo_name(path: &str) -> String {
    let components: Vec<&str> = path.split('/').collect();
    for (i, component) in components.iter().enumerate() {
        if component.contains('.') && !component.starts_with('.') {
            return components[i..].join("/");
        }
    }
    let parts: Vec<&str> = path.rsplit('/').take(2).collect();
    parts.into_iter().rev().collect::<Vec<_>>().join("/")
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
    fn extract_repo_name_custom_host() {
        let name = extract_repo_name("/home/user/dev/ghq/git.example.com/team/project");
        assert_eq!(name, "git.example.com/team/project");
    }

    #[test]
    fn extract_repo_name_hidden_dir_ignored() {
        let name = extract_repo_name("/home/user/.local/share/repos/owner/project");
        assert_eq!(name, "owner/project");
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
    fn extract_repo_name_single_component() {
        let name = extract_repo_name("/repo");
        assert_eq!(name, "/repo");
    }
}
