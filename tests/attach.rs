//! # Attachment Command Tests
//!
//! Tests for the `qstack attachments add`, `qstack attachments remove`, and `qstack list --attachments` commands.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod common;

use common::{create_test_item, create_test_item_with_attachments, GlobalConfigBuilder, TestEnv};
use qstack::commands::{
    self, AttachAddArgs, AttachRemoveArgs, InteractiveArgs, ListFilter, ListMode, SortBy,
    StatusFilter, UpdateArgs,
};

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
// Attachments List Command Tests (via list --attachments)
// =============================================================================

fn make_attachments_filter(id: &str) -> ListFilter {
    ListFilter {
        mode: ListMode::Attachments,
        status: StatusFilter::Open,
        label: None,
        author: None,
        sort: SortBy::Id,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: Some(id.to_string()),
    }
}

#[test]
fn test_attachments_list_empty() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    create_test_item(&env, "260101-AAA", "Test Item", "open", &[], None);

    let filter = make_attachments_filter("260101-AAA");
    // Should succeed but show "No attachments"
    let result = commands::list(&filter);
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

    let filter = make_attachments_filter(item_id);
    let result = commands::list(&filter);
    assert!(result.is_ok(), "attachments list should succeed");
}

#[test]
fn test_attachments_nonexistent_item() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().unwrap();

    let filter = make_attachments_filter("NONEXISTENT");
    let result = commands::list(&filter);
    assert!(result.is_err(), "Should fail for nonexistent item");
}

// =============================================================================
// Attachment Movement Tests (update category)
// =============================================================================

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
