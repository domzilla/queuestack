//! # Integration Tests
//!
//! Comprehensive integration tests for all qstack commands.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod harness;

use harness::{
    create_test_item, create_test_item_with_attachments, GlobalConfigBuilder, ProjectConfigBuilder,
    TestEnv,
};
use qstack::commands::{
    self, execute_close, execute_reopen, AttachAddArgs, AttachRemoveArgs, AttachmentsArgs,
    CategoriesArgs, LabelsArgs, ListFilter, NewArgs, SearchArgs, SortBy, UpdateArgs,
};

// =============================================================================
// Init Command Tests
// =============================================================================

#[test]
fn test_init_creates_project_structure() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());

    // Run init
    commands::init().expect("init should succeed");

    // Verify structure
    assert!(
        env.project_config_path().exists(),
        "Project config should exist"
    );
    assert!(env.stack_path().exists(), "Stack directory should exist");
    assert!(
        env.archive_path().exists(),
        "Archive directory should exist"
    );
}

#[test]
fn test_init_fails_without_global_config() {
    let env = TestEnv::new();
    assert!(
        !env.global_config_path().exists(),
        "Global config should not exist initially"
    );

    // Run init - should fail because global config doesn't exist
    let result = commands::init();
    assert!(result.is_err(), "init should fail without global config");

    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("qstack setup"),
        "Error should mention running 'qstack setup': {err}"
    );
}

#[test]
fn test_init_fails_if_already_initialized() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());

    // First init
    commands::init().expect("first init should succeed");

    // Second init should fail
    let result = commands::init();
    assert!(result.is_err(), "Second init should fail");
}

// =============================================================================
// New Command Tests
// =============================================================================

#[test]
fn test_new_creates_item() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Test Item".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
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
        title: "Bug Report".to_string(),
        labels: vec!["bug".to_string(), "urgent".to_string()],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
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
        title: "Bug in Login".to_string(),
        labels: vec![],
        category: Some("bugs".to_string()),
        attachments: vec![],
        interactive: false,
        no_interactive: true,
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
        title: "Custom ID Item".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
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
        title: "Project Pattern".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    let filename = files[0].file_name().unwrap().to_str().unwrap();
    assert!(
        filename.starts_with("PROJ-"),
        "Should use project ID pattern"
    );
}

// =============================================================================
// List Command Tests
// =============================================================================

#[test]
fn test_list_empty_project() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let filter = ListFilter {
        open: false,
        closed: false,
        label: None,
        author: None,
        sort: SortBy::Id,
        interactive: false,
        no_interactive: true,
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
        open: true,
        closed: false,
        label: None,
        author: None,
        sort: SortBy::Id,
        interactive: false,
        no_interactive: true,
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
        open: false,
        closed: false,
        label: Some("bug".to_string()),
        author: None,
        sort: SortBy::Id,
        interactive: false,
        no_interactive: true,
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
        open: false,
        closed: false,
        label: None,
        author: None,
        sort: SortBy::Title,
        interactive: false,
        no_interactive: true,
    };

    let result = commands::list(&filter);
    assert!(result.is_ok(), "list with sort should succeed");
}

// =============================================================================
// Search Command Tests
// =============================================================================

#[test]
fn test_search_by_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Login Bug", "open", &[], None);
    create_test_item(&env, "260102-BBB", "Feature Request", "open", &[], None);

    let args = SearchArgs {
        query: "login".to_string(),
        full_text: false,
        interactive: false,
        no_interactive: true,
        closed: false,
    };

    let result = commands::search(&args);
    assert!(result.is_ok(), "search should succeed");
}

#[test]
fn test_search_by_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Some Task", "open", &[], None);

    let args = SearchArgs {
        query: "260101".to_string(),
        full_text: false,
        interactive: false,
        no_interactive: true,
        closed: false,
    };

    let result = commands::search(&args);
    assert!(result.is_ok(), "search by ID should succeed");
}

#[test]
fn test_search_case_insensitive() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Important Bug", "open", &[], None);

    let args = SearchArgs {
        query: "IMPORTANT".to_string(),
        full_text: false,
        interactive: false,
        no_interactive: true,
        closed: false,
    };

    let result = commands::search(&args);
    assert!(result.is_ok(), "search should be case insensitive");
}

#[test]
fn test_search_no_results() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Some Task", "open", &[], None);

    let args = SearchArgs {
        query: "nonexistent".to_string(),
        full_text: false,
        interactive: false,
        no_interactive: true,
        closed: false,
    };

    let result = commands::search(&args);
    assert!(result.is_err(), "search with no results should error");
}

// =============================================================================
// Update Command Tests
// =============================================================================

#[test]
fn test_update_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Old Title", "open", &[], None);

    let args = UpdateArgs {
        id: "260101".to_string(),
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
        id: "260101".to_string(),
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
        id: "260101".to_string(),
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
        id: "260101".to_string(),
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
        id: "999999".to_string(),
        title: Some("New Title".to_string()),
        labels: vec![],
        category: None,
        clear_category: false,
    };

    let result = commands::update(args);
    assert!(result.is_err(), "update with nonexistent ID should fail");
}

// =============================================================================
// Close/Reopen Command Tests
// =============================================================================

#[test]
fn test_close_moves_to_archive() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task to Close", "open", &[], None);

    execute_close("260101").expect("close should succeed");

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

    execute_close("260101").expect("close should succeed");

    let category_files = env.list_category_files("bugs");
    assert!(category_files.is_empty(), "Category should be empty");

    let archive_files = env.list_archive_files();
    assert_eq!(archive_files.len(), 1, "Archive should have one item");
}

#[test]
fn test_reopen_moves_from_archive() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    // Create item and close it
    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);
    execute_close("260101").expect("close should succeed");

    // Now reopen
    execute_reopen("260101").expect("reopen should succeed");

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
    execute_close("260101").expect("close should succeed");

    // Reopen - should restore to category
    execute_reopen("260101").expect("reopen should succeed");

    let category_files = env.list_category_files("bugs");
    assert_eq!(category_files.len(), 1, "Item should be back in category");
}

#[test]
fn test_close_nonexistent_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let result = execute_close("999999");
    assert!(result.is_err(), "close with nonexistent ID should fail");
}

// =============================================================================
// Config Combination Tests (interactive + no_interactive)
// =============================================================================

/// Tests that commands work correctly with interactive=true and no_interactive=false.
/// Note: Editor won't actually open in tests because stdout is not a terminal.
#[test]
fn test_config_interactive_true_no_interactive_false() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(true).build());
    commands::init().expect("init should succeed");

    // With interactive=true and no_interactive=false, editor would open (if terminal)
    let args = NewArgs {
        title: "Test".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: false, // Would open editor if in terminal
    };

    let result = commands::new(args);
    assert!(result.is_ok(), "new should succeed");
    assert_eq!(env.count_all_items(), 1);
}

/// Tests that no_interactive flag overrides interactive=true config.
#[test]
fn test_config_interactive_true_no_interactive_true() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(true).build());
    commands::init().expect("init should succeed");

    // With no_interactive=true, editor should never open
    let args = NewArgs {
        title: "Test".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true, // Overrides interactive
    };

    let result = commands::new(args);
    assert!(result.is_ok(), "new should succeed");
    assert_eq!(env.count_all_items(), 1);
}

/// Tests that with interactive=false, editor never opens regardless of no_interactive.
#[test]
fn test_config_interactive_false_no_interactive_false() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    // With interactive=false, editor should never open
    let args = NewArgs {
        title: "Test".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: false, // Doesn't matter since interactive is false
    };

    let result = commands::new(args);
    assert!(result.is_ok(), "new should succeed");
    assert_eq!(env.count_all_items(), 1);
}

/// Tests that both interactive=false and no_interactive=true definitely prevents editor.
#[test]
fn test_config_interactive_false_no_interactive_true() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Test".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
    };

    let result = commands::new(args);
    assert!(result.is_ok(), "new should succeed");
    assert_eq!(env.count_all_items(), 1);
}

/// Tests list command with interactive configurations.
#[test]
fn test_list_interactive_combinations() {
    // Test with interactive=true, no_interactive=true (override)
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().interactive(true).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

        let filter = ListFilter {
            open: false,
            closed: false,
            label: None,
            author: None,
            sort: SortBy::Id,
            interactive: false,
            no_interactive: true, // Override interactive
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
            open: false,
            closed: false,
            label: None,
            author: None,
            sort: SortBy::Id,
            interactive: false,
            no_interactive: false, // Would show selector if in terminal
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
            open: false,
            closed: false,
            label: None,
            author: None,
            sort: SortBy::Id,
            interactive: false,
            no_interactive: false,
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
            open: false,
            closed: false,
            label: None,
            author: None,
            sort: SortBy::Id,
            interactive: false,
            no_interactive: true,
        };

        commands::list(&filter).expect("list should succeed");
    }
}

/// Tests search command with interactive configurations.
#[test]
fn test_search_interactive_combinations() {
    // Test with interactive=true, no_interactive=true
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().interactive(true).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Login Bug", "open", &[], None);

        let args = SearchArgs {
            query: "login".to_string(),
            full_text: false,
            interactive: false,
            no_interactive: true,
            closed: false,
        };

        commands::search(&args).expect("search should succeed");
    }

    // Test with interactive=false
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Login Bug", "open", &[], None);

        let args = SearchArgs {
            query: "login".to_string(),
            full_text: false,
            interactive: false,
            no_interactive: false,
            closed: false,
        };

        commands::search(&args).expect("search should succeed");
    }
}

// =============================================================================
// use_git_user Config Tests
// =============================================================================

/// Tests that use_git_user=false prevents using git user.name even if available.
#[test]
fn test_use_git_user_disabled() {
    let env = TestEnv::new();
    // Explicit user_name set, use_git_user=false should not matter
    env.write_global_config(
        &GlobalConfigBuilder::new()
            .user_name("Explicit User")
            .use_git_user(false)
            .build(),
    );
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Test".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    let content = env.read_item(&files[0]);
    assert!(
        content.contains("author: Explicit User"),
        "Should use explicit user_name"
    );
}

/// Tests that use_git_user=true allows falling back to git config.
/// Note: This test verifies the config is parsed correctly; actual git fallback
/// depends on git being configured on the test machine.
#[test]
fn test_use_git_user_enabled_with_explicit_name() {
    let env = TestEnv::new();
    // When both user_name and use_git_user are set, user_name takes precedence
    env.write_global_config(
        &GlobalConfigBuilder::new()
            .user_name("Config User")
            .use_git_user(true)
            .build(),
    );
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Test".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    let content = env.read_item(&files[0]);
    assert!(
        content.contains("author: Config User"),
        "Explicit user_name should take precedence over git"
    );
}

// =============================================================================
// editor Config Tests
// =============================================================================

/// Tests that custom editor config is parsed correctly.
/// Note: Editor won't actually open in tests (not a terminal), but we verify
/// the config value is stored and retrievable.
#[test]
fn test_custom_editor_config() {
    let env = TestEnv::new();
    env.write_global_config(
        &GlobalConfigBuilder::new()
            .editor("nvim")
            .interactive(false) // Don't try to open
            .build(),
    );
    commands::init().expect("init should succeed");

    // Verify config was written correctly
    let content = env.read_global_config();
    assert!(
        content.contains("editor = \"nvim\""),
        "Editor should be set in config"
    );
}

/// Tests editor with arguments (like "code --wait").
#[test]
fn test_editor_with_arguments() {
    let env = TestEnv::new();
    env.write_global_config(
        &GlobalConfigBuilder::new()
            .editor("code --wait")
            .interactive(false)
            .build(),
    );
    commands::init().expect("init should succeed");

    let content = env.read_global_config();
    assert!(
        content.contains("editor = \"code --wait\""),
        "Editor with args should be set"
    );
}

/// Tests that Config::editor() returns the configured value.
#[test]
fn test_config_editor_resolution() {
    use qstack::Config;

    let env = TestEnv::new();
    env.write_global_config(
        &GlobalConfigBuilder::new()
            .editor("custom-editor")
            .interactive(false)
            .build(),
    );
    commands::init().expect("init should succeed");

    let config = Config::load().expect("load config");
    assert_eq!(
        config.editor(),
        Some("custom-editor".to_string()),
        "Config should return custom editor"
    );
}

// Note: Editor env var fallback (VISUAL/EDITOR) is tested at the unit level
// in src/config/mod.rs. We don't test it here to avoid modifying shell env vars.

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

#[test]
fn test_special_characters_in_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Bug: 100% failure rate (critical!)".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
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
        title: "Support für Umlaute (日本語テスト)".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
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
        title: "".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
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
        title: long_title,
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
    };

    commands::new(args).expect("new should succeed with long title");
    assert_eq!(env.count_all_items(), 1);

    // Filename should be truncated
    let files = env.list_stack_files();
    let filename = files[0].file_name().unwrap().to_str().unwrap();
    assert!(filename.len() < 200, "Filename should be truncated");
}

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

// =============================================================================
// Custom Config Directory Tests
// =============================================================================

#[test]
fn test_custom_stack_directory() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    env.write_project_config(&ProjectConfigBuilder::new().stack_dir("tasks").build());

    let tasks_dir = env.project_path().join("tasks");
    let archive_dir = tasks_dir.join("archive");
    std::fs::create_dir_all(&tasks_dir).expect("create tasks dir");
    std::fs::create_dir_all(&archive_dir).expect("create archive dir");

    let args = NewArgs {
        title: "Task".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
    };

    commands::new(args).expect("new should succeed");

    // Check item was created in custom directory
    let files: Vec<_> = std::fs::read_dir(&tasks_dir)
        .expect("read tasks dir")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();

    assert_eq!(files.len(), 1, "Item should be in custom stack dir");
}

#[test]
fn test_custom_archive_directory() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    env.write_project_config(&ProjectConfigBuilder::new().archive_dir("done").build());

    std::fs::create_dir_all(env.stack_path()).expect("create stack dir");
    std::fs::create_dir_all(env.stack_path().join("done")).expect("create archive dir");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);
    execute_close("260101").expect("close should succeed");

    let done_dir = env.stack_path().join("done");
    let files: Vec<_> = std::fs::read_dir(&done_dir)
        .expect("read done dir")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();

    assert_eq!(files.len(), 1, "Item should be in custom archive dir");
}

// =============================================================================
// Global Config Isolation Tests
// =============================================================================

#[test]
fn test_global_config_isolation() {
    // Verify that tests don't affect each other's global config
    let env1 = TestEnv::new();
    env1.write_global_config(&GlobalConfigBuilder::new().user_name("User One").build());
    drop(env1);

    let env2 = TestEnv::new();
    assert!(
        !env2.global_config_path().exists(),
        "New env should not have previous env's config"
    );
}

#[test]
fn test_different_users_in_parallel() {
    // Note: These run sequentially due to ENV_LOCK, but test the isolation
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().user_name("Alice").build());
        commands::init().expect("init should succeed");

        let args = NewArgs {
            title: "Alice's Task".to_string(),
            labels: vec![],
            category: None,
            attachments: vec![],
            interactive: false,
            no_interactive: true,
        };

        commands::new(args).expect("new should succeed");

        let files = env.list_stack_files();
        let content = env.read_item(&files[0]);
        assert!(content.contains("author: Alice"), "Should be Alice's item");
    }

    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().user_name("Bob").build());
        commands::init().expect("init should succeed");

        let args = NewArgs {
            title: "Bob's Task".to_string(),
            labels: vec![],
            category: None,
            attachments: vec![],
            interactive: false,
            no_interactive: true,
        };

        commands::new(args).expect("new should succeed");

        let files = env.list_stack_files();
        let content = env.read_item(&files[0]);
        assert!(content.contains("author: Bob"), "Should be Bob's item");
    }
}

// =============================================================================
// Additional List Command Tests
// =============================================================================

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
        open: false,
        closed: true,
        label: None,
        author: None,
        sort: SortBy::Id,
        interactive: false,
        no_interactive: true,
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
        open: false,
        closed: false,
        label: None,
        author: Some("Test User".to_string()),
        sort: SortBy::Id,
        interactive: false,
        no_interactive: true,
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
        open: false,
        closed: false,
        label: None,
        author: None,
        sort: SortBy::Date,
        interactive: false,
        no_interactive: true,
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
        open: true,
        closed: false,
        label: Some("bug".to_string()),
        author: Some("Test User".to_string()),
        sort: SortBy::Title,
        interactive: false,
        no_interactive: true,
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
        open: true,
        closed: true,
        label: None,
        author: None,
        sort: SortBy::Id,
        interactive: false,
        no_interactive: true,
    };

    let result = commands::list(&filter);
    assert!(
        result.is_ok(),
        "list with both --open and --closed should succeed"
    );
}

// =============================================================================
// Additional Search Command Tests
// =============================================================================

#[test]
fn test_search_full_text() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    // Create item with specific body content
    let content = r#"---
id: 260101-AAA
title: Generic Title
author: Test User
created_at: 2026-01-09T12:00:00Z
status: open
labels: []
category: ~
---

This is the body with unique keyword: SEARCHTERM123
"#;
    std::fs::write(
        env.stack_path().join("260101-AAA-generic-title.md"),
        content,
    )
    .expect("write item");

    let args = SearchArgs {
        query: "SEARCHTERM123".to_string(),
        full_text: true,
        interactive: false,
        no_interactive: true,
        closed: false,
    };

    let result = commands::search(&args);
    assert!(result.is_ok(), "full-text search should find body content");
}

#[test]
fn test_search_full_text_no_match_without_flag() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    // Create item with body content but not in title
    let content = r#"---
id: 260101-AAA
title: Generic Title
author: Test User
created_at: 2026-01-09T12:00:00Z
status: open
labels: []
category: ~
---

Body with keyword: ONLYINBODY
"#;
    std::fs::write(
        env.stack_path().join("260101-AAA-generic-title.md"),
        content,
    )
    .expect("write item");

    let args = SearchArgs {
        query: "ONLYINBODY".to_string(),
        full_text: false, // Not searching body
        interactive: false,
        no_interactive: true,
        closed: false,
    };

    let result = commands::search(&args);
    assert!(
        result.is_err(),
        "should not find body content without full-text"
    );
}

#[test]
fn test_search_closed_items() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Archived Bug", "closed", &["bug"], None);
    std::fs::rename(
        env.stack_path().join("260101-AAA-archived-bug.md"),
        env.archive_path().join("260101-AAA-archived-bug.md"),
    )
    .expect("move to archive");

    let args = SearchArgs {
        query: "archived".to_string(),
        full_text: false,
        interactive: false,
        no_interactive: true,
        closed: true,
    };

    let result = commands::search(&args);
    assert!(result.is_ok(), "search --closed should find archived items");
}

#[test]
fn test_search_full_text_and_closed_combined() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    // Create a closed item with searchable body
    let content = r#"---
id: 260101-AAA
title: Old Task
author: Test User
created_at: 2026-01-09T12:00:00Z
status: closed
labels: []
category: ~
---

Body contains: ARCHIVEDCONTENT
"#;
    std::fs::write(env.archive_path().join("260101-AAA-old-task.md"), content).expect("write item");

    let args = SearchArgs {
        query: "ARCHIVEDCONTENT".to_string(),
        full_text: true,
        interactive: false,
        no_interactive: true,
        closed: true,
    };

    let result = commands::search(&args);
    assert!(
        result.is_ok(),
        "search with full-text and closed should find archived body content"
    );
}

#[test]
fn test_search_multiple_matches() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Login Bug", "open", &[], None);
    create_test_item(&env, "260102-BBB", "Login Feature", "open", &[], None);
    create_test_item(&env, "260103-CCC", "Login Improvement", "open", &[], None);

    let args = SearchArgs {
        query: "login".to_string(),
        full_text: false,
        interactive: false,
        no_interactive: true,
        closed: false,
    };

    let result = commands::search(&args);
    assert!(
        result.is_ok(),
        "search with multiple matches should succeed"
    );
}

// =============================================================================
// Additional New Command Tests
// =============================================================================

#[test]
fn test_new_with_labels_and_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Critical Bug".to_string(),
        labels: vec!["bug".to_string(), "urgent".to_string(), "p0".to_string()],
        category: Some("bugs".to_string()),
        attachments: vec![],
        interactive: false,
        no_interactive: true,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_category_files("bugs");
    assert_eq!(files.len(), 1, "Should have one item in bugs category");

    let content = env.read_item(&files[0]);
    assert!(content.contains("- bug"), "Should have bug label");
    assert!(content.contains("- urgent"), "Should have urgent label");
    assert!(content.contains("- p0"), "Should have p0 label");
    assert!(
        content.contains("category: bugs"),
        "Should have bugs category"
    );
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
        title: "Bug with attachments".to_string(),
        labels: vec!["bug".to_string()],
        category: None,
        attachments: vec![
            file1.to_string_lossy().to_string(),
            file2.to_string_lossy().to_string(),
            "https://github.com/issue/42".to_string(),
        ],
        interactive: false,
        no_interactive: true,
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
        content.contains("-Attachment-1-"),
        "Should have first file attachment"
    );
    assert!(
        content.contains("-Attachment-2-"),
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
        title: "No Labels".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
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
            title: format!("Task {}", i),
            labels: vec![],
            category: None,
            attachments: vec![],
            interactive: false,
            no_interactive: true,
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
fn test_new_nested_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Nested Task".to_string(),
        labels: vec![],
        category: Some("level1/level2".to_string()),
        attachments: vec![],
        interactive: false,
        no_interactive: true,
    };

    commands::new(args).expect("new should succeed with nested category");

    let nested_path = env.stack_path().join("level1").join("level2");
    assert!(nested_path.exists(), "Nested category should be created");
}

// =============================================================================
// Additional Update Command Tests
// =============================================================================

#[test]
fn test_update_multiple_labels_at_once() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    let args = UpdateArgs {
        id: "260101".to_string(),
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
        id: "260101".to_string(),
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
        id: "260101".to_string(),
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
        id: "260101".to_string(),
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
        id: "260101".to_string(),
        title: None,
        labels: vec![],
        category: Some("bugs".to_string()),
        clear_category: false,
    };
    commands::update(args).expect("update should succeed");

    // Then clear category
    let args = UpdateArgs {
        id: "260101".to_string(),
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
        id: "260101-ABCDEFG".to_string(), // Full ID
        title: Some("Updated".to_string()),
        labels: vec![],
        category: None,
        clear_category: false,
    };

    let result = commands::update(args);
    assert!(result.is_ok(), "update with full ID should succeed");
}

// =============================================================================
// Additional Close/Reopen Tests
// =============================================================================

#[test]
fn test_close_already_closed() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);
    execute_close("260101").expect("first close should succeed");

    // Try to close again
    let result = execute_close("260101");
    assert!(result.is_err(), "closing already closed item should fail");
}

#[test]
fn test_reopen_already_open() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    // Try to reopen an already open item
    let result = execute_reopen("260101");
    assert!(result.is_err(), "reopening already open item should fail");
}

#[test]
fn test_close_with_partial_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-ABCDEFG", "Task", "open", &[], None);

    // Close with minimal partial ID
    let result = execute_close("2601");
    assert!(result.is_ok(), "close with partial ID should succeed");

    let archive_files = env.list_archive_files();
    assert_eq!(archive_files.len(), 1, "Item should be in archive");
}

#[test]
fn test_reopen_with_partial_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-ABCDEFG", "Task", "open", &[], None);
    execute_close("260101").expect("close should succeed");

    // Reopen with minimal partial ID
    let result = execute_reopen("2601");
    assert!(result.is_ok(), "reopen with partial ID should succeed");

    let stack_files = env.list_stack_files();
    assert_eq!(stack_files.len(), 1, "Item should be back in stack");
}

#[test]
fn test_close_nonexistent_item() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let result = execute_close("nonexistent");
    assert!(result.is_err(), "close nonexistent item should fail");
}

#[test]
fn test_reopen_nonexistent_item() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let result = execute_reopen("nonexistent");
    assert!(result.is_err(), "reopen nonexistent item should fail");
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

    execute_close("260101").expect("close should succeed");
    execute_reopen("260101").expect("reopen should succeed");

    let item = env.find_item_by_id("260101").expect("item should exist");
    let content = env.read_item(&item);
    assert!(content.contains("- bug"), "Bug label should be preserved");
    assert!(
        content.contains("- urgent"),
        "Urgent label should be preserved"
    );
}

// =============================================================================
// Error Cases: Project Not Initialized
// =============================================================================

#[test]
fn test_new_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    // Don't call init

    let args = NewArgs {
        title: "Task".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
    };

    let result = commands::new(args);
    assert!(result.is_err(), "new without init should fail");
}

#[test]
fn test_list_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    // Don't call init

    let filter = ListFilter {
        open: false,
        closed: false,
        label: None,
        author: None,
        sort: SortBy::Id,
        interactive: false,
        no_interactive: true,
    };

    let result = commands::list(&filter);
    assert!(result.is_err(), "list without init should fail");
}

#[test]
fn test_search_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    // Don't call init

    let args = SearchArgs {
        query: "test".to_string(),
        full_text: false,
        interactive: false,
        no_interactive: true,
        closed: false,
    };

    let result = commands::search(&args);
    assert!(result.is_err(), "search without init should fail");
}

#[test]
fn test_update_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    // Don't call init

    let args = UpdateArgs {
        id: "260101".to_string(),
        title: Some("New".to_string()),
        labels: vec![],
        category: None,
        clear_category: false,
    };

    let result = commands::update(args);
    assert!(result.is_err(), "update without init should fail");
}

#[test]
fn test_close_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    // Don't call init

    let result = execute_close("260101");
    assert!(result.is_err(), "close without init should fail");
}

#[test]
fn test_reopen_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    // Don't call init

    let result = execute_reopen("260101");
    assert!(result.is_err(), "reopen without init should fail");
}

// =============================================================================
// Additional Edge Cases
// =============================================================================

#[test]
fn test_category_with_special_characters() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    // Category names should be sanitized
    let args = NewArgs {
        title: "Task".to_string(),
        labels: vec![],
        category: Some("my-category_v2".to_string()),
        attachments: vec![],
        interactive: false,
        no_interactive: true,
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
        title: "Task".to_string(),
        labels: vec![
            "bug-fix".to_string(),
            "v2.0".to_string(),
            "priority_high".to_string(),
        ],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    let content = env.read_item(&files[0]);
    assert!(content.contains("- bug-fix"), "Label with dash");
    assert!(content.contains("- v2.0"), "Label with dot");
    assert!(content.contains("- priority_high"), "Label with underscore");
}

#[test]
fn test_whitespace_only_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "   ".to_string(),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
    };

    // Should create item (whitespace is trimmed to empty)
    let result = commands::new(args);
    assert!(result.is_ok(), "whitespace title should be handled");
}

#[test]
fn test_duplicate_labels_ignored() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Task".to_string(),
        labels: vec!["bug".to_string(), "bug".to_string(), "bug".to_string()],
        category: None,
        attachments: vec![],
        interactive: false,
        no_interactive: true,
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

#[test]
fn test_search_partial_word() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(
        &env,
        "260101-AAA",
        "Authentication System",
        "open",
        &[],
        None,
    );

    let args = SearchArgs {
        query: "auth".to_string(),
        full_text: false,
        interactive: false,
        no_interactive: true,
        closed: false,
    };

    let result = commands::search(&args);
    assert!(result.is_ok(), "partial word search should match");
}

#[test]
fn test_list_author_case_insensitive() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    // Author filter uses exact match but is case-insensitive
    let filter = ListFilter {
        open: false,
        closed: false,
        label: None,
        author: Some("TEST USER".to_string()), // uppercase of "Test User"
        sort: SortBy::Id,
        interactive: false,
        no_interactive: true,
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
        open: false,
        closed: false,
        label: Some("nonexistent-label".to_string()),
        author: None,
        sort: SortBy::Id,
        interactive: false,
        no_interactive: true,
    };

    // Should succeed but return empty list
    let result = commands::list(&filter);
    assert!(result.is_ok(), "list with no matching label should succeed");
}

// =============================================================================
// Labels Command Tests
// =============================================================================

#[test]
fn test_labels_empty_project() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let args = LabelsArgs {
        interactive: false,
        no_interactive: true,
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
        interactive: false,
        no_interactive: true,
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
        interactive: false,
        no_interactive: true,
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
        interactive: false,
        no_interactive: true,
    };

    let result = commands::labels(&args);
    assert!(result.is_err(), "labels without init should fail");
}

// =============================================================================
// Categories Command Tests
// =============================================================================

#[test]
fn test_categories_empty_project() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let args = CategoriesArgs {
        interactive: false,
        no_interactive: true,
    };

    let result = commands::categories(&args);
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

    let args = CategoriesArgs {
        interactive: false,
        no_interactive: true,
    };

    let result = commands::categories(&args);
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

    let args = CategoriesArgs {
        interactive: false,
        no_interactive: true,
    };

    // Should include categories from both open and archived items
    let result = commands::categories(&args);
    assert!(result.is_ok(), "categories should include archived items");
}

#[test]
fn test_categories_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    // Don't init

    let args = CategoriesArgs {
        interactive: false,
        no_interactive: true,
    };

    let result = commands::categories(&args);
    assert!(result.is_err(), "categories without init should fail");
}

// =============================================================================
// Attach Add Command Tests
// =============================================================================

#[test]
fn test_attach_add_file() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    // Create an item
    let item_path = create_test_item(&env, "260101-AAA", "Test Item", "open", &[], None);
    let item_id = "260101-AAA";

    // Create a test file
    let test_file = env.create_test_file("test.txt", "test content");

    // Attach the file
    let args = AttachAddArgs {
        id: item_id.to_string(),
        sources: vec![test_file.to_string_lossy().to_string()],
    };
    commands::attach_add(&args).expect("attach add should succeed");

    // Verify attachment file was copied
    let attachments = env.list_attachment_files(item_id);
    assert_eq!(attachments.len(), 1, "Should have one attachment file");
    assert!(
        attachments[0]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("test"),
        "Filename should contain original name"
    );

    // Verify frontmatter was updated
    let content = std::fs::read_to_string(&item_path).unwrap();
    assert!(
        content.contains("attachments:"),
        "Item should have attachments field"
    );
}

#[test]
fn test_attach_add_url() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    let item_path = create_test_item(&env, "260101-AAA", "Test Item", "open", &[], None);
    let item_id = "260101-AAA";

    let args = AttachAddArgs {
        id: item_id.to_string(),
        sources: vec!["https://github.com/user/repo/issues/42".to_string()],
    };
    commands::attach_add(&args).expect("attach add URL should succeed");

    // No file should be created for URLs
    let attachments = env.list_attachment_files(item_id);
    assert!(
        attachments.is_empty(),
        "URL attachments should not create files"
    );

    // Verify frontmatter was updated
    let content = std::fs::read_to_string(&item_path).unwrap();
    assert!(
        content.contains("https://github.com/user/repo/issues/42"),
        "Item should contain the URL"
    );
}

#[test]
fn test_attach_add_multiple() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    create_test_item(&env, "260101-AAA", "Test Item", "open", &[], None);
    let item_id = "260101-AAA";

    let file1 = env.create_test_file("file1.txt", "content 1");
    let file2 = env.create_test_file("file2.txt", "content 2");

    let args = AttachAddArgs {
        id: item_id.to_string(),
        sources: vec![
            file1.to_string_lossy().to_string(),
            file2.to_string_lossy().to_string(),
            "https://example.com".to_string(),
        ],
    };
    commands::attach_add(&args).expect("attach add multiple should succeed");

    let attachments = env.list_attachment_files(item_id);
    assert_eq!(attachments.len(), 2, "Should have two attachment files");
}

#[test]
fn test_attach_add_counter_increments() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    create_test_item(&env, "260101-AAA", "Test Item", "open", &[], None);
    let item_id = "260101-AAA";

    // Add first file
    let file1 = env.create_test_file("first.txt", "content 1");
    let args1 = AttachAddArgs {
        id: item_id.to_string(),
        sources: vec![file1.to_string_lossy().to_string()],
    };
    commands::attach_add(&args1).unwrap();

    // Add second file
    let file2 = env.create_test_file("second.txt", "content 2");
    let args2 = AttachAddArgs {
        id: item_id.to_string(),
        sources: vec![file2.to_string_lossy().to_string()],
    };
    commands::attach_add(&args2).unwrap();

    let attachments = env.list_attachment_files(item_id);
    assert_eq!(attachments.len(), 2);

    // Check that counters are different
    let names: Vec<String> = attachments
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();
    assert!(
        names.iter().any(|n| n.contains("-1-")),
        "Should have counter 1"
    );
    assert!(
        names.iter().any(|n| n.contains("-2-")),
        "Should have counter 2"
    );
}

#[test]
fn test_attach_add_nonexistent_file() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    create_test_item(&env, "260101-AAA", "Test Item", "open", &[], None);

    let args = AttachAddArgs {
        id: "260101-AAA".to_string(),
        sources: vec!["/nonexistent/file.txt".to_string()],
    };
    // Should succeed but with warning, not adding the file
    let result = commands::attach_add(&args);
    assert!(
        result.is_ok(),
        "Should succeed even with missing file (warning)"
    );

    let attachments = env.list_attachment_files("260101-AAA");
    assert!(attachments.is_empty(), "No attachments should be created");
}

#[test]
fn test_attach_add_nonexistent_item() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    let args = AttachAddArgs {
        id: "NONEXISTENT".to_string(),
        sources: vec!["https://example.com".to_string()],
    };
    let result = commands::attach_add(&args);
    assert!(result.is_err(), "Should fail for nonexistent item");
}

#[test]
fn test_attach_add_to_closed_item_fails() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    create_test_item(&env, "260101-AAA", "Test Item", "closed", &[], None);
    // Move to archive
    std::fs::rename(
        env.stack_path().join("260101-AAA-test-item.md"),
        env.archive_path().join("260101-AAA-test-item.md"),
    )
    .unwrap();

    let args = AttachAddArgs {
        id: "260101-AAA".to_string(),
        sources: vec!["https://example.com".to_string()],
    };
    let result = commands::attach_add(&args);
    assert!(result.is_err(), "Should fail for closed item");
}

#[test]
fn test_attach_add_empty_sources_fails() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    create_test_item(&env, "260101-AAA", "Test Item", "open", &[], None);

    let args = AttachAddArgs {
        id: "260101-AAA".to_string(),
        sources: vec![],
    };
    let result = commands::attach_add(&args);
    assert!(result.is_err(), "Should fail with empty sources");
}

#[test]
fn test_attach_add_to_item_in_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    create_test_item(&env, "260101-AAA", "Test Item", "open", &[], Some("bugs"));

    let test_file = env.create_test_file("test.txt", "content");
    let args = AttachAddArgs {
        id: "260101-AAA".to_string(),
        sources: vec![test_file.to_string_lossy().to_string()],
    };
    commands::attach_add(&args).expect("attach add in category should succeed");

    // Verify attachment is in category directory
    let category_path = env.stack_path().join("bugs");
    let attachments: Vec<_> = std::fs::read_dir(&category_path)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.contains("-Attachment-"))
        })
        .collect();
    assert_eq!(
        attachments.len(),
        1,
        "Attachment should be in category directory"
    );
}

// =============================================================================
// Attach Remove Command Tests
// =============================================================================

#[test]
fn test_attach_remove_single() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    let item_id = "260101-AAA";
    create_test_item_with_attachments(
        &env,
        item_id,
        "Test Item",
        "open",
        &[
            &format!("{item_id}-Attachment-1-file.txt"),
            "https://example.com",
        ],
        None,
    );

    let args = AttachRemoveArgs {
        id: item_id.to_string(),
        indices: vec![1], // Remove the file attachment
    };
    commands::attach_remove(&args).expect("attach remove should succeed");

    // Verify file was deleted
    let attachments = env.list_attachment_files(item_id);
    assert!(attachments.is_empty(), "File attachment should be deleted");
}

#[test]
fn test_attach_remove_multiple() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    let item_id = "260101-AAA";
    create_test_item_with_attachments(
        &env,
        item_id,
        "Test Item",
        "open",
        &[
            &format!("{item_id}-Attachment-1-a.txt"),
            &format!("{item_id}-Attachment-2-b.txt"),
            &format!("{item_id}-Attachment-3-c.txt"),
        ],
        None,
    );

    let args = AttachRemoveArgs {
        id: item_id.to_string(),
        indices: vec![1, 3], // Remove first and third
    };
    commands::attach_remove(&args).expect("attach remove multiple should succeed");

    let attachments = env.list_attachment_files(item_id);
    assert_eq!(attachments.len(), 1, "Should have one attachment left");
    assert!(
        attachments[0]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("-2-"),
        "Middle attachment should remain"
    );
}

#[test]
fn test_attach_remove_url_only_updates_frontmatter() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    let item_id = "260101-AAA";
    let item_path = create_test_item_with_attachments(
        &env,
        item_id,
        "Test Item",
        "open",
        &["https://example.com", "https://other.com"],
        None,
    );

    let args = AttachRemoveArgs {
        id: item_id.to_string(),
        indices: vec![1],
    };
    commands::attach_remove(&args).expect("remove URL should succeed");

    let content = std::fs::read_to_string(&item_path).unwrap();
    assert!(
        !content.contains("https://example.com"),
        "First URL should be removed"
    );
    assert!(
        content.contains("https://other.com"),
        "Second URL should remain"
    );
}

#[test]
fn test_attach_remove_invalid_index() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    let item_id = "260101-AAA";
    create_test_item_with_attachments(
        &env,
        item_id,
        "Test Item",
        "open",
        &["https://example.com"],
        None,
    );

    let args = AttachRemoveArgs {
        id: item_id.to_string(),
        indices: vec![5], // Only 1 attachment exists
    };
    let result = commands::attach_remove(&args);
    assert!(result.is_err(), "Should fail with invalid index");
}

#[test]
fn test_attach_remove_from_empty_item() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    create_test_item(&env, "260101-AAA", "Test Item", "open", &[], None);

    let args = AttachRemoveArgs {
        id: "260101-AAA".to_string(),
        indices: vec![1],
    };
    let result = commands::attach_remove(&args);
    assert!(result.is_err(), "Should fail when item has no attachments");
}

// =============================================================================
// Attachments List Command Tests
// =============================================================================

#[test]
fn test_attachments_list_empty() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    create_test_item(&env, "260101-AAA", "Test Item", "open", &[], None);

    let args = AttachmentsArgs {
        id: "260101-AAA".to_string(),
    };
    // Should succeed but show "No attachments"
    let result = commands::attachments(&args);
    assert!(result.is_ok(), "attachments list should succeed for empty");
}

#[test]
fn test_attachments_list_mixed() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    let item_id = "260101-AAA";
    create_test_item_with_attachments(
        &env,
        item_id,
        "Test Item",
        "open",
        &[
            &format!("{item_id}-Attachment-1-file.txt"),
            "https://example.com",
        ],
        None,
    );

    let args = AttachmentsArgs {
        id: item_id.to_string(),
    };
    let result = commands::attachments(&args);
    assert!(result.is_ok(), "attachments list should succeed");
}

#[test]
fn test_attachments_nonexistent_item() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    let args = AttachmentsArgs {
        id: "NONEXISTENT".to_string(),
    };
    let result = commands::attachments(&args);
    assert!(result.is_err(), "Should fail for nonexistent item");
}

// =============================================================================
// Attachment Movement Tests (close/reopen/category)
// =============================================================================

#[test]
fn test_close_moves_attachments_to_archive() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    let item_id = "260101-AAA";
    create_test_item_with_attachments(
        &env,
        item_id,
        "Test Item",
        "open",
        &[&format!("{item_id}-Attachment-1-file.txt")],
        None,
    );

    // Verify attachment exists in stack
    assert_eq!(env.list_attachment_files(item_id).len(), 1);

    // Close the item
    execute_close(item_id).expect("close should succeed");

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
    create_test_item_with_attachments(
        &env,
        item_id,
        "Test Item",
        "open",
        &[&format!("{item_id}-Attachment-1-file.txt")],
        None,
    );

    // Close and then reopen
    execute_close(item_id).expect("close should succeed");
    execute_reopen(item_id).expect("reopen should succeed");

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
fn test_update_category_moves_attachments() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    let item_id = "260101-AAA";
    create_test_item_with_attachments(
        &env,
        item_id,
        "Test Item",
        "open",
        &[&format!("{item_id}-Attachment-1-file.txt")],
        None,
    );

    // Move to category
    let args = UpdateArgs {
        id: item_id.to_string(),
        title: None,
        labels: vec![],
        category: Some("bugs".to_string()),
        clear_category: false,
    };
    commands::update(args).expect("update category should succeed");

    // Verify attachment is in category directory
    let category_dir = env.stack_path().join("bugs");
    let attachments: Vec<_> = std::fs::read_dir(&category_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.contains("-Attachment-"))
        })
        .collect();
    assert_eq!(attachments.len(), 1, "Attachment should be in category");
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
        &[&format!("{item_id}-Attachment-1-file.txt")],
        Some("bugs"),
    );

    execute_close(item_id).expect("close should succeed");

    assert_eq!(
        env.list_archive_attachment_files(item_id).len(),
        1,
        "Attachment should be in archive"
    );
}
