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

use crate::{config::Config, item::Item};

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
        git::move_file(path, &dest)?;
    }

    Ok(dest)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_target_directory_no_category() {
        // This would need a mock config, skipping for now
    }
}
