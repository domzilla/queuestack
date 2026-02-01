//! # New Command Tests
//!
//! Tests for the `qs new` command.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod common;

use common::{GlobalConfigBuilder, ProjectConfigBuilder, TestEnv};
use queuestack::commands::{self, InteractiveArgs, NewArgs};

#[test]
fn test_new_creates_item() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("Test Item".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: None,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    assert_eq!(files.len(), 1, "Should have one item");

    let content = env.read_item(&files[0]);
    assert!(
        content.contains("title: Test Item"),
        "Should have correct title"
    );
    assert!(content.contains("author: Test User"), "Should have author");
    assert!(content.contains("status: open"), "Should be open");
}

#[test]
fn test_new_with_labels() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("Bug Report".to_string()),
        labels: vec!["bug".to_string(), "urgent".to_string()],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: None,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    let content = env.read_item(&files[0]);
    assert!(content.contains("- bug"), "Should have bug label");
    assert!(content.contains("- urgent"), "Should have urgent label");
}

#[test]
fn test_new_with_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("Bug in Login".to_string()),
        labels: vec![],
        category: Some("bugs".to_string()),
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: None,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_category_files("bugs");
    assert_eq!(files.len(), 1, "Should have one item in bugs category");
}

#[test]
fn test_new_uses_custom_id_pattern() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().id_pattern("%y%j-%RR").build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("Custom ID Item".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: None,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    let filename = files[0].file_name().unwrap().to_str().unwrap();
    // Pattern %y%j-%RR produces 8 characters: YYJJJ-RR
    assert!(filename.len() > 8, "Filename should include ID and slug");
}

#[test]
fn test_new_project_id_pattern_overrides_global() {
    let env = TestEnv::new();
    env.write_global_config(
        &GlobalConfigBuilder::new()
            .id_pattern("%y%m%d-%T%RRR")
            .build(),
    );
    env.write_project_config(&ProjectConfigBuilder::new().id_pattern("PROJ-%RRR").build());
    std::fs::create_dir_all(env.stack_path()).expect("create stack dir");
    std::fs::create_dir_all(env.archive_path()).expect("create archive dir");

    let args = NewArgs {
        title: Some("Project Pattern".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: None,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    let filename = files[0].file_name().unwrap().to_str().unwrap();
    assert!(
        filename.starts_with("PROJ-"),
        "Should use project ID pattern"
    );
}

#[test]
fn test_new_with_labels_and_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("Critical Bug".to_string()),
        labels: vec!["bug".to_string(), "urgent".to_string(), "p0".to_string()],
        category: Some("bugs".to_string()),
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: None,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_category_files("bugs");
    assert_eq!(files.len(), 1, "Should have one item in bugs category");

    let content = env.read_item(&files[0]);
    assert!(content.contains("- bug"), "Should have bug label");
    assert!(content.contains("- urgent"), "Should have urgent label");
    assert!(content.contains("- p0"), "Should have p0 label");
    // Category is derived from folder path, not stored in metadata
}

#[test]
fn test_new_with_attachments() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    // Create test files
    let file1 = env.create_test_file("screenshot.png", "fake png");
    let file2 = env.create_test_file("debug.log", "log content");

    let args = NewArgs {
        title: Some("Bug with attachments".to_string()),
        labels: vec!["bug".to_string()],
        category: None,
        attachments: vec![
            file1.to_string_lossy().to_string(),
            file2.to_string_lossy().to_string(),
            "https://github.com/issue/42".to_string(),
        ],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: None,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    assert_eq!(files.len(), 1, "Should have one item");

    // Verify attachments were added
    let content = env.read_item(&files[0]);
    assert!(
        content.contains("attachments:"),
        "Should have attachments field"
    );
    assert!(
        content.contains("https://github.com/issue/42"),
        "Should have URL attachment"
    );
    assert!(
        content.contains("1-screenshot.png"),
        "Should have first file attachment"
    );
    assert!(
        content.contains("2-debug.log"),
        "Should have second file attachment"
    );

    // Get item ID from filename
    let item_id = files[0]
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .split('-')
        .take(2)
        .collect::<Vec<_>>()
        .join("-");

    // Verify files were copied
    let attachment_files = env.list_attachment_files(&item_id);
    assert_eq!(
        attachment_files.len(),
        2,
        "Should have two attachment files"
    );
}

#[test]
fn test_new_with_empty_labels() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("No Labels".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: None,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    assert_eq!(files.len(), 1, "Should have one item");

    let content = env.read_item(&files[0]);
    assert!(
        content.contains("title: No Labels"),
        "Should have correct title"
    );
    // Empty labels are omitted from serialization (skip_serializing_if = "Vec::is_empty")
    // so there should be no "labels:" section with list items
    assert!(
        !content.contains("labels:\n  - "),
        "Should not have label list entries"
    );
}

#[test]
fn test_new_multiple_items_unique_ids() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    // Create multiple items rapidly
    for i in 0..5 {
        let args = NewArgs {
            title: Some(format!("Task {i}")),
            labels: vec![],
            category: None,
            attachments: vec![],
            interactive: InteractiveArgs {
                interactive: false,
                no_interactive: true,
            },
            as_template: false,
            from_template: None,
        };
        commands::new(args).expect("new should succeed");
    }

    let files = env.list_stack_files();
    assert_eq!(files.len(), 5, "Should have 5 items");

    // Verify all IDs are unique
    let mut ids: Vec<String> = files
        .iter()
        .map(|f| {
            f.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .split('-')
                .take(2)
                .collect::<Vec<_>>()
                .join("-")
        })
        .collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 5, "All IDs should be unique");
}

#[test]
fn test_new_category_with_slash_normalized() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("Nested Task".to_string()),
        labels: vec![],
        category: Some("level1/level2".to_string()), // slash normalized to hyphen
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: None,
    };

    commands::new(args).expect("new should succeed");

    // Slashes in category names are normalized to hyphens
    let normalized_path = env.stack_path().join("level1-level2");
    assert!(
        normalized_path.exists(),
        "Category with normalized slash should exist"
    );
}

#[test]
fn test_new_normalizes_spaces_in_labels_and_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("Test Item".to_string()),
        labels: vec!["my label".to_string(), "another one".to_string()],
        category: Some("my category".to_string()),
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: None,
    };

    commands::new(args).expect("new should succeed");

    // Category folder should have hyphens instead of spaces
    let category_path = env.stack_path().join("my-category");
    assert!(
        category_path.exists(),
        "Category folder should use hyphens: my-category"
    );

    let files = env.list_category_files("my-category");
    assert_eq!(files.len(), 1, "Should have one item in category");

    // Labels should have hyphens instead of spaces
    let content = env.read_item(&files[0]);
    assert!(
        content.contains("- my-label"),
        "Label should be normalized to my-label"
    );
    assert!(
        content.contains("- another-one"),
        "Label should be normalized to another-one"
    );
    assert!(
        !content.contains("my label"),
        "Should not contain spaces in labels"
    );
}

#[test]
fn test_new_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    // Don't call init

    let args = NewArgs {
        title: Some("Task".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: None,
    };

    let result = commands::new(args);
    assert!(result.is_err(), "new without init should fail");
}
