//! # Close/Reopen Command Tests
//!
//! Tests for the `qs close` and `qs reopen` commands.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod common;

use common::{create_test_item, create_test_item_with_attachments, GlobalConfigBuilder, TestEnv};
use queuestack::commands::{self, execute_close, execute_reopen};

// =============================================================================
// Close Command Tests
// =============================================================================

#[test]
fn test_close_moves_to_archive() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task to Close", "open", &[], None);

    execute_close(Some("260101".to_string()), None).expect("close should succeed");

    let stack_files = env.list_stack_files();
    assert!(stack_files.is_empty(), "Stack should be empty");

    let archive_files = env.list_archive_files();
    assert_eq!(archive_files.len(), 1, "Archive should have one item");

    // Check status was updated
    let content = env.read_item(&archive_files[0]);
    assert!(
        content.contains("status: closed"),
        "Status should be closed"
    );
}

#[test]
fn test_close_item_with_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Bug Task", "open", &[], Some("bugs"));

    execute_close(Some("260101".to_string()), None).expect("close should succeed");

    let category_files = env.list_category_files("bugs");
    assert!(category_files.is_empty(), "Category should be empty");

    let archive_files = env.list_archive_files();
    assert_eq!(archive_files.len(), 1, "Archive should have one item");
}

#[test]
fn test_close_nonexistent_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let result = execute_close(Some("999999".to_string()), None);
    assert!(result.is_err(), "close with nonexistent ID should fail");
}

#[test]
fn test_close_already_closed() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);
    execute_close(Some("260101".to_string()), None).expect("first close should succeed");

    // Try to close again
    let result = execute_close(Some("260101".to_string()), None);
    assert!(result.is_err(), "closing already closed item should fail");
}

#[test]
fn test_close_with_partial_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-ABCDEFG", "Task", "open", &[], None);

    // Close with minimal partial ID
    let result = execute_close(Some("2601".to_string()), None);
    assert!(result.is_ok(), "close with partial ID should succeed");

    let archive_files = env.list_archive_files();
    assert_eq!(archive_files.len(), 1, "Item should be in archive");
}

#[test]
fn test_close_nonexistent_item() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let result = execute_close(Some("nonexistent".to_string()), None);
    assert!(result.is_err(), "close nonexistent item should fail");
}

#[test]
fn test_close_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    // Don't call init

    let result = execute_close(Some("260101".to_string()), None);
    assert!(result.is_err(), "close without init should fail");
}

// =============================================================================
// Reopen Command Tests
// =============================================================================

#[test]
fn test_reopen_moves_from_archive() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    // Create item and close it
    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);
    execute_close(Some("260101".to_string()), None).expect("close should succeed");

    // Now reopen
    execute_reopen(Some("260101".to_string()), None).expect("reopen should succeed");

    let stack_files = env.list_stack_files();
    assert_eq!(stack_files.len(), 1, "Stack should have one item");

    let archive_files = env.list_archive_files();
    assert!(archive_files.is_empty(), "Archive should be empty");

    // Check status was updated
    let content = env.read_item(&stack_files[0]);
    assert!(content.contains("status: open"), "Status should be open");
}

#[test]
fn test_reopen_restores_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    // Create item with category and close it
    create_test_item(&env, "260101-AAA", "Bug Task", "open", &[], Some("bugs"));
    execute_close(Some("260101".to_string()), None).expect("close should succeed");

    // Reopen - should restore to category
    execute_reopen(Some("260101".to_string()), None).expect("reopen should succeed");

    let category_files = env.list_category_files("bugs");
    assert_eq!(category_files.len(), 1, "Item should be back in category");
}

#[test]
fn test_reopen_already_open() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    // Try to reopen an already open item
    let result = execute_reopen(Some("260101".to_string()), None);
    assert!(result.is_err(), "reopening already open item should fail");
}

#[test]
fn test_reopen_with_partial_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-ABCDEFG", "Task", "open", &[], None);
    execute_close(Some("260101".to_string()), None).expect("close should succeed");

    // Reopen with minimal partial ID
    let result = execute_reopen(Some("2601".to_string()), None);
    assert!(result.is_ok(), "reopen with partial ID should succeed");

    let stack_files = env.list_stack_files();
    assert_eq!(stack_files.len(), 1, "Item should be back in stack");
}

#[test]
fn test_reopen_nonexistent_item() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let result = execute_reopen(Some("nonexistent".to_string()), None);
    assert!(result.is_err(), "reopen nonexistent item should fail");
}

#[test]
fn test_reopen_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    // Don't call init

    let result = execute_reopen(Some("260101".to_string()), None);
    assert!(result.is_err(), "reopen without init should fail");
}

#[test]
fn test_close_and_reopen_preserves_labels() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(
        &env,
        "260101-AAA",
        "Task with Labels",
        "open",
        &["bug", "urgent"],
        None,
    );

    execute_close(Some("260101".to_string()), None).expect("close should succeed");
    execute_reopen(Some("260101".to_string()), None).expect("reopen should succeed");

    let item = env.find_item_by_id("260101").expect("item should exist");
    let content = env.read_item(&item);
    assert!(content.contains("- bug"), "Bug label should be preserved");
    assert!(
        content.contains("- urgent"),
        "Urgent label should be preserved"
    );
}

// =============================================================================
// Attachment Movement Tests (close/reopen)
// =============================================================================

#[test]
fn test_close_moves_attachments_to_archive() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    let item_id = "260101-AAA";
    create_test_item_with_attachments(&env, item_id, "Test Item", "open", &["1-file.txt"], None);

    // Verify attachment exists in stack
    assert_eq!(env.list_attachment_files(item_id).len(), 1);

    // Close the item
    execute_close(Some(item_id.to_string()), None).expect("close should succeed");

    // Verify attachment moved to archive
    assert!(
        env.list_attachment_files(item_id).is_empty(),
        "No attachments in stack"
    );
    assert_eq!(
        env.list_archive_attachment_files(item_id).len(),
        1,
        "Attachment should be in archive"
    );
}

#[test]
fn test_reopen_moves_attachments_from_archive() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    let item_id = "260101-AAA";
    create_test_item_with_attachments(&env, item_id, "Test Item", "open", &["1-file.txt"], None);

    // Close and then reopen
    execute_close(Some(item_id.to_string()), None).expect("close should succeed");
    execute_reopen(Some(item_id.to_string()), None).expect("reopen should succeed");

    // Verify attachment is back in stack
    assert_eq!(
        env.list_attachment_files(item_id).len(),
        1,
        "Attachment should be back in stack"
    );
    assert!(
        env.list_archive_attachment_files(item_id).is_empty(),
        "No attachments in archive"
    );
}

#[test]
fn test_close_item_with_category_moves_attachments() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    let item_id = "260101-AAA";
    create_test_item_with_attachments(
        &env,
        item_id,
        "Test Item",
        "open",
        &["1-file.txt"],
        Some("bugs"),
    );

    execute_close(Some(item_id.to_string()), None).expect("close should succeed");

    assert_eq!(
        env.list_archive_attachment_files(item_id).len(),
        1,
        "Attachment should be in archive"
    );
}
