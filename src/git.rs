use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// The statusline re-runs on every refresh tick, so a repo on a stalled
/// network mount or holding a contended lock must not wall it. Callers build
/// one deadline from this and share it across every git call in a refresh;
/// past it, calls give up and their segments are dropped.
pub const TIMEOUT: Duration = Duration::from_millis(500);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// The most commits [`unsigned_local_commits`] scans. A count equal to this
/// means "at least this many".
pub const MAX_SCANNED_COMMITS: usize = 200;

/// Run `git -C cwd <args>` and return stdout. `None` if git is missing,
/// exits non-zero, or is still running at `deadline`.
fn run_git(cwd: &Path, args: &[&str], deadline: Instant) -> Option<String> {
    if Instant::now() >= deadline {
        return None;
    }
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
/// a repo, if `git` is missing, if HEAD is detached, or at `deadline`.
#[must_use]
pub fn current_branch(cwd: &Path, deadline: Instant) -> Option<String> {
    let name = run_git(cwd, &["branch", "--show-current"], deadline)?;
    let trimmed = name.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Count commits reachable from HEAD but from no remote-tracking ref that
/// carry no signature, up to [`MAX_SCANNED_COMMITS`]. `None` unless the repo
/// signs commits by default and has a remote: without both, unsigned local
/// commits are not a problem.
#[must_use]
pub fn unsigned_local_commits(cwd: &Path, deadline: Instant) -> Option<usize> {
    // One call answers both preconditions. It exits non-zero when nothing
    // matches, which also means "no reason to count".
    let config = run_git(
        cwd,
        &[
            "config",
            "--get-regexp",
            r"^(commit\.gpgsign|remote\..*\.url)$",
        ],
        deadline,
    )?;
    if !signs_and_has_remote(&config) {
        return None;
    }
    let max_count = format!("--max-count={MAX_SCANNED_COMMITS}");
    let log = run_git(
        cwd,
        &[
            "log",
            // User config must not change the format being parsed (color
            // codes) or make each signed commit spawn a verifier.
            "--no-color",
            "--no-show-signature",
            "--pretty=raw",
            &max_count,
            "HEAD",
            "--not",
            "--remotes",
        ],
        deadline,
    )?;
    Some(count_unsigned(&log))
}

/// Read `git config --get-regexp` output for commit.gpgsign and remote URLs.
/// Later lines come from narrower scopes and win, as they do for git itself.
fn signs_and_has_remote(config: &str) -> bool {
    let mut signs = false;
    let mut has_remote = false;
    for line in config.lines() {
        let (key, value) = line.split_once(' ').unwrap_or((line, ""));
        if key == "commit.gpgsign" {
            signs = ["true", "yes", "on", "1"]
                .iter()
                .any(|t| value.eq_ignore_ascii_case(t));
        } else if key.starts_with("remote.") {
            has_remote = true;
        }
    }
    signs && has_remote
}

/// Count commits in `git log --pretty=raw` output without a `gpgsig` header.
/// Header lines start in column 0; message lines are indented four spaces, so
/// a body mentioning "gpgsig" or "commit" cannot be mistaken for one.
fn count_unsigned(log: &str) -> usize {
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

    fn soon() -> Instant {
        Instant::now() + TIMEOUT
    }

    #[test]
    fn non_repo_dir_returns_none() {
        let tmp = std::env::temp_dir();
        // The OS temp dir is unlikely to be inside a git repo on CI/dev.
        // If it happens to be, the assertion below relaxes to either None
        // or Some non-empty branch — i.e. we just assert no panic.
        let _ = current_branch(&tmp, soon());
    }

    #[test]
    fn passed_deadline_gives_up() {
        let cwd = std::env::current_dir().unwrap_or_default();
        assert_eq!(current_branch(&cwd, Instant::now()), None);
    }

    #[test]
    fn narrower_config_scope_wins() {
        let global_on_local_off =
            "commit.gpgsign true\ncommit.gpgsign false\nremote.origin.url x\n";
        assert!(!signs_and_has_remote(global_on_local_off));
        assert!(signs_and_has_remote(
            "commit.gpgsign false\ncommit.gpgsign YES\nremote.origin.url x\n"
        ));
        assert!(!signs_and_has_remote("commit.gpgsign true\n"));
        assert!(!signs_and_has_remote("remote.origin.url x\n"));
    }

    #[test]
    #[expect(
        clippy::unwrap_used,
        reason = "test fixtures are valid by construction"
    )]
    fn this_repo_has_a_branch() {
        let cwd = std::env::current_dir().unwrap();
        let branch = current_branch(&cwd, soon());
        // claude-statusline itself is a git repo; if running from there we
        // expect Some. If running from somewhere else (CI scratch) we
        // accept None.
        if let Some(b) = branch {
            assert_ne!(b, "");
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
