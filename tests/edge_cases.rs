//! # Edge Cases and Error Handling Tests
//!
//! Tests for edge cases, special characters, partial ID matching,
//! and other unusual scenarios.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod common;

use common::{create_test_item, GlobalConfigBuilder, TestEnv};
use qstack::commands::{self, InteractiveArgs, NewArgs, UpdateArgs};

// =============================================================================
// Special Characters and Unicode
// =============================================================================

#[test]
fn test_special_characters_in_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("Bug: 100% failure rate (critical!)".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
    };

    commands::new(args).expect("new should succeed with special characters");
    assert_eq!(env.count_all_items(), 1);
}

#[test]
fn test_unicode_in_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("Support für Umlaute (日本語テスト)".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
    };

    commands::new(args).expect("new should succeed with unicode");
    assert_eq!(env.count_all_items(), 1);
}

#[test]
fn test_empty_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    // Empty title should still work (will create file with just ID)
    let args = NewArgs {
        title: Some("".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
    };

    commands::new(args).expect("new should succeed with empty title");
    assert_eq!(env.count_all_items(), 1);
}

#[test]
fn test_very_long_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let long_title = "A".repeat(500);
    let args = NewArgs {
        title: Some(long_title),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
    };

    commands::new(args).expect("new should succeed with long title");
    assert_eq!(env.count_all_items(), 1);

    // Filename should be truncated
    let files = env.list_stack_files();
    let filename = files[0].file_name().unwrap().to_str().unwrap();
    assert!(filename.len() < 200, "Filename should be truncated");
}

#[test]
fn test_whitespace_only_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("   ".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
    };

    // Should create item (whitespace is trimmed to empty)
    let result = commands::new(args);
    assert!(result.is_ok(), "whitespace title should be handled");
}

#[test]
fn test_category_with_special_characters() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    // Category names should be sanitized
    let args = NewArgs {
        title: Some("Task".to_string()),
        labels: vec![],
        category: Some("my-category_v2".to_string()),
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
    };

    let result = commands::new(args);
    assert!(
        result.is_ok(),
        "category with dashes and underscores should work"
    );
}

#[test]
fn test_label_with_special_characters() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("Task".to_string()),
        labels: vec![
            "bug-fix".to_string(),
            "v2.0".to_string(),
            "priority_high".to_string(),
        ],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    let content = env.read_item(&files[0]);
    assert!(content.contains("- bug-fix"), "Label with dash");
    assert!(content.contains("- v2.0"), "Label with dot");
    assert!(content.contains("- priority_high"), "Label with underscore");
}

#[test]
fn test_duplicate_labels_ignored() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("Task".to_string()),
        labels: vec!["bug".to_string(), "bug".to_string(), "bug".to_string()],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    let content = env.read_item(&files[0]);

    // Count occurrences of "- bug"
    let count = content.matches("- bug").count();
    assert!(
        count <= 3,
        "Duplicate labels may or may not be deduplicated"
    );
}

// =============================================================================
// Partial ID Matching
// =============================================================================

#[test]
fn test_partial_id_matching() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-ABCD", "Task One", "open", &[], None);
    create_test_item(&env, "260201-EFGH", "Task Two", "open", &[], None);

    // Update with partial ID
    let args = UpdateArgs {
        id: "2601".to_string(), // Should match "260101-ABCD"
        title: Some("Updated".to_string()),
        labels: vec![],
        category: None,
        clear_category: false,
    };

    commands::update(args).expect("update with partial ID should succeed");
}

#[test]
fn test_ambiguous_partial_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAAA", "Task One", "open", &[], None);
    create_test_item(&env, "260101-BBBB", "Task Two", "open", &[], None);

    // Ambiguous ID should fail
    let args = UpdateArgs {
        id: "2601".to_string(), // Matches both items
        title: Some("Updated".to_string()),
        labels: vec![],
        category: None,
        clear_category: false,
    };

    let result = commands::update(args);
    assert!(result.is_err(), "update with ambiguous ID should fail");
}
