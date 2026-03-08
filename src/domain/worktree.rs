use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::Result;
use color_eyre::eyre::eyre;

use super::pty::PtySession;
use super::repo::Repository;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeEntry {
    pub branch: String,
    pub repo_name: String,
    pub source_repo_path: String,
    pub worktree_path: PathBuf,
}

#[allow(dead_code)]
pub struct WorktreeManager {
    base_dir: PathBuf,
}

#[allow(dead_code)]
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

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn remove_worktree(&self, entry: &WorktreeEntry) -> Result<()> {
        let worktree_path_str = entry.worktree_path.to_string_lossy();
        run_git(
            &entry.source_repo_path,
            &["worktree", "remove", "--force", &worktree_path_str],
            "git worktree remove",
        )?;

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

/// Check if a branch exists in the given repository (local or remote-tracking).
pub fn branch_exists(repo_path: &str, branch: &str) -> Result<bool> {
    let stdout = git_stdout(
        repo_path,
        &["branch", "--list", "--all", branch, &format!("*/{branch}")],
    )?;
    Ok(!stdout.is_empty())
}

/// Ensure a local branch is up-to-date with its remote-tracking branch.
/// If the branch has no configured remote, this is a no-op.
pub fn ensure_branch_up_to_date(repo_path: &str, branch: &str) -> Result<()> {
    let remote = git_stdout(
        repo_path,
        &["config", "--get", &format!("branch.{branch}.remote")],
    )?;
    if remote.is_empty() {
        return Ok(());
    }

    run_git(repo_path, &["fetch", &remote, branch], "git fetch")?;

    let behind: usize = git_stdout(
        repo_path,
        &[
            "rev-list",
            "--count",
            &format!("{branch}..{remote}/{branch}"),
        ],
    )?
    .parse()
    .unwrap_or(0);
    if behind == 0 {
        return Ok(());
    }

    let target = git_stdout(repo_path, &["rev-parse", &format!("{remote}/{branch}")])?;
    let current_branch = git_stdout(repo_path, &["symbolic-ref", "--short", "HEAD"])?;

    if current_branch == branch {
        run_git(
            repo_path,
            &["merge", "--ff-only", &format!("{remote}/{branch}")],
            "git merge --ff-only",
        )?;
    } else {
        run_git(
            repo_path,
            &["update-ref", &format!("refs/heads/{branch}"), &target],
            "git update-ref",
        )?;
    }

    Ok(())
}

pub fn list_local_branches(repo_path: &str) -> Result<Vec<String>> {
    let stdout = git_stdout(
        repo_path,
        &["branch", "--list", "--format=%(refname:short)"],
    )?;
    let mut branches: Vec<String> = stdout.lines().map(ToString::to_string).collect();
    branches.sort();
    Ok(branches)
}

/// Run a git command and return its trimmed stdout. Does not check exit status.
fn git_stdout(repo_path: &str, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run a git command and return an error if it fails.
fn run_git(repo_path: &str, args: &[&str], context: &str) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!("{context} failed: {stderr}"));
    }
    Ok(())
}

#[allow(dead_code)]
fn is_dir_empty(path: &Path) -> Result<bool> {
    Ok(fs::read_dir(path)?.next().is_none())
}

/// Resolve the source (main) repository path from a worktree's `.git` file.
/// A worktree's `.git` is a file containing `gitdir: /path/to/main/.git/worktrees/<name>`.
#[allow(dead_code)]
fn resolve_source_repo(worktree_path: &Path) -> Option<String> {
    let git_path = worktree_path.join(".git");
    let content = fs::read_to_string(&git_path).ok()?;
    let gitdir = content.strip_prefix("gitdir: ")?.trim();
    // gitdir points to <main_repo>/.git/worktrees/<name>
    let path = PathBuf::from(gitdir);
    let main_git = path.ancestors().nth(2)?;
    let main_repo = main_git.parent()?;
    Some(main_repo.to_string_lossy().to_string())
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeState {
    Running,
    Stopped,
}

#[allow(dead_code, clippy::struct_field_names)]
pub struct Worktree {
    pub branch: String,
    pub pty: Option<PtySession>,
    pub qa_pty: Option<PtySession>,
    pub repo: String,
    pub source_repo_path: String,
    pub worktree_path: PathBuf,
}

impl Worktree {
    pub fn any_pty_dirty(&self) -> bool {
        let main_dirty = self.pty.as_ref().is_some_and(PtySession::is_dirty);
        let qa_dirty = self.qa_pty.as_ref().is_some_and(PtySession::is_dirty);
        main_dirty || qa_dirty
    }

    pub fn clear_pty_dirty(&self) {
        if let Some(pty) = &self.pty {
            pty.clear_dirty();
        }
        if let Some(qa) = &self.qa_pty {
            qa.clear_dirty();
        }
    }

    pub fn display_name(&self) -> &str {
        self.repo.rsplit('/').next().unwrap_or(&self.repo)
    }

    pub fn close_qa(&mut self) {
        if let Some(qa) = &mut self.qa_pty {
            qa.kill();
        }
        self.qa_pty = None;
    }

    pub fn create_qa(&mut self, fork: bool, rows: u16, cols: u16) -> Result<()> {
        let working_dir = self.working_dir();
        let qa_pty = if fork {
            PtySession::spawn_with_args("claude", &["--continue"], &working_dir, rows, cols)?
        } else {
            PtySession::spawn("claude", &working_dir, rows, cols)?
        };
        self.qa_pty = Some(qa_pty);
        Ok(())
    }

    pub fn from_entry(entry: &WorktreeEntry) -> Self {
        Self {
            branch: entry.branch.clone(),
            pty: None,
            qa_pty: None,
            repo: entry.repo_name.clone(),
            source_repo_path: entry.source_repo_path.clone(),
            worktree_path: entry.worktree_path.clone(),
        }
    }

    pub fn has_qa(&self) -> bool {
        self.qa_pty.is_some()
    }

    pub fn is_running(&self) -> bool {
        self.pty.is_some()
    }

    pub fn start(&mut self, rows: u16, cols: u16) -> Result<()> {
        if self.pty.is_some() {
            return Ok(());
        }
        let working_dir = self.working_dir();
        let pty = PtySession::spawn("claude", &working_dir, rows, cols)?;
        self.pty = Some(pty);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn state(&self) -> WorktreeState {
        if self.is_running() {
            WorktreeState::Running
        } else {
            WorktreeState::Stopped
        }
    }

    pub fn stop(&mut self) {
        self.close_qa();
        if let Some(pty) = &mut self.pty {
            pty.kill();
        }
        self.pty = None;
    }

    pub fn to_entry(&self) -> WorktreeEntry {
        WorktreeEntry {
            branch: self.branch.clone(),
            repo_name: self.repo.clone(),
            source_repo_path: self.source_repo_path.clone(),
            worktree_path: self.worktree_path.clone(),
        }
    }

    pub fn working_dir(&self) -> String {
        self.worktree_path.to_string_lossy().to_string()
    }
}

pub struct WorktreePool {
    worktrees: Vec<Worktree>,
}

impl WorktreePool {
    pub fn new() -> Self {
        Self {
            worktrees: Vec::new(),
        }
    }

    pub fn add(&mut self, wt: Worktree) {
        self.worktrees.push(wt);
    }

    pub fn all(&self) -> &[Worktree] {
        &self.worktrees
    }

    pub fn get(&self, index: usize) -> Option<&Worktree> {
        self.worktrees.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Worktree> {
        self.worktrees.get_mut(index)
    }

    pub fn is_empty(&self) -> bool {
        self.worktrees.is_empty()
    }

    pub fn len(&self) -> usize {
        self.worktrees.len()
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.worktrees.len() {
            self.worktrees[index].stop();
            self.worktrees.remove(index);
        }
    }

    #[allow(dead_code)]
    pub fn sync_with_worktrees(&mut self, entries: &[WorktreeEntry]) {
        // Keep existing worktrees that still have an entry, add new ones
        let mut new_worktrees: Vec<Worktree> = Vec::new();

        for entry in entries {
            if let Some(pos) = self
                .worktrees
                .iter()
                .position(|wt| wt.worktree_path == entry.worktree_path)
            {
                // Move the existing worktree (preserves running PTY)
                new_worktrees.push(self.worktrees.remove(pos));
            } else {
                new_worktrees.push(Worktree::from_entry(entry));
            }
        }

        // Kill remaining worktrees whose entries no longer exist
        for wt in &mut self.worktrees {
            wt.stop();
        }

        self.worktrees = new_worktrees;
    }

    #[cfg(test)]
    pub fn add_stopped(&mut self, repo: &str, branch: &str, worktree_path: &str) {
        self.worktrees.push(Worktree {
            branch: branch.to_string(),
            pty: None,
            qa_pty: None,
            repo: repo.to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from(worktree_path),
        });
    }

    #[cfg(test)]
    pub fn add_test(&mut self, repo: &str, branch: &str, worktree_path: &str) -> Result<()> {
        let pty = PtySession::spawn("cat", worktree_path, 24, 80)?;
        self.worktrees.push(Worktree {
            branch: branch.to_string(),
            pty: Some(pty),
            qa_pty: None,
            repo: repo.to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from(worktree_path),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

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
        let output = Command::new("git")
            .args(["branch", "--list", "feat-new"])
            .current_dir(&repo.path)
            .output()
            .unwrap();
        assert!(
            !String::from_utf8_lossy(&output.stdout).trim().is_empty(),
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

    fn create_test_worktree(pool: &mut WorktreePool) {
        pool.add_test("test/repo", "main", "/tmp").unwrap();
    }

    #[test]
    fn new_pool_is_empty() {
        let pool = WorktreePool::new();
        assert_eq!(pool.len(), 0);
        assert!(pool.is_empty());
    }

    #[test]
    fn add_test_increases_len() {
        let mut pool = WorktreePool::new();
        create_test_worktree(&mut pool);
        assert_eq!(pool.len(), 1);
        assert!(!pool.is_empty());
    }

    #[test]
    fn create_multiple_worktrees() {
        let mut pool = WorktreePool::new();
        create_test_worktree(&mut pool);
        create_test_worktree(&mut pool);
        create_test_worktree(&mut pool);
        assert_eq!(pool.len(), 3);
    }

    #[test]
    fn remove_decreases_len() {
        let mut pool = WorktreePool::new();
        create_test_worktree(&mut pool);
        create_test_worktree(&mut pool);
        pool.remove(0);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn remove_out_of_bounds_is_safe() {
        let mut pool = WorktreePool::new();
        create_test_worktree(&mut pool);
        pool.remove(99);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn get_returns_worktree() {
        let mut pool = WorktreePool::new();
        create_test_worktree(&mut pool);
        let wt = pool.get(0);
        assert!(wt.is_some());
        assert_eq!(wt.unwrap().repo, "test/repo");
    }

    #[test]
    fn get_out_of_bounds_returns_none() {
        let pool = WorktreePool::new();
        assert!(pool.get(99).is_none());
    }

    #[test]
    fn display_name_returns_last_component() {
        let mut pool = WorktreePool::new();
        pool.add_stopped("github.com/owner/myrepo", "main", "/tmp");
        let wt = pool.get(0).unwrap();
        assert_eq!(wt.display_name(), "myrepo");
    }

    #[test]
    fn display_name_without_slash() {
        let mut pool = WorktreePool::new();
        pool.add_stopped("myrepo", "main", "/tmp");
        let wt = pool.get(0).unwrap();
        assert_eq!(wt.display_name(), "myrepo");
    }

    #[test]
    fn worktree_state_stopped_by_default() {
        let mut pool = WorktreePool::new();
        pool.add_stopped("test/repo", "main", "/tmp");
        let wt = pool.get(0).unwrap();
        assert_eq!(wt.state(), WorktreeState::Stopped);
        assert!(!wt.is_running());
    }

    #[test]
    fn worktree_state_running_with_pty() {
        let mut pool = WorktreePool::new();
        create_test_worktree(&mut pool);
        let wt = pool.get(0).unwrap();
        assert_eq!(wt.state(), WorktreeState::Running);
        assert!(wt.is_running());
    }

    #[test]
    fn stop_kills_pty() {
        let mut pool = WorktreePool::new();
        create_test_worktree(&mut pool);
        let wt = pool.get_mut(0).unwrap();
        assert!(wt.is_running());
        wt.stop();
        assert!(!wt.is_running());
    }

    #[test]
    fn sync_with_worktrees_adds_new_entries() {
        let mut pool = WorktreePool::new();
        let entries = vec![WorktreeEntry {
            branch: "main".to_string(),
            repo_name: "test/repo".to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from("/tmp/wt1"),
        }];
        pool.sync_with_worktrees(&entries);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(0).unwrap().branch, "main");
    }

    #[test]
    fn sync_with_worktrees_preserves_existing() {
        let mut pool = WorktreePool::new();
        pool.add_test("test/repo", "main", "/tmp").unwrap();

        let entries = vec![WorktreeEntry {
            branch: "main".to_string(),
            repo_name: "test/repo".to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from("/tmp"),
        }];
        pool.sync_with_worktrees(&entries);
        assert_eq!(pool.len(), 1);
        assert!(pool.get(0).unwrap().is_running()); // PTY preserved
    }

    #[test]
    fn sync_with_worktrees_removes_stale() {
        let mut pool = WorktreePool::new();
        pool.add_stopped("test/repo", "old-branch", "/tmp/old");

        let entries = vec![WorktreeEntry {
            branch: "new-branch".to_string(),
            repo_name: "test/repo".to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from("/tmp/new"),
        }];
        pool.sync_with_worktrees(&entries);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(0).unwrap().branch, "new-branch");
    }
}
