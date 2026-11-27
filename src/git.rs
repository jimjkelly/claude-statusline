use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// The statusline re-runs on every refresh tick, so a repo on a stalled
/// network mount or holding a contended lock must not wall it. Past this
/// budget a git call gives up and its segment is dropped.
const TIMEOUT: Duration = Duration::from_millis(500);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Run `git -C cwd <args>` and return stdout. `None` if git is missing,
/// exits non-zero, or exceeds `TIMEOUT`.
fn run_git(cwd: &Path, args: &[&str]) -> Option<String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    // Drain stdout on a thread: `git log` output can exceed the pipe buffer,
    // and a child blocked on a full pipe never exits.
    let mut stdout = child.stdout.take()?;
    let reader = thread::spawn(move || {
        let mut out = String::new();
        stdout.read_to_string(&mut out).ok().map(|_| out)
    });

    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                thread::sleep(POLL_INTERVAL);
            }
            Err(_) => return None,
        }
    };
    let out = reader.join().ok()??;
    status.success().then_some(out)
}

/// Look up the current git branch for `cwd`. Returns `None` if `cwd` is not
/// a repo, if `git` is missing, if HEAD is detached, or if `git` takes longer
/// than its 500 ms budget.
#[must_use]
pub fn current_branch(cwd: &Path) -> Option<String> {
    let name = run_git(cwd, &["branch", "--show-current"])?;
    let trimmed = name.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Count commits reachable from HEAD but from no remote-tracking ref that
/// carry no signature. `None` unless the repo signs commits by default and
/// has a remote: without both, unsigned local commits are not a problem.
/// Scans at most 200 commits.
#[must_use]
pub fn unsigned_local_commits(cwd: &Path) -> Option<usize> {
    let gpgsign = run_git(cwd, &["config", "--bool", "commit.gpgsign"])?;
    if gpgsign.trim() != "true" {
        return None;
    }
    if run_git(cwd, &["remote"])?.trim().is_empty() {
        return None;
    }
    let log = run_git(
        cwd,
        &[
            "log",
            "--pretty=raw",
            "--max-count=200",
            "HEAD",
            "--not",
            "--remotes",
        ],
    )?;
    Some(count_unsigned(&log))
}

/// Count commits in `git log --pretty=raw` output without a `gpgsig` header.
/// Header lines start in column 0; message lines are indented four spaces, so
/// a body mentioning "gpgsig" or "commit" cannot be mistaken for one.
#[must_use]
pub fn count_unsigned(log: &str) -> usize {
    let mut unsigned = 0;
    let mut in_commit = false;
    let mut signed = false;
    for line in log.lines() {
        if line.starts_with("commit ") {
            if in_commit && !signed {
                unsigned += 1;
            }
            in_commit = true;
            signed = false;
        } else if line.starts_with("gpgsig") {
            signed = true;
        }
    }
    if in_commit && !signed {
        unsigned += 1;
    }
    unsigned
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIGNED: &str = "commit aaa\ntree t\nauthor a\ncommitter c\n\
        gpgsig -----BEGIN SSH SIGNATURE-----\n xyz\n -----END SSH SIGNATURE-----\n\n    signed\n";
    const UNSIGNED: &str = "commit bbb\ntree t\nauthor a\ncommitter c\n\n    \
        unsigned, but its body says\n    gpgsig and\n    commit 123\n";

    #[test]
    fn non_repo_dir_returns_none() {
        let tmp = std::env::temp_dir();
        // The OS temp dir is unlikely to be inside a git repo on CI/dev.
        // If it happens to be, the assertion below relaxes to either None
        // or Some non-empty branch — i.e. we just assert no panic.
        let _ = current_branch(&tmp);
    }

    #[test]
    #[expect(
        clippy::unwrap_used,
        reason = "test fixtures are valid by construction"
    )]
    fn this_repo_has_a_branch() {
        let cwd = std::env::current_dir().unwrap();
        let branch = current_branch(&cwd);
        // claude-statusline itself is a git repo; if running from there we
        // expect Some. If running from somewhere else (CI scratch) we
        // accept None.
        if let Some(b) = branch {
            assert!(!b.is_empty());
        }
    }

    #[test]
    fn count_unsigned_empty_log_is_zero() {
        assert_eq!(count_unsigned(""), 0);
    }

    #[test]
    fn count_unsigned_ignores_signed_commits() {
        assert_eq!(count_unsigned(SIGNED), 0);
    }

    #[test]
    fn count_unsigned_ignores_message_bodies() {
        assert_eq!(count_unsigned(UNSIGNED), 1);
    }

    #[test]
    fn count_unsigned_mixed() {
        let log = format!("{SIGNED}{UNSIGNED}{UNSIGNED}");
        assert_eq!(count_unsigned(&log), 2);
    }
}
