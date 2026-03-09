use color_eyre::Result;
use color_eyre::eyre::eyre;

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

/// Run a git command and return its trimmed stdout. Does not check exit status.
pub fn git_stdout(repo_path: &str, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn is_dir_empty(path: &std::path::Path) -> Result<bool> {
    Ok(std::fs::read_dir(path)?.next().is_none())
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

/// Resolve the source (main) repository path from a worktree's `.git` file.
/// A worktree's `.git` is a file containing `gitdir: /path/to/main/.git/worktrees/<name>`.
pub fn resolve_source_repo(worktree_path: &std::path::Path) -> Option<String> {
    let git_path = worktree_path.join(".git");
    let content = std::fs::read_to_string(&git_path).ok()?;
    let gitdir = content.strip_prefix("gitdir: ")?.trim();
    // gitdir points to <main_repo>/.git/worktrees/<name>
    let path = std::path::PathBuf::from(gitdir);
    let main_git = path.ancestors().nth(2)?;
    let main_repo = main_git.parent()?;
    Some(main_repo.to_string_lossy().to_string())
}

/// Run a git command and return an error if it fails.
pub fn run_git(repo_path: &str, args: &[&str], context: &str) -> Result<()> {
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
