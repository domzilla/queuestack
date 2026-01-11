//! # Labels Command Tests
//!
//! Tests for the `qstack labels` command.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod common;

use common::{create_test_item, GlobalConfigBuilder, TestEnv};
use qstack::commands::{self, InteractiveArgs, LabelsArgs};

#[test]
fn test_labels_empty_project() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let args = LabelsArgs {
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
    };

    let result = commands::labels(&args);
    assert!(result.is_ok(), "labels on empty project should succeed");
}

#[test]
fn test_labels_shows_unique_labels() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task 1", "open", &["bug", "ui"], None);
    create_test_item(&env, "260102-BBB", "Task 2", "open", &["bug"], None);
    create_test_item(&env, "260103-CCC", "Task 3", "open", &["feature"], None);

    let args = LabelsArgs {
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
    };

    let result = commands::labels(&args);
    assert!(result.is_ok(), "labels should succeed");
}

#[test]
fn test_labels_includes_archived_items() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(
        &env,
        "260101-AAA",
        "Open Task",
        "open",
        &["open-label"],
        None,
    );
    create_test_item(
        &env,
        "260102-BBB",
        "Closed Task",
        "closed",
        &["closed-label"],
        None,
    );
    std::fs::rename(
        env.stack_path().join("260102-BBB-closed-task.md"),
        env.archive_path().join("260102-BBB-closed-task.md"),
    )
    .expect("move to archive");

    let args = LabelsArgs {
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
    };

    // Should include labels from both open and archived items
    let result = commands::labels(&args);
    assert!(result.is_ok(), "labels should include archived items");
}

#[test]
fn test_labels_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    // Don't init

    let args = LabelsArgs {
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
    };

    let result = commands::labels(&args);
    assert!(result.is_err(), "labels without init should fail");
}
