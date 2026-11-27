use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use claude_statusline::git::{unsigned_local_commits, TIMEOUT};

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

/// A scratch repository with two unsigned commits, removed on drop.
struct Repo(PathBuf);

impl Repo {
    #[expect(clippy::unwrap_used, reason = "temp dir setup fails loudly by design")]
    fn with_two_commits(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("claude-statusline-{name}-{}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        git(&dir, &["commit", "-q", "--allow-empty", "-m", "one"]);
        git(&dir, &["commit", "-q", "--allow-empty", "-m", "two"]);
        Self(dir)
    }

    fn git(&self, args: &[&str]) {
        git(&self.0, args);
    }

    fn unsigned(&self) -> Option<usize> {
        unsigned_local_commits(&self.0, Instant::now() + TIMEOUT)
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn signing_off_reports_nothing() {
    let repo = Repo::with_two_commits("off");
    repo.git(&["config", "commit.gpgsign", "false"]);
    repo.git(&["remote", "add", "origin", "/nonexistent.git"]);
    assert_eq!(repo.unsigned(), None);
}

#[test]
fn no_remote_reports_nothing() {
    let repo = Repo::with_two_commits("noremote");
    repo.git(&["config", "commit.gpgsign", "true"]);
    assert_eq!(repo.unsigned(), None);
}

#[test]
fn counts_only_commits_on_no_remote() {
    let repo = Repo::with_two_commits("count");
    repo.git(&["config", "commit.gpgsign", "true"]);
    repo.git(&["remote", "add", "origin", "/nonexistent.git"]);
    assert_eq!(repo.unsigned(), Some(2));
    repo.git(&["update-ref", "refs/remotes/origin/main", "HEAD~1"]);
    assert_eq!(repo.unsigned(), Some(1));
}

#[test]
fn user_log_settings_do_not_change_the_count() {
    let repo = Repo::with_two_commits("logconfig");
    repo.git(&["config", "commit.gpgsign", "true"]);
    repo.git(&["remote", "add", "origin", "/nonexistent.git"]);
    repo.git(&["config", "color.ui", "always"]);
    repo.git(&["config", "log.showSignature", "true"]);
    assert_eq!(repo.unsigned(), Some(2));
}
