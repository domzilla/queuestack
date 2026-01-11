//! # Storage
//!
//! File system operations for qstack items.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

pub mod git;

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use walkdir::WalkDir;

use crate::{
    config::Config,
    item::{slugify, Item},
};

/// Walks all item files in the stack directory.
pub fn walk_items(config: &Config) -> impl Iterator<Item = PathBuf> {
    let stack_path = config.stack_path();
    let archive_path = config.archive_path();

    WalkDir::new(&stack_path)
        .min_depth(1)
        .max_depth(3)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .filter(move |e| !e.path().starts_with(&archive_path))
        .map(walkdir::DirEntry::into_path)
}

/// Walks all archived item files.
pub fn walk_archived(config: &Config) -> impl Iterator<Item = PathBuf> {
    let archive_path = config.archive_path();

    WalkDir::new(&archive_path)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .map(walkdir::DirEntry::into_path)
}

/// Walks all items (both active and archived).
pub fn walk_all(config: &Config) -> impl Iterator<Item = PathBuf> {
    walk_items(config).chain(walk_archived(config))
}

/// Finds an item by partial ID match.
///
/// Returns the full path to the item file.
pub fn find_by_id(config: &Config, partial_id: &str) -> Result<PathBuf> {
    let partial_upper = partial_id.to_uppercase();

    let matches: Vec<_> = walk_all(config)
        .filter(|path| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|name| name.to_uppercase().starts_with(&partial_upper))
        })
        .collect();

    match matches.len() {
        0 => bail!("No item found matching '{partial_id}'"),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => {
            let ids: Vec<_> = matches
                .iter()
                .filter_map(|p| p.file_stem().and_then(|s| s.to_str()))
                .collect();
            bail!(
                "Multiple items match '{partial_id}':\n  {}",
                ids.join("\n  ")
            );
        }
    }
}

/// Determines the target directory for an item based on its category.
pub fn target_directory(config: &Config, category: Option<&str>) -> PathBuf {
    category.map_or_else(|| config.stack_path(), |cat| config.category_path(cat))
}

/// Creates a new item file and returns its path.
pub fn create_item(config: &Config, item: &Item) -> Result<PathBuf> {
    let dir = target_directory(config, item.category());

    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create directory: {}", dir.display()))?;

    let path = dir.join(item.filename());

    item.save(&path)?;

    Ok(path)
}

/// Moves an item to the archive.
pub fn archive_item(config: &Config, path: &Path) -> Result<PathBuf> {
    let filename = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

    let archive_path = config.archive_path();
    std::fs::create_dir_all(&archive_path)?;

    // Move attachments first
    if let Some(src_dir) = path.parent() {
        move_attachments(src_dir, &archive_path, path);
    }

    let dest = archive_path.join(filename);
    git::move_file(path, &dest)?;

    Ok(dest)
}

/// Moves an item from the archive back to the stack.
pub fn unarchive_item(config: &Config, path: &Path, category: Option<&str>) -> Result<PathBuf> {
    let filename = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

    let dest_dir = target_directory(config, category);
    std::fs::create_dir_all(&dest_dir)?;

    // Move attachments first
    if let Some(src_dir) = path.parent() {
        move_attachments(src_dir, &dest_dir, path);
    }

    let dest = dest_dir.join(filename);
    git::move_file(path, &dest)?;

    Ok(dest)
}

/// Renames an item file (when title changes).
pub fn rename_item(path: &Path, new_filename: &str) -> Result<PathBuf> {
    let dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;
    let new_path = dir.join(new_filename);

    if path != new_path {
        git::move_file(path, &new_path)?;
    }

    Ok(new_path)
}

/// Moves an item to a different category.
pub fn move_to_category(config: &Config, path: &Path, category: Option<&str>) -> Result<PathBuf> {
    let filename = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

    let dest_dir = target_directory(config, category);
    std::fs::create_dir_all(&dest_dir)?;

    let dest = dest_dir.join(filename);

    if path != dest {
        // Move attachments first
        if let Some(src_dir) = path.parent() {
            move_attachments(src_dir, &dest_dir, path);
        }
        git::move_file(path, &dest)?;
    }

    Ok(dest)
}

// =============================================================================
// Attachment Operations
// =============================================================================

/// Copies a file as an attachment to the item's directory.
///
/// Returns the new filename: `{item_id}-Attachment-{counter}-{slugified_name}.{ext}`
pub fn copy_attachment(
    source: &Path,
    item_dir: &Path,
    item_id: &str,
    counter: u32,
) -> Result<String> {
    // Get original filename parts
    let original_name = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("attachment");

    let extension = source.extension().and_then(|s| s.to_str()).unwrap_or("");

    // Create slugified name
    let slug = slugify(original_name);
    let slug_part = if slug.is_empty() {
        "file".to_string()
    } else {
        slug
    };

    // Build new filename
    let new_filename = if extension.is_empty() {
        format!("{item_id}-Attachment-{counter}-{slug_part}")
    } else {
        format!("{item_id}-Attachment-{counter}-{slug_part}.{extension}")
    };

    let dest = item_dir.join(&new_filename);

    // Copy the file
    std::fs::copy(source, &dest).with_context(|| {
        format!(
            "Failed to copy attachment: {} -> {}",
            source.display(),
            dest.display()
        )
    })?;

    Ok(new_filename)
}

/// Deletes an attachment file.
///
/// Uses `trash` command if available (macOS), otherwise uses git rm or standard remove.
pub fn delete_attachment(item_dir: &Path, filename: &str) -> Result<()> {
    let path = item_dir.join(filename);

    if !path.exists() {
        // File already gone, nothing to do
        return Ok(());
    }

    // Try to use trash command (macOS) for safe deletion
    #[cfg(target_os = "macos")]
    {
        use std::process::{Command, Stdio};

        let status = Command::new("trash")
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if status.is_ok_and(|s| s.success()) {
            return Ok(());
        }
        // Fall through to git rm / standard remove
    }

    // Use git rm if in a git repo, otherwise standard remove
    git::remove_file(&path)
}

/// Finds all attachment files for an item in a directory.
///
/// Looks for files matching `{item_id}-Attachment-*`
pub fn find_attachment_files(item_dir: &Path, item_id: &str) -> Vec<PathBuf> {
    let prefix = format!("{item_id}-Attachment-");

    if !item_dir.exists() {
        return Vec::new();
    }

    std::fs::read_dir(item_dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
        })
        .collect()
}

/// Moves attachment files alongside an item.
///
/// Called internally when archiving, unarchiving, or moving items between categories.
fn move_attachments(src_dir: &Path, dest_dir: &Path, item_path: &Path) {
    // Extract item ID from the item filename
    let item_id = item_path
        .file_stem()
        .and_then(|s| s.to_str())
        .and_then(|name| {
            // ID is everything before the first dash after the pattern YYMMDD-XXXXX
            // Format: "260109-02F7K9M-title-slug.md" -> "260109-02F7K9M"
            let parts: Vec<&str> = name.splitn(3, '-').collect();
            if parts.len() >= 2 {
                Some(format!("{}-{}", parts[0], parts[1]))
            } else {
                None
            }
        });

    let Some(item_id) = item_id else {
        return; // Can't determine ID, skip attachment move
    };

    let attachments = find_attachment_files(src_dir, &item_id);

    for attachment_path in attachments {
        if let Some(filename) = attachment_path.file_name() {
            let dest_path = dest_dir.join(filename);
            // Use git mv for tracked files, falls back to rename
            if let Err(e) = git::move_file(&attachment_path, &dest_path) {
                // Log warning but continue - attachment might have been manually deleted
                eprintln!(
                    "Warning: Failed to move attachment {}: {}",
                    attachment_path.display(),
                    e
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_target_directory_no_category() {
        // This would need a mock config, skipping for now
    }
}
