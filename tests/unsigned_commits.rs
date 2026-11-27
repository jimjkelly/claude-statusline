use std::path::{Path, PathBuf};
use std::process::Command;

use claude_statusline::git::unsigned_local_commits;

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=test", "-c", "user.email=test@example.com"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .status()
        .is_ok_and(|s| s.success());
    assert!(ok, "git {args:?} failed in {}", dir.display());
}

#[expect(clippy::unwrap_used, reason = "temp dir setup fails loudly by design")]
fn repo_with_two_commits(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("claude-statusline-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    git(&dir, &["commit", "-q", "--allow-empty", "-m", "one"]);
    git(&dir, &["commit", "-q", "--allow-empty", "-m", "two"]);
    dir
}

#[test]
fn signing_off_reports_nothing() {
    let dir = repo_with_two_commits("off");
    git(&dir, &["config", "commit.gpgsign", "false"]);
    git(&dir, &["remote", "add", "origin", "/nonexistent.git"]);
    assert_eq!(unsigned_local_commits(&dir), None);
}

#[test]
fn no_remote_reports_nothing() {
    let dir = repo_with_two_commits("noremote");
    git(&dir, &["config", "commit.gpgsign", "true"]);
    assert_eq!(unsigned_local_commits(&dir), None);
}

#[test]
fn counts_only_commits_on_no_remote() {
    let dir = repo_with_two_commits("count");
    git(&dir, &["config", "commit.gpgsign", "true"]);
    git(&dir, &["remote", "add", "origin", "/nonexistent.git"]);
    assert_eq!(unsigned_local_commits(&dir), Some(2));
    git(&dir, &["update-ref", "refs/remotes/origin/main", "HEAD~1"]);
    assert_eq!(unsigned_local_commits(&dir), Some(1));
}
