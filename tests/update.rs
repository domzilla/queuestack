//! # Update Command Tests
//!
//! Tests for the `qstack update` command.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod common;

use common::{create_test_item, GlobalConfigBuilder, TestEnv};
use qstack::commands::{self, UpdateArgs};

#[test]
fn test_update_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Old Title", "open", &[], None);

    let args = UpdateArgs {
        id: Some("260101".to_string()),
        file: None,
        title: Some("New Title".to_string()),
        labels: vec![],
        category: None,
        clear_category: false,
    };

    commands::update(args).expect("update should succeed");

    // Verify the file was renamed and content updated
    let item = env.find_item_by_id("260101").expect("item should exist");
    let content = env.read_item(&item);
    assert!(
        content.contains("title: New Title"),
        "Title should be updated"
    );
}

#[test]
fn test_update_add_labels() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &["existing"], None);

    let args = UpdateArgs {
        id: Some("260101".to_string()),
        file: None,
        title: None,
        labels: vec!["new-label".to_string()],
        category: None,
        clear_category: false,
    };

    commands::update(args).expect("update should succeed");

    let item = env.find_item_by_id("260101").expect("item should exist");
    let content = env.read_item(&item);
    assert!(
        content.contains("- existing"),
        "Original label should remain"
    );
    assert!(content.contains("- new-label"), "New label should be added");
}

#[test]
fn test_update_move_to_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    let args = UpdateArgs {
        id: Some("260101".to_string()),
        file: None,
        title: None,
        labels: vec![],
        category: Some("bugs".to_string()),
        clear_category: false,
    };

    commands::update(args).expect("update should succeed");

    let files = env.list_category_files("bugs");
    assert_eq!(files.len(), 1, "Item should be in bugs category");
}

#[test]
fn test_update_clear_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], Some("bugs"));

    let args = UpdateArgs {
        id: Some("260101".to_string()),
        file: None,
        title: None,
        labels: vec![],
        category: None,
        clear_category: true,
    };

    commands::update(args).expect("update should succeed");

    let stack_files = env.list_stack_files();
    assert_eq!(stack_files.len(), 1, "Item should be in root stack");

    let category_files = env.list_category_files("bugs");
    assert!(category_files.is_empty(), "Category should be empty");
}

#[test]
fn test_update_nonexistent_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let args = UpdateArgs {
        id: Some("999999".to_string()),
        file: None,
        title: Some("New Title".to_string()),
        labels: vec![],
        category: None,
        clear_category: false,
    };

    let result = commands::update(args);
    assert!(result.is_err(), "update with nonexistent ID should fail");
}

#[test]
fn test_update_multiple_labels_at_once() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    let args = UpdateArgs {
        id: Some("260101".to_string()),
        file: None,
        title: None,
        labels: vec![
            "bug".to_string(),
            "urgent".to_string(),
            "critical".to_string(),
        ],
        category: None,
        clear_category: false,
    };

    commands::update(args).expect("update should succeed");

    let item = env.find_item_by_id("260101").expect("item should exist");
    let content = env.read_item(&item);
    assert!(content.contains("- bug"), "Should have bug label");
    assert!(content.contains("- urgent"), "Should have urgent label");
    assert!(content.contains("- critical"), "Should have critical label");
}

#[test]
fn test_update_title_and_category_combined() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Old Title", "open", &[], None);

    let args = UpdateArgs {
        id: Some("260101".to_string()),
        file: None,
        title: Some("New Title".to_string()),
        labels: vec![],
        category: Some("bugs".to_string()),
        clear_category: false,
    };

    commands::update(args).expect("update should succeed");

    let files = env.list_category_files("bugs");
    assert_eq!(files.len(), 1, "Item should be in bugs category");

    let content = env.read_item(&files[0]);
    assert!(
        content.contains("title: New Title"),
        "Title should be updated"
    );
}

#[test]
fn test_update_title_and_labels_combined() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(
        &env,
        "260101-AAA",
        "Old Title",
        "open",
        &["old-label"],
        None,
    );

    let args = UpdateArgs {
        id: Some("260101".to_string()),
        file: None,
        title: Some("New Title".to_string()),
        labels: vec!["new-label".to_string()],
        category: None,
        clear_category: false,
    };

    commands::update(args).expect("update should succeed");

    let item = env.find_item_by_id("260101").expect("item should exist");
    let content = env.read_item(&item);
    assert!(
        content.contains("title: New Title"),
        "Title should be updated"
    );
    assert!(
        content.contains("- old-label"),
        "Old label should be preserved"
    );
    assert!(content.contains("- new-label"), "New label should be added");
}

#[test]
fn test_update_all_fields_combined() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Original", "open", &[], None);

    let args = UpdateArgs {
        id: Some("260101".to_string()),
        file: None,
        title: Some("Updated Title".to_string()),
        labels: vec!["label1".to_string(), "label2".to_string()],
        category: Some("features".to_string()),
        clear_category: false,
    };

    commands::update(args).expect("update should succeed");

    let files = env.list_category_files("features");
    assert_eq!(files.len(), 1, "Item should be in features category");

    let content = env.read_item(&files[0]);
    assert!(content.contains("title: Updated Title"), "Title updated");
    assert!(content.contains("- label1"), "Label1 added");
    assert!(content.contains("- label2"), "Label2 added");
}

#[test]
fn test_update_category_then_clear() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    // First, move to category
    let args = UpdateArgs {
        id: Some("260101".to_string()),
        file: None,
        title: None,
        labels: vec![],
        category: Some("bugs".to_string()),
        clear_category: false,
    };
    commands::update(args).expect("update should succeed");

    // Then clear category
    let args = UpdateArgs {
        id: Some("260101".to_string()),
        file: None,
        title: None,
        labels: vec![],
        category: None,
        clear_category: true,
    };
    commands::update(args).expect("clear category should succeed");

    let files = env.list_stack_files();
    assert_eq!(files.len(), 1, "Item should be in root stack");

    let category_files = env.list_category_files("bugs");
    assert!(category_files.is_empty(), "Category should be empty");
}

#[test]
fn test_update_with_full_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-ABCDEFG", "Task", "open", &[], None);

    let args = UpdateArgs {
        id: Some("260101-ABCDEFG".to_string()), // Full ID
        file: None,
        title: Some("Updated".to_string()),
        labels: vec![],
        category: None,
        clear_category: false,
    };

    let result = commands::update(args);
    assert!(result.is_ok(), "update with full ID should succeed");
}

#[test]
fn test_update_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    // Don't call init

    let args = UpdateArgs {
        id: Some("260101".to_string()),
        file: None,
        title: Some("New".to_string()),
        labels: vec![],
        category: None,
        clear_category: false,
    };

    let result = commands::update(args);
    assert!(result.is_err(), "update without init should fail");
}
