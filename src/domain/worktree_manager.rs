use std::fs;
use std::path::PathBuf;

use color_eyre::Result;
use color_eyre::eyre::eyre;

use super::git::{
    branch_exists, ensure_branch_up_to_date, is_dir_empty, resolve_source_repo, run_git,
};
use super::repo::Repository;
use super::worktree_entry::WorktreeEntry;

pub struct WorktreeManager {
    base_dir: PathBuf,
}

impl WorktreeManager {
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&base_dir)?;
        Ok(Self { base_dir })
    }

    pub fn add_worktree(
        &self,
        repo: &Repository,
        branch: &str,
        base_branch: Option<&str>,
    ) -> Result<WorktreeEntry> {
        let repo_dir = self.repo_dir(&repo.name);
        fs::create_dir_all(&repo_dir)?;

        let worktree_path = repo_dir.join(branch);
        if worktree_path.exists() {
            return Err(eyre!(
                "worktree already exists: {}",
                worktree_path.display()
            ));
        }

        let worktree_path_str = worktree_path.to_string_lossy().to_string();
        if branch_exists(&repo.path, branch)? {
            // Existing branch: check it out in a new worktree
            run_git(
                &repo.path,
                &["worktree", "add", &worktree_path_str, branch],
                "git worktree add",
            )?;
        } else if let Some(base) = base_branch {
            // New branch from specified base: update base first, then create
            ensure_branch_up_to_date(&repo.path, base)?;
            run_git(
                &repo.path,
                &["worktree", "add", "-b", branch, &worktree_path_str, base],
                "git worktree add",
            )?;
        } else {
            // New branch from HEAD
            run_git(
                &repo.path,
                &["worktree", "add", "-b", branch, &worktree_path_str],
                "git worktree add",
            )?;
        }

        Ok(WorktreeEntry {
            branch: branch.to_string(),
            repo_name: repo.name.clone(),
            source_repo_path: repo.path.clone(),
            worktree_path,
        })
    }

    pub fn remove_worktree(&self, entry: &WorktreeEntry) -> Result<()> {
        const PROTECTED_BRANCHES: &[&str] = &["main", "master", "develop"];

        let worktree_path_str = entry.worktree_path.to_string_lossy();
        run_git(
            &entry.source_repo_path,
            &["worktree", "remove", "--force", &worktree_path_str],
            "git worktree remove",
        )?;
        if !PROTECTED_BRANCHES.contains(&entry.branch.as_str()) {
            let _ = run_git(
                &entry.source_repo_path,
                &["branch", "-D", &entry.branch],
                "git branch -D",
            );
        }

        // Clean up empty repo directory
        let repo_dir = self.repo_dir(&entry.repo_name);
        if repo_dir.exists() && is_dir_empty(&repo_dir)? {
            fs::remove_dir(&repo_dir)?;
        }

        Ok(())
    }

    pub fn scan(&self) -> Result<Vec<WorktreeEntry>> {
        let mut entries = Vec::new();

        if !self.base_dir.exists() {
            return Ok(entries);
        }

        for host_entry in fs::read_dir(&self.base_dir)? {
            let host_entry = host_entry?;
            let host_path = host_entry.path();
            if !host_path.is_dir() {
                continue;
            }

            for owner_entry in fs::read_dir(&host_path)? {
                let owner_entry = owner_entry?;
                let owner_path = owner_entry.path();
                if !owner_path.is_dir() {
                    continue;
                }

                for repo_entry in fs::read_dir(&owner_path)? {
                    let repo_entry = repo_entry?;
                    let repo_path = repo_entry.path();
                    if !repo_path.is_dir() {
                        continue;
                    }

                    let repo_name = format!(
                        "{}/{}/{}",
                        host_path.file_name().unwrap_or_default().to_string_lossy(),
                        owner_path.file_name().unwrap_or_default().to_string_lossy(),
                        repo_path.file_name().unwrap_or_default().to_string_lossy(),
                    );

                    for branch_entry in fs::read_dir(&repo_path)? {
                        let branch_entry = branch_entry?;
                        let branch_path = branch_entry.path();
                        if !branch_path.is_dir() {
                            continue;
                        }

                        // Verify it's a valid git worktree
                        if !branch_path.join(".git").exists() {
                            continue;
                        }

                        let branch = branch_entry.file_name().to_string_lossy().to_string();

                        let source_repo_path =
                            resolve_source_repo(&branch_path).unwrap_or_default();

                        entries.push(WorktreeEntry {
                            branch,
                            repo_name: repo_name.clone(),
                            source_repo_path,
                            worktree_path: branch_path,
                        });
                    }
                }
            }
        }

        entries.sort_by(|a, b| a.repo_name.cmp(&b.repo_name).then(a.branch.cmp(&b.branch)));
        Ok(entries)
    }

    fn repo_dir(&self, repo_name: &str) -> PathBuf {
        // repo_name is like "github.com/owner/repo" → use nested dirs
        self.base_dir.join(repo_name)
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use std::path::Path;

    use super::*;

    fn branch_exists_in_repo(repo_path: &str, branch: &str) -> bool {
        let output = Command::new("git")
            .args(["branch", "--list", branch])
            .current_dir(repo_path)
            .output()
            .unwrap();
        !String::from_utf8_lossy(&output.stdout).trim().is_empty()
    }

    fn setup_test_repo(dir: &Path) -> String {
        let repo_path = dir.join("source-repo");
        fs::create_dir_all(&repo_path).unwrap();

        Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
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

        repo_path.to_string_lossy().to_string()
    }

    #[test]
    fn new_creates_base_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("worktrees");
        assert!(!base.exists());

        let _manager = WorktreeManager::new(base.clone()).unwrap();
        assert!(base.exists());
    }

    #[test]
    fn scan_empty_base_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = WorktreeManager::new(tmp.path().join("worktrees")).unwrap();
        let entries = manager.scan().unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn scan_discovers_valid_worktrees() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("worktrees");
        let manager = WorktreeManager::new(base.clone()).unwrap();

        let repo_path = setup_test_repo(tmp.path());
        let repo = Repository {
            path: repo_path,
            name: "github.com/test/repo".to_string(),
        };

        // Create a branch first
        Command::new("git")
            .args(["branch", "feat-test"])
            .current_dir(&repo.path)
            .output()
            .unwrap();

        manager.add_worktree(&repo, "feat-test", None).unwrap();

        let entries = manager.scan().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].repo_name, "github.com/test/repo");
        assert_eq!(entries[0].branch, "feat-test");
    }

    #[test]
    fn add_and_remove_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("worktrees");
        let manager = WorktreeManager::new(base.clone()).unwrap();

        let repo_path = setup_test_repo(tmp.path());
        let repo = Repository {
            path: repo_path,
            name: "github.com/test/repo".to_string(),
        };

        Command::new("git")
            .args(["branch", "feat-add-remove"])
            .current_dir(&repo.path)
            .output()
            .unwrap();

        let entry = manager
            .add_worktree(&repo, "feat-add-remove", None)
            .unwrap();
        assert!(entry.worktree_path.exists());
        assert_eq!(entry.branch, "feat-add-remove");

        manager.remove_worktree(&entry).unwrap();
        assert!(!entry.worktree_path.exists());

        assert!(
            !branch_exists_in_repo(&repo.path, "feat-add-remove"),
            "Branch feat-add-remove should be deleted after worktree removal"
        );
    }

    #[test]
    fn remove_worktree_preserves_protected_branches() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("worktrees");
        let manager = WorktreeManager::new(base).unwrap();

        let repo_path = setup_test_repo(tmp.path());
        let repo = Repository {
            path: repo_path,
            name: "github.com/test/repo".to_string(),
        };

        Command::new("git")
            .args(["branch", "develop"])
            .current_dir(&repo.path)
            .output()
            .unwrap();

        let entry = manager.add_worktree(&repo, "develop", None).unwrap();
        manager.remove_worktree(&entry).unwrap();
        assert!(!entry.worktree_path.exists());

        assert!(
            branch_exists_in_repo(&repo.path, "develop"),
            "Protected branch develop should not be deleted"
        );
    }

    #[test]
    fn add_duplicate_worktree_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("worktrees");
        let manager = WorktreeManager::new(base).unwrap();

        let repo_path = setup_test_repo(tmp.path());
        let repo = Repository {
            path: repo_path,
            name: "github.com/test/repo".to_string(),
        };

        Command::new("git")
            .args(["branch", "feat-dup"])
            .current_dir(&repo.path)
            .output()
            .unwrap();

        manager.add_worktree(&repo, "feat-dup", None).unwrap();
        let result = manager.add_worktree(&repo, "feat-dup", None);
        assert!(result.is_err());
    }

    #[test]
    fn repo_dir_uses_nested_structure() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = WorktreeManager::new(tmp.path().to_path_buf()).unwrap();
        let dir = manager.repo_dir("github.com/owner/repo");
        assert!(dir.ends_with("github.com/owner/repo"));
    }

    #[test]
    fn add_worktree_creates_new_branch() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("worktrees");
        let manager = WorktreeManager::new(base).unwrap();

        let repo_path = setup_test_repo(tmp.path());
        let repo = Repository {
            path: repo_path,
            name: "github.com/test/repo".to_string(),
        };

        // "feat-new" does not exist yet — should be created with -b
        let entry = manager.add_worktree(&repo, "feat-new", None).unwrap();
        assert!(entry.worktree_path.exists());
        assert_eq!(entry.branch, "feat-new");

        // Verify the branch was actually created in the source repo
        assert!(
            branch_exists_in_repo(&repo.path, "feat-new"),
            "Branch feat-new should exist in source repo"
        );
    }

    #[test]
    fn scan_ignores_non_git_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("worktrees");
        let manager = WorktreeManager::new(base.clone()).unwrap();

        // Create a directory structure that looks like a worktree but isn't
        let fake = base.join("github.com/test/repo/not-a-worktree");
        fs::create_dir_all(&fake).unwrap();

        let entries = manager.scan().unwrap();
        assert!(entries.is_empty());
    }
}
