//! # Output Format Tests
//!
//! Tests that all non-interactive list outputs are plain, scriptable,
//! line-separated lists without headers or explanatory messages.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod common;

use std::fs;

use assert_cmd::Command;
use common::{create_test_item, create_test_item_with_attachments, GlobalConfigBuilder, TestEnv};
use predicates::prelude::*;
use queuestack::commands;

// =============================================================================
// Helper Functions
// =============================================================================

/// Creates a qs command configured to run in the test environment.
fn qs_cmd(env: &TestEnv) -> Command {
    let mut cmd = Command::cargo_bin("qs").unwrap();
    cmd.current_dir(env.project_dir.path());
    cmd.env("HOME", env.home_dir.path());
    cmd
}

// =============================================================================
// list --no-interactive Output Tests
// =============================================================================

#[test]
fn test_list_items_output_plain_paths() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item(&env, "260101-AAA", "First Task", "open", &[], None);
    create_test_item(&env, "260102-BBB", "Second Task", "open", &[], None);

    qs_cmd(&env)
        .args(["list", "--no-interactive"])
        .assert()
        .success()
        .stdout(predicate::str::contains("queuestack/260101-AAA-first-task.md"))
        .stdout(predicate::str::contains("queuestack/260102-BBB-second-task.md"))
        // No headers or explanatory text
        .stdout(predicate::str::contains("ID").not())
        .stdout(predicate::str::contains("Title").not())
        .stdout(predicate::str::contains("Status").not())
        .stdout(predicate::str::contains("Items:").not())
        .stdout(predicate::str::contains("Found").not());
}

#[test]
fn test_list_items_empty_no_extra_output() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    // Empty list should just print "No items found." - nothing else
    qs_cmd(&env)
        .args(["list", "--no-interactive"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^No items found\.\n$").unwrap());
}

#[test]
fn test_list_items_one_per_line() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item(&env, "260101-AAA", "Task One", "open", &[], None);
    create_test_item(&env, "260102-BBB", "Task Two", "open", &[], None);
    create_test_item(&env, "260103-CCC", "Task Three", "open", &[], None);

    let output = qs_cmd(&env)
        .args(["list", "--no-interactive"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let lines: Vec<&str> = stdout.lines().collect();

    // Should be exactly 3 lines (one per item)
    assert_eq!(lines.len(), 3, "Expected 3 lines, got: {:?}", lines);

    // Each line should be a path
    for line in &lines {
        assert!(
            line.ends_with(".md"),
            "Each line should be a .md file path, got: {}",
            line
        );
        assert!(
            !line.contains("  "),
            "Lines should not have double spaces (no table formatting): {}",
            line
        );
    }
}

// =============================================================================
// list --labels --no-interactive Output Tests
// =============================================================================

#[test]
fn test_list_labels_output_plain_format() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item(&env, "260101-AAA", "Bug Task", "open", &["bug"], None);
    create_test_item(
        &env,
        "260102-BBB",
        "Feature Task",
        "open",
        &["feature", "urgent"],
        None,
    );

    qs_cmd(&env)
        .args(["list", "--labels", "--no-interactive"])
        .assert()
        .success()
        // Should have label (count) format
        .stdout(predicate::str::contains("bug (1)"))
        .stdout(predicate::str::contains("feature (1)"))
        .stdout(predicate::str::contains("urgent (1)"))
        // No headers
        .stdout(predicate::str::contains("Labels:").not())
        .stdout(predicate::str::contains("Label").not().or(predicate::str::contains("label ("))) // Allow "label (X)" but not "Label" header
        .stdout(predicate::str::contains("Count").not())
        .stdout(predicate::str::contains("Open items").not());
}

#[test]
fn test_list_labels_empty_no_extra_output() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    // Item with no labels
    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    qs_cmd(&env)
        .args(["list", "--labels", "--no-interactive"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^No labels found\.\n$").unwrap());
}

#[test]
fn test_list_labels_one_per_line() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item(
        &env,
        "260101-AAA",
        "Task",
        "open",
        &["bug", "feature", "urgent"],
        None,
    );

    let output = qs_cmd(&env)
        .args(["list", "--labels", "--no-interactive"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 3, "Expected 3 lines (one per label)");

    // Each line should match pattern: label (count)
    for line in &lines {
        assert!(
            line.contains(" (") && line.ends_with(')'),
            "Each line should be 'label (count)', got: {}",
            line
        );
    }
}

#[test]
fn test_list_labels_includes_closed_labels_with_zero_count() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    // Create open item with one label
    create_test_item(&env, "260101-AAA", "Open Task", "open", &["active"], None);

    // Create closed item with different label
    create_test_item(
        &env,
        "260102-BBB",
        "Closed Task",
        "closed",
        &["archived"],
        None,
    );
    fs::rename(
        env.stack_path().join("260102-BBB-closed-task.md"),
        env.archive_path().join("260102-BBB-closed-task.md"),
    )
    .expect("move to archive");

    let output = qs_cmd(&env)
        .args(["list", "--labels", "--no-interactive"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Labels from closed items should be shown with (0) count
    assert!(
        stdout.contains("active (1)"),
        "Should show open label with count"
    );
    assert!(
        stdout.contains("archived (0)"),
        "Should show closed labels with (0) count"
    );
}

// =============================================================================
// list --categories --no-interactive Output Tests
// =============================================================================

#[test]
fn test_list_categories_output_plain_format() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item(&env, "260101-AAA", "Bug", "open", &[], Some("bugs"));
    create_test_item(&env, "260102-BBB", "Feature", "open", &[], Some("features"));

    qs_cmd(&env)
        .args(["list", "--categories", "--no-interactive"])
        .assert()
        .success()
        // Should have category (count) format
        .stdout(predicate::str::contains("bugs (1)"))
        .stdout(predicate::str::contains("features (1)"))
        // No headers
        .stdout(predicate::str::contains("Categories:").not())
        .stdout(predicate::str::contains("Category").not().or(predicate::str::contains("category (")))
        .stdout(predicate::str::contains("Count").not());
}

#[test]
fn test_list_categories_uncategorized_format() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    // Item without category
    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    qs_cmd(&env)
        .args(["list", "--categories", "--no-interactive"])
        .assert()
        .success()
        // Should show "Uncategorized" not "(uncategorized)"
        .stdout(predicate::str::contains("Uncategorized (1)"))
        .stdout(predicate::str::contains("(uncategorized)").not());
}

#[test]
fn test_list_categories_empty_no_extra_output() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    // No items at all
    qs_cmd(&env)
        .args(["list", "--categories", "--no-interactive"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^No categories found\.\n$").unwrap());
}

#[test]
fn test_list_categories_includes_closed_categories_with_zero_count() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    // Create open item with one category
    create_test_item(&env, "260101-AAA", "Open Task", "open", &[], Some("active"));

    // Create closed item with different category
    create_test_item(
        &env,
        "260102-BBB",
        "Closed Task",
        "closed",
        &[],
        Some("archived"),
    );
    // Move to archive preserving category structure
    let archive_category_dir = env.archive_path().join("archived");
    fs::create_dir_all(&archive_category_dir).expect("create archive category dir");
    fs::rename(
        env.stack_path().join("archived/260102-BBB-closed-task.md"),
        archive_category_dir.join("260102-BBB-closed-task.md"),
    )
    .expect("move to archive");

    let output = qs_cmd(&env)
        .args(["list", "--categories", "--no-interactive"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Categories from closed items should be shown with (0) count
    assert!(
        stdout.contains("active (1)"),
        "Should show open category with count"
    );
    assert!(
        stdout.contains("archived (0)"),
        "Should show closed categories with (0) count"
    );
}

#[test]
fn test_list_categories_one_per_line() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item(&env, "260101-AAA", "Bug", "open", &[], Some("bugs"));
    create_test_item(&env, "260102-BBB", "Feature", "open", &[], Some("features"));
    create_test_item(&env, "260103-CCC", "Uncategorized", "open", &[], None);

    let output = qs_cmd(&env)
        .args(["list", "--categories", "--no-interactive"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 3, "Expected 3 lines (one per category)");

    for line in &lines {
        assert!(
            line.contains(" (") && line.ends_with(')'),
            "Each line should be 'category (count)', got: {}",
            line
        );
    }
}

// =============================================================================
// list --attachments --id <ID> Output Tests
// =============================================================================

#[test]
fn test_list_attachments_output_plain_format() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item_with_attachments(
        &env,
        "260101-AAA",
        "Task",
        "open",
        &["1-screenshot.png", "https://example.com/doc.pdf"],
        None,
    );

    qs_cmd(&env)
        .args(["list", "--attachments", "--id", "260101"])
        .assert()
        .success()
        // Should list attachments one per line
        .stdout(predicate::str::contains("1-screenshot.png"))
        .stdout(predicate::str::contains("https://example.com/doc.pdf"))
        // No headers
        .stdout(predicate::str::contains("Attachments:").not())
        .stdout(predicate::str::contains("File").not())
        .stdout(predicate::str::contains("URL").not());
}

#[test]
fn test_list_attachments_empty_no_extra_output() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    qs_cmd(&env)
        .args(["list", "--attachments", "--id", "260101"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^No attachments\.\n$").unwrap());
}

#[test]
fn test_list_attachments_one_per_line() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item_with_attachments(
        &env,
        "260101-AAA",
        "Task",
        "open",
        &["1-file1.png", "2-file2.pdf", "https://example.com/link"],
        None,
    );

    let output = qs_cmd(&env)
        .args(["list", "--attachments", "--id", "260101"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 3, "Expected 3 lines (one per attachment)");
}

#[test]
fn test_list_attachments_requires_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    qs_cmd(&env)
        .args(["list", "--attachments"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--id"));
}

// =============================================================================
// list --meta --id <ID> Output Tests
// =============================================================================

#[test]
fn test_list_meta_output_plain_format() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item(
        &env,
        "260101-AAA",
        "Test Task",
        "open",
        &["bug", "urgent"],
        Some("bugs"),
    );

    qs_cmd(&env)
        .args(["list", "--meta", "--id", "260101"])
        .assert()
        .success()
        // Should have key: value format
        .stdout(predicate::str::contains("id: 260101-AAA"))
        .stdout(predicate::str::contains("title: Test Task"))
        .stdout(predicate::str::contains("author: Test User"))
        .stdout(predicate::str::contains("status: open"))
        .stdout(predicate::str::contains("labels: bug, urgent"))
        .stdout(predicate::str::contains("category: bugs"))
        // No table formatting or extra headers
        .stdout(predicate::str::contains("Metadata:").not())
        .stdout(predicate::str::contains("Frontmatter:").not())
        .stdout(predicate::str::contains("---").not());
}

#[test]
fn test_list_meta_empty_labels() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    let output = qs_cmd(&env)
        .args(["list", "--meta", "--id", "260101"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Labels line should not appear when empty
    assert!(
        !stdout.contains("labels:"),
        "Labels line should not appear when empty"
    );
}

#[test]
fn test_list_meta_no_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    let output = qs_cmd(&env)
        .args(["list", "--meta", "--id", "260101"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Category should not appear when None
    assert!(
        !stdout.contains("category:"),
        "Should not show category line when None"
    );
}

#[test]
fn test_list_meta_requires_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    qs_cmd(&env)
        .args(["list", "--meta"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--id"));
}

#[test]
fn test_list_meta_with_attachments() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item_with_attachments(
        &env,
        "260101-AAA",
        "Task",
        "open",
        &["1-file.png", "https://example.com"],
        None,
    );

    qs_cmd(&env)
        .args(["list", "--meta", "--id", "260101"])
        .assert()
        .success()
        .stdout(predicate::str::contains("attachments:"))
        .stdout(predicate::str::contains("- 1-file.png"))
        .stdout(predicate::str::contains("- https://example.com"));
}

// =============================================================================
// General Output Consistency Tests
// =============================================================================

#[test]
fn test_all_outputs_no_ansi_codes_when_not_tty() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item(&env, "260101-AAA", "Task", "open", &["bug"], Some("bugs"));

    // Check list items
    let output = qs_cmd(&env)
        .args(["list", "--no-interactive"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(
        !output.contains(&0x1b),
        "list output should not contain ANSI escape codes"
    );

    // Check list labels
    let output = qs_cmd(&env)
        .args(["list", "--labels", "--no-interactive"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(
        !output.contains(&0x1b),
        "list --labels output should not contain ANSI escape codes"
    );

    // Check list categories
    let output = qs_cmd(&env)
        .args(["list", "--categories", "--no-interactive"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(
        !output.contains(&0x1b),
        "list --categories output should not contain ANSI escape codes"
    );
}

#[test]
fn test_output_ends_with_newline() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item(&env, "260101-AAA", "Task", "open", &["bug"], Some("bugs"));

    // All outputs should end with a newline for proper piping
    for args in [
        vec!["list", "--no-interactive"],
        vec!["list", "--labels", "--no-interactive"],
        vec!["list", "--categories", "--no-interactive"],
        vec!["list", "--attachments", "--id", "260101"],
        vec!["list", "--meta", "--id", "260101"],
    ] {
        let output = qs_cmd(&env)
            .args(&args)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        assert!(
            output.ends_with(b"\n"),
            "{:?} output should end with newline",
            args
        );
    }
}

// =============================================================================
// --file Option Tests (Alternative to --id)
// =============================================================================

#[test]
fn test_list_attachments_with_file_option() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    let item_path = create_test_item_with_attachments(
        &env,
        "260101-AAA",
        "Task",
        "open",
        &["1-screenshot.png"],
        None,
    );

    // Use --file instead of --id
    qs_cmd(&env)
        .args([
            "list",
            "--attachments",
            "--file",
            item_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("1-screenshot.png"));
}

#[test]
fn test_list_meta_with_file_option() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    let item_path = create_test_item(
        &env,
        "260101-AAA",
        "Test Task",
        "open",
        &["bug"],
        Some("bugs"),
    );

    // Use --file instead of --id
    qs_cmd(&env)
        .args(["list", "--meta", "--file", item_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("id: 260101-AAA"))
        .stdout(predicate::str::contains("title: Test Task"))
        .stdout(predicate::str::contains("labels: bug"))
        .stdout(predicate::str::contains("category: bugs"));
}

#[test]
fn test_update_with_file_option() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    let item_path = create_test_item(&env, "260101-AAA", "Original Title", "open", &[], None);

    // Update using --file
    qs_cmd(&env)
        .args([
            "update",
            "--file",
            item_path.to_str().unwrap(),
            "--title",
            "New Title",
        ])
        .assert()
        .success();

    // Verify the update took effect
    let content = fs::read_to_string(env.stack_path().join("260101-AAA-new-title.md")).unwrap();
    assert!(content.contains("title: New Title"));
}

#[test]
fn test_close_with_file_option() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    let item_path = create_test_item(&env, "260101-AAA", "Task to Close", "open", &[], None);

    // Close using --file
    qs_cmd(&env)
        .args(["close", "--file", item_path.to_str().unwrap()])
        .assert()
        .success();

    // Verify item was moved to archive
    let archive_files: Vec<_> = fs::read_dir(env.archive_path())
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert_eq!(archive_files.len(), 1, "Item should be in archive");
}

#[test]
fn test_reopen_with_file_option() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    // Create and close an item
    create_test_item(&env, "260101-AAA", "Task to Reopen", "open", &[], None);
    qs_cmd(&env)
        .args(["close", "--id", "260101"])
        .assert()
        .success();

    // Find the archived file path
    let archived_path = env.archive_path().join("260101-AAA-task-to-reopen.md");

    // Reopen using --file
    qs_cmd(&env)
        .args(["reopen", "--file", archived_path.to_str().unwrap()])
        .assert()
        .success();

    // Verify item was moved back to stack
    let stack_files: Vec<_> = fs::read_dir(env.stack_path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();
    assert_eq!(stack_files.len(), 1, "Item should be back in stack");
}

#[test]
fn test_file_and_id_mutually_exclusive() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    let item_path = create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    // Both --id and --file should fail
    qs_cmd(&env)
        .args([
            "list",
            "--meta",
            "--id",
            "260101",
            "--file",
            item_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_file_option_relative_path() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    create_test_item(&env, "260101-AAA", "Task", "open", &["bug"], None);

    // Use relative path from project directory
    qs_cmd(&env)
        .args(["list", "--meta", "--file", "queuestack/260101-AAA-task.md"])
        .assert()
        .success()
        .stdout(predicate::str::contains("id: 260101-AAA"))
        .stdout(predicate::str::contains("labels: bug"));
}

#[test]
fn test_file_option_nonexistent_file() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init");

    qs_cmd(&env)
        .args(["list", "--meta", "--file", "queuestack/nonexistent-file.md"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("No such file")));
}
