//! # Git Integration
//!
//! Detects git repositories and provides git-aware file operations.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};

/// Gets the user name from git config (`git config user.name`).
pub fn user_name() -> Option<String> {
    Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|name| !name.is_empty())
}

/// Checks if the current directory is inside a git repository.
pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Checks if a file is tracked by git.
///
/// Returns `false` if the file is untracked, ignored (via .gitignore or global gitignore),
/// or if we're not in a git repository.
fn is_tracked(path: &Path) -> bool {
    Command::new("git")
        .args(["ls-files", "--error-unmatch", &path.to_string_lossy()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Moves a file, using `git mv` if tracked, otherwise standard rename.
pub fn move_file(from: &Path, to: &Path) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    if is_git_repo() && is_tracked(from) {
        // File is tracked by git - use git mv
        let status = Command::new("git")
            .args(["mv", &from.to_string_lossy(), &to.to_string_lossy()])
            .status()
            .context("Failed to execute git mv")?;

        if !status.success() {
            // git mv failed unexpectedly - fall back to standard rename
            std::fs::rename(from, to).with_context(|| {
                format!("Failed to move {} to {}", from.display(), to.display())
            })?;
        }
    } else {
        // Not in git repo or file not tracked - use standard rename
        std::fs::rename(from, to)
            .with_context(|| format!("Failed to move {} to {}", from.display(), to.display()))?;
    }

    Ok(())
}

/// Removes a file, using `git rm` if tracked, otherwise standard remove.
pub fn remove_file(path: &Path) -> Result<()> {
    if is_git_repo() && is_tracked(path) {
        // File is tracked by git - use git rm
        let status = Command::new("git")
            .args(["rm", "-f", &path.to_string_lossy()])
            .status()
            .context("Failed to execute git rm")?;

        if !status.success() {
            // git rm failed unexpectedly - fall back to standard remove
            std::fs::remove_file(path)
                .with_context(|| format!("Failed to remove {}", path.display()))?;
        }
    } else {
        // Not in git repo or file not tracked - use standard remove
        std::fs::remove_file(path)
            .with_context(|| format!("Failed to remove {}", path.display()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_move_file_no_git() {
        let dir = tempdir().unwrap();
        let from = dir.path().join("source.txt");
        let to = dir.path().join("dest.txt");

        fs::write(&from, "content").unwrap();

        // This test runs outside git, so it uses standard rename
        move_file(&from, &to).unwrap();

        assert!(!from.exists());
        assert!(to.exists());
        assert_eq!(fs::read_to_string(&to).unwrap(), "content");
    }

    #[test]
    fn test_move_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let from = dir.path().join("source.txt");
        let to = dir.path().join("subdir/nested/dest.txt");

        fs::write(&from, "content").unwrap();

        move_file(&from, &to).unwrap();

        assert!(!from.exists());
        assert!(to.exists());
    }
}
