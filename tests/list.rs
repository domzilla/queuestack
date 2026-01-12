//! # List Command Tests
//!
//! Tests for the `qstack list` command.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod common;

use common::{create_test_item, GlobalConfigBuilder, TestEnv};
use qstack::commands::{self, InteractiveArgs, ListFilter, ListMode, SortBy, StatusFilter};

#[test]
fn test_list_empty_project() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let filter = ListFilter {
        mode: ListMode::Items,
        status: StatusFilter::All,
        label: None,
        author: None,
        sort: SortBy::Id,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: None,
    };

    // Should not error even if empty
    let result = commands::list(&filter);
    assert!(result.is_ok(), "list should succeed even if empty");
}

#[test]
fn test_list_shows_open_items() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    // Create test items
    create_test_item(&env, "260101-AAA", "First Task", "open", &[], None);
    create_test_item(&env, "260102-BBB", "Second Task", "open", &[], None);
    create_test_item(&env, "260103-CCC", "Closed Task", "closed", &[], None);

    // Move closed task to archive
    let archive = env.archive_path();
    std::fs::rename(
        env.stack_path().join("260103-CCC-closed-task.md"),
        archive.join("260103-CCC-closed-task.md"),
    )
    .expect("move to archive");

    let filter = ListFilter {
        mode: ListMode::Items,
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

    // Should succeed (output goes to stdout)
    let result = commands::list(&filter);
    assert!(result.is_ok(), "list should succeed");
}

#[test]
fn test_list_filter_by_label() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Bug Task", "open", &["bug"], None);
    create_test_item(
        &env,
        "260102-BBB",
        "Feature Task",
        "open",
        &["feature"],
        None,
    );

    let filter = ListFilter {
        mode: ListMode::Items,
        status: StatusFilter::All,
        label: Some("bug".to_string()),
        author: None,
        sort: SortBy::Id,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: None,
    };

    let result = commands::list(&filter);
    assert!(result.is_ok(), "list with label filter should succeed");
}

#[test]
fn test_list_sort_by_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Zebra Task", "open", &[], None);
    create_test_item(&env, "260102-BBB", "Alpha Task", "open", &[], None);

    let filter = ListFilter {
        mode: ListMode::Items,
        status: StatusFilter::All,
        label: None,
        author: None,
        sort: SortBy::Title,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: None,
    };

    let result = commands::list(&filter);
    assert!(result.is_ok(), "list with sort should succeed");
}

#[test]
fn test_list_shows_closed_items() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Open Task", "open", &[], None);
    create_test_item(&env, "260102-BBB", "Closed Task", "closed", &[], None);

    // Move closed task to archive
    std::fs::rename(
        env.stack_path().join("260102-BBB-closed-task.md"),
        env.archive_path().join("260102-BBB-closed-task.md"),
    )
    .expect("move to archive");

    let filter = ListFilter {
        mode: ListMode::Items,
        status: StatusFilter::Closed,
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
    assert!(result.is_ok(), "list --closed should succeed");
}

#[test]
fn test_list_filter_by_author() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task by Test User", "open", &[], None);

    // Author filter uses exact match (case-insensitive)
    let filter = ListFilter {
        mode: ListMode::Items,
        status: StatusFilter::All,
        label: None,
        author: Some("Test User".to_string()),
        sort: SortBy::Id,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: None,
    };

    let result = commands::list(&filter);
    assert!(result.is_ok(), "list with author filter should succeed");
}

#[test]
fn test_list_sort_by_date() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "First Task", "open", &[], None);
    create_test_item(&env, "260102-BBB", "Second Task", "open", &[], None);

    let filter = ListFilter {
        mode: ListMode::Items,
        status: StatusFilter::All,
        label: None,
        author: None,
        sort: SortBy::Date,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: None,
    };

    let result = commands::list(&filter);
    assert!(result.is_ok(), "list with date sort should succeed");
}

#[test]
fn test_list_combined_filters() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Bug One", "open", &["bug"], None);
    create_test_item(
        &env,
        "260102-BBB",
        "Bug Two",
        "open",
        &["bug", "urgent"],
        None,
    );
    create_test_item(&env, "260103-CCC", "Feature", "open", &["feature"], None);

    // Author filter uses exact match (case-insensitive)
    let filter = ListFilter {
        mode: ListMode::Items,
        status: StatusFilter::Open,
        label: Some("bug".to_string()),
        author: Some("Test User".to_string()),
        sort: SortBy::Title,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: None,
    };

    let result = commands::list(&filter);
    assert!(result.is_ok(), "list with combined filters should succeed");
}

#[test]
fn test_list_open_and_closed_flags_together() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Open Task", "open", &[], None);
    create_test_item(&env, "260102-BBB", "Closed Task", "closed", &[], None);
    std::fs::rename(
        env.stack_path().join("260102-BBB-closed-task.md"),
        env.archive_path().join("260102-BBB-closed-task.md"),
    )
    .expect("move to archive");

    // Both flags true - should show all items
    let filter = ListFilter {
        mode: ListMode::Items,
        status: StatusFilter::All,
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
    assert!(
        result.is_ok(),
        "list with both --open and --closed should succeed"
    );
}

#[test]
fn test_list_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    // Don't call init

    let filter = ListFilter {
        mode: ListMode::Items,
        status: StatusFilter::All,
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
    assert!(result.is_err(), "list without init should fail");
}

#[test]
fn test_list_author_case_insensitive() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    // Author filter uses exact match but is case-insensitive
    let filter = ListFilter {
        mode: ListMode::Items,
        status: StatusFilter::All,
        label: None,
        author: Some("TEST USER".to_string()), // uppercase of "Test User"
        sort: SortBy::Id,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: None,
    };

    let result = commands::list(&filter);
    assert!(result.is_ok(), "author case-insensitive match should work");
}

#[test]
fn test_list_nonexistent_label_filter() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &["bug"], None);

    let filter = ListFilter {
        mode: ListMode::Items,
        status: StatusFilter::All,
        label: Some("nonexistent-label".to_string()),
        author: None,
        sort: SortBy::Id,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: None,
    };

    // Should succeed but return empty list
    let result = commands::list(&filter);
    assert!(result.is_ok(), "list with no matching label should succeed");
}

#[test]
fn test_list_interactive_combinations() {
    // Test with interactive=true, no_interactive=true (override)
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().interactive(true).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

        let filter = ListFilter {
            mode: ListMode::Items,
            status: StatusFilter::All,
            label: None,
            author: None,
            sort: SortBy::Id,
            interactive: InteractiveArgs {
                interactive: false,
                no_interactive: true,
            }, // Override interactive
            id: None,
        };

        commands::list(&filter).expect("list should succeed");
    }

    // Test with interactive=true, no_interactive=false (would show interactive selector if terminal)
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().interactive(true).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

        let filter = ListFilter {
            mode: ListMode::Items,
            status: StatusFilter::All,
            label: None,
            author: None,
            sort: SortBy::Id,
            interactive: InteractiveArgs {
                interactive: false,
                no_interactive: false,
            }, // Would show selector if in terminal
            id: None,
        };

        // Works because we're not in a terminal, so interactive selection is skipped
        commands::list(&filter).expect("list should succeed");
    }

    // Test with interactive=false, no_interactive=false (never shows selector)
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

        let filter = ListFilter {
            mode: ListMode::Items,
            status: StatusFilter::All,
            label: None,
            author: None,
            sort: SortBy::Id,
            interactive: InteractiveArgs {
                interactive: false,
                no_interactive: false,
            },
            id: None,
        };

        commands::list(&filter).expect("list should succeed");
    }

    // Test with interactive=false, no_interactive=true (definitely no selector)
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

        let filter = ListFilter {
            mode: ListMode::Items,
            status: StatusFilter::All,
            label: None,
            author: None,
            sort: SortBy::Id,
            interactive: InteractiveArgs {
                interactive: false,
                no_interactive: true,
            },
            id: None,
        };

        commands::list(&filter).expect("list should succeed");
    }
}
