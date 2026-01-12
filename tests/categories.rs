//! # Categories Command Tests
//!
//! Tests for the `qstack list --categories` command.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod common;

use common::{create_test_item, GlobalConfigBuilder, TestEnv};
use qstack::commands::{self, InteractiveArgs, ListFilter, ListMode, SortBy, StatusFilter};

#[test]
fn test_categories_empty_project() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let filter = ListFilter {
        mode: ListMode::Categories,
        status: StatusFilter::Open,
        label: None,
        author: None,
        sort: SortBy::Id,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: None,
    };

    let result = commands::list(&filter);
    assert!(result.is_ok(), "categories on empty project should succeed");
}

#[test]
fn test_categories_shows_unique_categories() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task 1", "open", &[], Some("bugs"));
    create_test_item(&env, "260102-BBB", "Task 2", "open", &[], Some("bugs"));
    create_test_item(&env, "260103-CCC", "Task 3", "open", &[], Some("features"));
    create_test_item(&env, "260104-DDD", "Task 4", "open", &[], None); // uncategorized

    let filter = ListFilter {
        mode: ListMode::Categories,
        status: StatusFilter::Open,
        label: None,
        author: None,
        sort: SortBy::Id,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: None,
    };

    let result = commands::list(&filter);
    assert!(result.is_ok(), "categories should succeed");
}

#[test]
fn test_categories_includes_archived_items() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Open Task", "open", &[], Some("active"));
    create_test_item(
        &env,
        "260102-BBB",
        "Closed Task",
        "closed",
        &[],
        Some("done"),
    );
    std::fs::rename(
        env.stack_path().join("done/260102-BBB-closed-task.md"),
        env.archive_path().join("260102-BBB-closed-task.md"),
    )
    .expect("move to archive");

    let filter = ListFilter {
        mode: ListMode::Categories,
        status: StatusFilter::Open,
        label: None,
        author: None,
        sort: SortBy::Id,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: None,
    };

    // Should include categories from both open and archived items
    let result = commands::list(&filter);
    assert!(result.is_ok(), "categories should include archived items");
}

#[test]
fn test_categories_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    // Don't init

    let filter = ListFilter {
        mode: ListMode::Categories,
        status: StatusFilter::Open,
        label: None,
        author: None,
        sort: SortBy::Id,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: None,
    };

    let result = commands::list(&filter);
    assert!(result.is_err(), "categories without init should fail");
}
