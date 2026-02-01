//! # Test Harness
//!
//! Provides utilities for integration testing queuestack without affecting user configuration.
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
use queuestack::set_home_override;

/// Global lock to ensure tests run sequentially.
/// This prevents races when tests change the current directory.
static TEST_LOCK: Mutex<()> = Mutex::new(());

/// Test environment that manages temporary directories for both
/// the "home" directory (for global config) and the project directory.
pub struct TestEnv {
    /// Temporary directory simulating user's home (for ~/.config/queuestack/config)
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
        self.home_dir
            .path()
            .join(".config")
            .join("queuestack")
            .join("config")
    }

    /// Returns the path where project config would be stored.
    pub fn project_config_path(&self) -> PathBuf {
        self.project_dir.path().join(".queuestack")
    }

    /// Returns the path to the stack directory.
    pub fn stack_path(&self) -> PathBuf {
        self.project_dir.path().join("queuestack")
    }

    /// Returns the path to the archive directory.
    pub fn archive_path(&self) -> PathBuf {
        self.stack_path().join(".archive")
    }

    /// Returns the path to the template directory.
    pub fn template_path(&self) -> PathBuf {
        self.stack_path().join(".templates")
    }

    /// Creates a global config file with the given content.
    pub fn write_global_config(&self, content: &str) {
        let path = self.global_config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create global config directory");
        }
        fs::write(path, content).expect("Failed to write global config");
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

    /// Lists all files in the archive directory (recursive, including categories).
    pub fn list_archive_files(&self) -> Vec<PathBuf> {
        self.list_files_recursive(&self.archive_path())
    }

    /// Lists all files in the template directory (recursive, including categories).
    pub fn list_template_files(&self) -> Vec<PathBuf> {
        self.list_files_recursive(&self.template_path())
    }

    /// Lists all .md files in a directory recursively.
    fn list_files_recursive(&self, dir: &Path) -> Vec<PathBuf> {
        if !dir.exists() {
            return Vec::new();
        }
        walkdir::WalkDir::new(dir)
            .min_depth(1)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .map(|e| e.into_path())
            .collect()
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

// =============================================================================
// Test Setup Helpers
// =============================================================================

/// Creates a fully initialized test environment with default global config.
///
/// Equivalent to:
/// ```
/// let env = TestEnv::new();
/// env.write_global_config(&GlobalConfigBuilder::new().build());
/// queuestack::commands::init().expect("init should succeed");
/// ```
pub fn setup_test_env() -> TestEnv {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    queuestack::commands::init().expect("init should succeed");
    env
}

/// Creates a fully initialized test environment with non-interactive mode.
///
/// Useful for tests that need to avoid interactive prompts.
pub fn setup_test_env_non_interactive() -> TestEnv {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    queuestack::commands::init().expect("init should succeed");
    env
}

// =============================================================================
// Config Builder Helpers
// =============================================================================

/// Helper to build TOML config lines from optional values.
struct ConfigLines(Vec<String>);

impl ConfigLines {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn add_string(&mut self, key: &str, value: Option<&str>) {
        if let Some(v) = value {
            self.0.push(format!("{key} = \"{v}\""));
        }
    }

    fn add_bool(&mut self, key: &str, value: Option<bool>) {
        if let Some(v) = value {
            self.0.push(format!("{key} = {v}"));
        }
    }

    fn build(self) -> String {
        self.0.join("\n")
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
        let mut lines = ConfigLines::new();
        lines.add_string("user_name", self.user_name.as_deref());
        lines.add_bool("use_git_user", Some(self.use_git_user));
        lines.add_string("editor", self.editor.as_deref());
        lines.add_bool("interactive", Some(self.interactive));
        lines.add_string("id_pattern", Some(&self.id_pattern));
        lines.add_string("stack_dir", self.stack_dir.as_deref());
        lines.add_string("archive_dir", self.archive_dir.as_deref());
        lines.build()
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
            stack_dir: Some("queuestack".to_string()),
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

    #[allow(dead_code)]
    pub fn use_git_user(mut self, use_git: bool) -> Self {
        self.use_git_user = Some(use_git);
        self
    }

    #[allow(dead_code)]
    pub fn editor(mut self, editor: impl Into<String>) -> Self {
        self.editor = Some(editor.into());
        self
    }

    #[allow(dead_code)]
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
        let mut lines = ConfigLines::new();
        lines.add_string("user_name", self.user_name.as_deref());
        lines.add_bool("use_git_user", self.use_git_user);
        lines.add_string("editor", self.editor.as_deref());
        lines.add_bool("interactive", self.interactive);
        lines.add_string("id_pattern", self.id_pattern.as_deref());
        lines.add_string("stack_dir", self.stack_dir.as_deref());
        lines.add_string("archive_dir", self.archive_dir.as_deref());
        lines.build()
    }
}

/// Creates test item file content with optional attachments.
///
/// Use `None` for attachments when not needed, or `Some(&[...])` to include them.
pub fn make_item_content(
    id: &str,
    title: &str,
    status: &str,
    labels: &[&str],
    category: Option<&str>,
    attachments: Option<&[&str]>,
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

    let attachments_yaml = match attachments {
        Some(att) if !att.is_empty() => format!(
            "attachments:\n{}",
            att.iter()
                .map(|a| format!("  - {}", a))
                .collect::<Vec<_>>()
                .join("\n")
        ),
        _ => String::new(),
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
    let content = make_item_content(id, title, status, labels, category, None);

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

    /// Returns the attachment directory path for an item file.
    fn attachment_dir_for_item(item_path: &Path) -> PathBuf {
        let stem = item_path.file_stem().unwrap_or_default();
        item_path.with_file_name(format!("{}.attachments", stem.to_string_lossy()))
    }

    /// Lists attachment files in the `.attachments/` directory for an item.
    fn list_attachment_files_for_item(&self, item_path: &Path) -> Vec<PathBuf> {
        let attachment_dir = Self::attachment_dir_for_item(item_path);
        if !attachment_dir.exists() {
            return Vec::new();
        }

        fs::read_dir(&attachment_dir)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect()
    }

    /// Lists attachment files for an item by ID prefix (excludes archive).
    pub fn list_attachment_files(&self, item_id: &str) -> Vec<PathBuf> {
        // Find the item file first (excluding archive)
        let archive = self.archive_path();
        let item_path = walkdir::WalkDir::new(self.stack_path())
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .map(|e| e.into_path())
            .filter(|p| !p.starts_with(&archive)) // Exclude archive
            .find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|name| name.to_lowercase().contains(&item_id.to_lowercase()))
            });

        item_path
            .map(|p| self.list_attachment_files_for_item(&p))
            .unwrap_or_default()
    }

    /// Checks if an attachment file exists in the item's attachment directory.
    #[allow(dead_code)]
    pub fn attachment_exists(&self, item_path: &Path, attachment_name: &str) -> bool {
        let attachment_dir = Self::attachment_dir_for_item(item_path);
        attachment_dir.join(attachment_name).exists()
    }

    /// Lists attachment files in the archive directory for an item ID.
    pub fn list_archive_attachment_files(&self, item_id: &str) -> Vec<PathBuf> {
        // Find the item in archive
        let archive_items = self.list_archive_files();
        for item_path in archive_items {
            if item_path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.to_lowercase().contains(&item_id.to_lowercase()))
            {
                return self.list_attachment_files_for_item(&item_path);
            }
        }
        Vec::new()
    }
}

/// Creates a test item with pre-existing attachments.
///
/// Attachments are stored in a sibling `.attachments/` directory.
/// Attachment names in the list should use the new format: `{counter}-{name}.{ext}`
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
    let content = make_item_content(id, title, status, &[], category, Some(attachments));

    let dir = if let Some(cat) = category {
        env.stack_path().join(cat)
    } else {
        env.stack_path()
    };

    fs::create_dir_all(&dir).expect("Failed to create directory");

    // Create the item file
    let item_path = dir.join(&filename);
    fs::write(&item_path, content).expect("Failed to write item file");

    // Create the attachment directory and files
    let file_attachments: Vec<_> = attachments
        .iter()
        .filter(|a| !a.starts_with("http://") && !a.starts_with("https://"))
        .collect();

    if !file_attachments.is_empty() {
        let stem = filename.strip_suffix(".md").unwrap_or(&filename);
        let attachment_dir = dir.join(format!("{stem}.attachments"));
        fs::create_dir_all(&attachment_dir).expect("Failed to create attachment directory");

        for attachment in file_attachments {
            let attachment_path = attachment_dir.join(attachment);
            fs::write(&attachment_path, "test attachment content")
                .expect("Failed to write attachment");
        }
    }

    item_path
}
