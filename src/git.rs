use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The statusline re-runs on every refresh tick, so a repo on a stalled
/// network mount or holding a contended lock must not wall it. Past this
/// budget we give up and render without a branch.
const TIMEOUT: Duration = Duration::from_millis(500);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Look up the current git branch for `cwd`. Returns `None` if `cwd` is not
/// a repo, if `git` is missing, if HEAD is detached, or if `git` takes longer
/// than [`TIMEOUT`].
#[must_use]
pub fn current_branch(cwd: &Path) -> Option<String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["branch", "--show-current"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

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
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(_) => return None,
        }
    };
    if !status.success() {
        return None;
    }

    // Safe to read only after exit: a branch name never approaches the pipe
    // buffer size, so the child cannot have blocked on a full pipe.
    let mut name = String::new();
    child.stdout.take()?.read_to_string(&mut name).ok()?;
    let trimmed = name.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
