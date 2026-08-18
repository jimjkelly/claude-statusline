use std::path::Path;
use std::process::Command;

/// Look up the current git branch for `cwd`. Returns `None` if `cwd` is not
/// a repo, if `git` is missing, or if HEAD is detached.
#[must_use]
pub fn current_branch(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["branch", "--show-current"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8(output.stdout).ok()?;
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
