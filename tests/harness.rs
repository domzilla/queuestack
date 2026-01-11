//! # Test Harness
//!
//! Provides utilities for integration testing qstack without affecting user configuration.
//! Uses thread-local storage instead of environment variables to avoid any interference
//! with the user's shell environment.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use tempfile::TempDir;

// Re-export from library - this is the mechanism for test isolation
use qstack::set_home_override;

/// Global lock to ensure tests run sequentially.
/// This prevents races when tests change the current directory.
static TEST_LOCK: Mutex<()> = Mutex::new(());

/// Test environment that manages temporary directories for both
/// the "home" directory (for global config) and the project directory.
pub struct TestEnv {
    /// Temporary directory simulating user's home (for ~/.qstack)
    pub home_dir: TempDir,
    /// Temporary directory for the project
    pub project_dir: TempDir,
    /// Original current directory to restore on drop
    original_cwd: PathBuf,
    /// Guard for the test lock
    #[allow(dead_code)]
    test_guard: std::sync::MutexGuard<'static, ()>,
}

impl TestEnv {
    /// Creates a new test environment with temporary directories.
    ///
    /// Uses thread-local storage to redirect global config (no env var modification).
    /// Changes to the project directory for the duration of the test.
    pub fn new() -> Self {
        // Recover from poisoned mutex (if a previous test panicked while holding the lock)
        let test_guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let home_dir = TempDir::new().expect("Failed to create temp home dir");
        let project_dir = TempDir::new().expect("Failed to create temp project dir");

        // Save original cwd
        let original_cwd = env::current_dir().expect("Failed to get current dir");

        // Set up test environment using thread-local (NOT env vars)
        set_home_override(Some(home_dir.path().to_path_buf()));
        env::set_current_dir(project_dir.path()).expect("Failed to change to project dir");

        Self {
            home_dir,
            project_dir,
            original_cwd,
            test_guard,
        }
    }

    /// Returns the path to the project directory.
    pub fn project_path(&self) -> &Path {
        self.project_dir.path()
    }

    /// Returns the path where global config would be stored.
    pub fn global_config_path(&self) -> PathBuf {
        self.home_dir.path().join(".qstack")
    }

    /// Returns the path where project config would be stored.
    pub fn project_config_path(&self) -> PathBuf {
        self.project_dir.path().join(".qstack")
    }

    /// Returns the path to the stack directory.
    pub fn stack_path(&self) -> PathBuf {
        self.project_dir.path().join("qstack")
    }

    /// Returns the path to the archive directory.
    pub fn archive_path(&self) -> PathBuf {
        self.stack_path().join("archive")
    }

    /// Creates a global config file with the given content.
    pub fn write_global_config(&self, content: &str) {
        fs::write(self.global_config_path(), content).expect("Failed to write global config");
    }

    /// Creates a project config file with the given content.
    pub fn write_project_config(&self, content: &str) {
        fs::write(self.project_config_path(), content).expect("Failed to write project config");
    }

    /// Reads the global config file content.
    pub fn read_global_config(&self) -> String {
        fs::read_to_string(self.global_config_path()).unwrap_or_default()
    }

    /// Lists all files in the stack directory (non-recursive).
    pub fn list_stack_files(&self) -> Vec<PathBuf> {
        self.list_files_in(&self.stack_path())
    }

    /// Lists all files in the archive directory.
    pub fn list_archive_files(&self) -> Vec<PathBuf> {
        self.list_files_in(&self.archive_path())
    }

    /// Lists all .md files in a directory (non-recursive).
    fn list_files_in(&self, dir: &Path) -> Vec<PathBuf> {
        if !dir.exists() {
            return Vec::new();
        }
        fs::read_dir(dir)
            .expect("Failed to read directory")
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
            .collect()
    }

    /// Lists all .md files in a category subdirectory.
    pub fn list_category_files(&self, category: &str) -> Vec<PathBuf> {
        self.list_files_in(&self.stack_path().join(category))
    }

    /// Counts total items across all locations.
    pub fn count_all_items(&self) -> usize {
        let stack_count = self.count_items_recursive(&self.stack_path());
        stack_count
    }

    /// Counts .md files recursively in a directory.
    fn count_items_recursive(&self, dir: &Path) -> usize {
        if !dir.exists() {
            return 0;
        }
        walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .count()
    }

    /// Reads an item file by its path and returns the content.
    pub fn read_item(&self, path: &Path) -> String {
        fs::read_to_string(path).expect("Failed to read item file")
    }

    /// Finds an item file by partial ID match.
    pub fn find_item_by_id(&self, partial_id: &str) -> Option<PathBuf> {
        walkdir::WalkDir::new(self.stack_path())
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .map(|e| e.into_path())
            .find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|name| name.to_lowercase().contains(&partial_id.to_lowercase()))
            })
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        // Restore original working directory first
        let _ = env::set_current_dir(&self.original_cwd);

        // Clear the thread-local home override
        set_home_override(None);
    }
}

/// Builder for creating test configurations.
pub struct GlobalConfigBuilder {
    user_name: Option<String>,
    use_git_user: bool,
    editor: Option<String>,
    interactive: bool,
    id_pattern: String,
    stack_dir: Option<String>,
    archive_dir: Option<String>,
}

impl Default for GlobalConfigBuilder {
    fn default() -> Self {
        Self {
            user_name: Some("Test User".to_string()),
            use_git_user: false,
            editor: Some("true".to_string()), // no-op editor
            interactive: true,
            id_pattern: "%y%m%d-%T%RRR".to_string(),
            stack_dir: None,
            archive_dir: None,
        }
    }
}

impl GlobalConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn user_name(mut self, name: impl Into<String>) -> Self {
        self.user_name = Some(name.into());
        self
    }

    pub fn use_git_user(mut self, use_git: bool) -> Self {
        self.use_git_user = use_git;
        self
    }

    pub fn editor(mut self, editor: impl Into<String>) -> Self {
        self.editor = Some(editor.into());
        self
    }

    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    pub fn id_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.id_pattern = pattern.into();
        self
    }

    pub fn stack_dir(mut self, dir: impl Into<String>) -> Self {
        self.stack_dir = Some(dir.into());
        self
    }

    pub fn archive_dir(mut self, dir: impl Into<String>) -> Self {
        self.archive_dir = Some(dir.into());
        self
    }

    pub fn build(&self) -> String {
        let mut lines = Vec::new();

        if let Some(ref name) = self.user_name {
            lines.push(format!("user_name = \"{}\"", name));
        }

        lines.push(format!("use_git_user = {}", self.use_git_user));

        if let Some(ref editor) = self.editor {
            lines.push(format!("editor = \"{}\"", editor));
        }

        lines.push(format!("interactive = {}", self.interactive));
        lines.push(format!("id_pattern = \"{}\"", self.id_pattern));

        if let Some(ref dir) = self.stack_dir {
            lines.push(format!("stack_dir = \"{}\"", dir));
        }

        if let Some(ref dir) = self.archive_dir {
            lines.push(format!("archive_dir = \"{}\"", dir));
        }

        lines.join("\n")
    }
}

/// Builder for creating project configurations.
///
/// All fields are optional - project config values override global config.
pub struct ProjectConfigBuilder {
    user_name: Option<String>,
    use_git_user: Option<bool>,
    editor: Option<String>,
    interactive: Option<bool>,
    id_pattern: Option<String>,
    stack_dir: Option<String>,
    archive_dir: Option<String>,
}

impl Default for ProjectConfigBuilder {
    fn default() -> Self {
        Self {
            user_name: None,
            use_git_user: None,
            editor: None,
            interactive: None,
            id_pattern: None,
            stack_dir: Some("qstack".to_string()),
            archive_dir: Some("archive".to_string()),
        }
    }
}

impl ProjectConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn user_name(mut self, name: impl Into<String>) -> Self {
        self.user_name = Some(name.into());
        self
    }

    pub fn use_git_user(mut self, use_git: bool) -> Self {
        self.use_git_user = Some(use_git);
        self
    }

    pub fn editor(mut self, editor: impl Into<String>) -> Self {
        self.editor = Some(editor.into());
        self
    }

    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = Some(interactive);
        self
    }

    pub fn id_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.id_pattern = Some(pattern.into());
        self
    }

    pub fn stack_dir(mut self, dir: impl Into<String>) -> Self {
        self.stack_dir = Some(dir.into());
        self
    }

    pub fn archive_dir(mut self, dir: impl Into<String>) -> Self {
        self.archive_dir = Some(dir.into());
        self
    }

    pub fn build(&self) -> String {
        let mut lines = Vec::new();

        if let Some(ref name) = self.user_name {
            lines.push(format!("user_name = \"{}\"", name));
        }

        if let Some(use_git) = self.use_git_user {
            lines.push(format!("use_git_user = {}", use_git));
        }

        if let Some(ref editor) = self.editor {
            lines.push(format!("editor = \"{}\"", editor));
        }

        if let Some(interactive) = self.interactive {
            lines.push(format!("interactive = {}", interactive));
        }

        if let Some(ref pattern) = self.id_pattern {
            lines.push(format!("id_pattern = \"{}\"", pattern));
        }

        if let Some(ref dir) = self.stack_dir {
            lines.push(format!("stack_dir = \"{}\"", dir));
        }

        if let Some(ref dir) = self.archive_dir {
            lines.push(format!("archive_dir = \"{}\"", dir));
        }

        lines.join("\n")
    }
}

/// Creates a minimal test item file content.
pub fn make_item_content(
    id: &str,
    title: &str,
    status: &str,
    labels: &[&str],
    category: Option<&str>,
) -> String {
    let labels_yaml = if labels.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "\n{}",
            labels
                .iter()
                .map(|l| format!("  - {}", l))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let category_yaml = match category {
        Some(cat) => format!("category: {cat}"),
        None => "category: ~".to_string(),
    };

    format!(
        r#"---
id: {id}
title: {title}
author: Test User
created_at: 2026-01-09T12:00:00Z
status: {status}
labels: {labels_yaml}
{category_yaml}
---

Test item body.
"#
    )
}

/// Creates an item file directly in the test environment.
pub fn create_test_item(
    env: &TestEnv,
    id: &str,
    title: &str,
    status: &str,
    labels: &[&str],
    category: Option<&str>,
) -> PathBuf {
    let slug = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();

    let filename = format!("{}-{}.md", id, slug);
    let content = make_item_content(id, title, status, labels, category);

    let dir = if let Some(cat) = category {
        env.stack_path().join(cat)
    } else {
        env.stack_path()
    };

    fs::create_dir_all(&dir).expect("Failed to create directory");

    let path = dir.join(filename);
    fs::write(&path, content).expect("Failed to write item file");
    path
}

// =============================================================================
// Attachment Test Helpers
// =============================================================================

impl TestEnv {
    /// Creates a test file that can be attached.
    pub fn create_test_file(&self, name: &str, content: &str) -> PathBuf {
        let path = self.project_dir.path().join(name);
        fs::write(&path, content).expect("Failed to write test file");
        path
    }

    /// Lists attachment files for an item by ID prefix (excludes archive).
    pub fn list_attachment_files(&self, item_id: &str) -> Vec<PathBuf> {
        let pattern = format!("{item_id}-Attachment-");
        let archive = self.archive_path();

        walkdir::WalkDir::new(self.stack_path())
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter(|e| !e.path().starts_with(&archive))
            .map(|e| e.into_path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|name| name.starts_with(&pattern))
            })
            .collect()
    }

    /// Checks if an attachment file exists in the given directory.
    pub fn attachment_exists(&self, item_path: &Path, attachment_name: &str) -> bool {
        item_path
            .parent()
            .map(|dir| dir.join(attachment_name).exists())
            .unwrap_or(false)
    }

    /// Lists attachment files in the archive directory for an item ID.
    pub fn list_archive_attachment_files(&self, item_id: &str) -> Vec<PathBuf> {
        let pattern = format!("{item_id}-Attachment-");

        walkdir::WalkDir::new(self.archive_path())
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|name| name.starts_with(&pattern))
            })
            .collect()
    }
}

/// Creates a test item file with attachments.
pub fn make_item_content_with_attachments(
    id: &str,
    title: &str,
    status: &str,
    labels: &[&str],
    category: Option<&str>,
    attachments: &[&str],
) -> String {
    let labels_yaml = if labels.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "\n{}",
            labels
                .iter()
                .map(|l| format!("  - {}", l))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let category_yaml = match category {
        Some(cat) => format!("category: {cat}"),
        None => "category: ~".to_string(),
    };

    let attachments_yaml = if attachments.is_empty() {
        String::new()
    } else {
        format!(
            "attachments:\n{}",
            attachments
                .iter()
                .map(|a| format!("  - {}", a))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    format!(
        r#"---
id: {id}
title: {title}
author: Test User
created_at: 2026-01-09T12:00:00Z
status: {status}
labels: {labels_yaml}
{category_yaml}
{attachments_yaml}
---

Test item body.
"#
    )
}

/// Creates a test item with pre-existing attachments.
pub fn create_test_item_with_attachments(
    env: &TestEnv,
    id: &str,
    title: &str,
    status: &str,
    attachments: &[&str],
    category: Option<&str>,
) -> PathBuf {
    let slug = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();

    let filename = format!("{}-{}.md", id, slug);
    let content = make_item_content_with_attachments(id, title, status, &[], category, attachments);

    let dir = if let Some(cat) = category {
        env.stack_path().join(cat)
    } else {
        env.stack_path()
    };

    fs::create_dir_all(&dir).expect("Failed to create directory");

    // Create the item file
    let path = dir.join(filename);
    fs::write(&path, content).expect("Failed to write item file");

    // Create the attachment files
    for attachment in attachments {
        if !attachment.starts_with("http://") && !attachment.starts_with("https://") {
            let attachment_path = dir.join(attachment);
            fs::write(&attachment_path, "test attachment content")
                .expect("Failed to write attachment");
        }
    }

    path
}
