//! # Search Command Tests
//!
//! Tests for the `qstack search` command.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod common;

use common::{create_test_item, GlobalConfigBuilder, TestEnv};
use qstack::commands::{self, InteractiveArgs, SearchArgs};

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
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
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
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
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
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
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
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        closed: false,
    };

    let result = commands::search(&args);
    assert!(result.is_err(), "search with no results should error");
}

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
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
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
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
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
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
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
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
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
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        closed: false,
    };

    let result = commands::search(&args);
    assert!(
        result.is_ok(),
        "search with multiple matches should succeed"
    );
}

#[test]
fn test_search_without_init() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    // Don't call init

    let args = SearchArgs {
        query: "test".to_string(),
        full_text: false,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        closed: false,
    };

    let result = commands::search(&args);
    assert!(result.is_err(), "search without init should fail");
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
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        closed: false,
    };

    let result = commands::search(&args);
    assert!(result.is_ok(), "partial word search should match");
}

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
            interactive: InteractiveArgs {
                interactive: false,
                no_interactive: true,
            },
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
            interactive: InteractiveArgs {
                interactive: false,
                no_interactive: false,
            },
            closed: false,
        };

        commands::search(&args).expect("search should succeed");
    }
}
