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
    constants::ITEM_FILE_EXTENSION,
    id,
    item::{slugify, Item},
};

/// Walks markdown files in a directory with specified depth constraints.
fn walk_markdown_files(
    path: PathBuf,
    min_depth: usize,
    max_depth: usize,
) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(path)
        .min_depth(min_depth)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == ITEM_FILE_EXTENSION)
        })
        .map(walkdir::DirEntry::into_path)
}

/// Walks all item files in the stack directory.
pub fn walk_items(config: &Config) -> impl Iterator<Item = PathBuf> {
    let archive_path = config.archive_path();

    walk_markdown_files(config.stack_path(), 1, 3).filter(move |p| !p.starts_with(&archive_path))
}

/// Walks all archived item files.
pub fn walk_archived(config: &Config) -> impl Iterator<Item = PathBuf> {
    walk_markdown_files(config.archive_path(), 1, 2)
}

/// Walks all items (both active and archived).
pub fn walk_all(config: &Config) -> impl Iterator<Item = PathBuf> {
    walk_items(config).chain(walk_archived(config))
}

/// Loads all items (both active and archived) into memory.
///
/// Silently skips items that fail to parse.
pub fn load_all_items(config: &Config) -> Vec<Item> {
    walk_all(config)
        .filter_map(|path| Item::load(&path).ok())
        .collect()
}

/// An item loaded from disk along with its path.
pub struct LoadedItem {
    /// The path to the item file
    pub path: PathBuf,
    /// The loaded item
    pub item: Item,
}

/// Finds and loads an item by partial ID match.
///
/// Convenience wrapper that combines `find_by_id` and `Item::load`.
pub fn find_and_load(config: &Config, partial_id: &str) -> Result<LoadedItem> {
    let path = find_by_id(config, partial_id)?;
    let item = Item::load(&path)?;
    Ok(LoadedItem { path, item })
}

/// Loads an item from a file path.
///
/// The path can be absolute or relative to the current working directory.
pub fn load_from_file(file_path: &Path) -> Result<LoadedItem> {
    let path = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("Failed to get current directory")?
            .join(file_path)
    };

    if !path.exists() {
        bail!("File not found: {}", file_path.display());
    }

    let item = Item::load(&path)?;
    Ok(LoadedItem { path, item })
}

/// Specifies how to identify an item - either by ID or file path.
#[derive(Debug, Clone)]
pub enum ItemRef {
    /// Partial ID match
    Id(String),
    /// Direct file path
    File(std::path::PathBuf),
}

impl ItemRef {
    /// Creates an `ItemRef` from optional id and file arguments.
    ///
    /// Returns an error if neither or both are specified.
    pub fn from_options(id: Option<String>, file: Option<std::path::PathBuf>) -> Result<Self> {
        match (id, file) {
            (Some(id), None) => Ok(Self::Id(id)),
            (None, Some(file)) => Ok(Self::File(file)),
            (None, None) => bail!("Either --id or --file must be specified"),
            (Some(_), Some(_)) => bail!("Cannot specify both --id and --file"),
        }
    }

    /// Resolves the reference to a loaded item.
    pub fn resolve(&self, config: &Config) -> Result<LoadedItem> {
        match self {
            Self::Id(id) => find_and_load(config, id),
            Self::File(path) => load_from_file(path),
        }
    }
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
                .and_then(crate::id::extract_from_filename)
                .is_some_and(|id| id.to_uppercase().contains(&partial_upper))
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
///
/// Returns the new path and any warnings from moving attachments.
pub fn archive_item(config: &Config, path: &Path) -> Result<(PathBuf, Vec<String>)> {
    let filename = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

    let archive_path = config.archive_path();
    std::fs::create_dir_all(&archive_path)?;

    // Move attachments first
    let warnings = path.parent().map_or_else(Vec::new, |src_dir| {
        move_attachments(src_dir, &archive_path, path)
    });

    let dest = archive_path.join(filename);
    git::move_file(path, &dest)?;

    Ok((dest, warnings))
}

/// Moves an item from the archive back to the stack.
///
/// Returns the new path and any warnings from moving attachments.
pub fn unarchive_item(
    config: &Config,
    path: &Path,
    category: Option<&str>,
) -> Result<(PathBuf, Vec<String>)> {
    let filename = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

    let dest_dir = target_directory(config, category);
    std::fs::create_dir_all(&dest_dir)?;

    // Move attachments first
    let warnings = path.parent().map_or_else(Vec::new, |src_dir| {
        move_attachments(src_dir, &dest_dir, path)
    });

    let dest = dest_dir.join(filename);
    git::move_file(path, &dest)?;

    Ok((dest, warnings))
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
///
/// Returns the new path and any warnings from moving attachments.
pub fn move_to_category(
    config: &Config,
    path: &Path,
    category: Option<&str>,
) -> Result<(PathBuf, Vec<String>)> {
    let filename = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

    let dest_dir = target_directory(config, category);
    std::fs::create_dir_all(&dest_dir)?;

    let dest = dest_dir.join(filename);

    let warnings = if path == dest {
        Vec::new()
    } else {
        // Move attachments first
        let warnings = path.parent().map_or_else(Vec::new, |src_dir| {
            move_attachments(src_dir, &dest_dir, path)
        });
        git::move_file(path, &dest)?;
        warnings
    };

    Ok((dest, warnings))
}

// =============================================================================
// Attachment Operations
// =============================================================================

/// Encapsulates the attachment filename convention: `{item_id}-Attachment-{counter}-{name}.{ext}`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentFileName {
    /// Item ID prefix
    pub item_id: String,
    /// Attachment counter (1-based)
    pub counter: u32,
    /// Slugified name
    pub name: String,
    /// File extension (without dot), if any
    pub extension: Option<String>,
}

impl AttachmentFileName {
    /// Creates a new attachment filename.
    pub fn new(item_id: &str, counter: u32, name: &str, extension: Option<&str>) -> Self {
        Self {
            item_id: item_id.to_string(),
            counter,
            name: name.to_string(),
            extension: extension.map(String::from),
        }
    }

    /// Parses an attachment filename.
    ///
    /// Expected format: `{item_id}-Attachment-{counter}-{name}.{ext}`
    pub fn parse(filename: &str) -> Option<Self> {
        // Remove extension
        let (stem, extension) = filename.rfind('.').map_or((filename, None), |dot_pos| {
            (&filename[..dot_pos], Some(&filename[dot_pos + 1..]))
        });

        // Split by "-Attachment-"
        let (item_id, rest) = stem.split_once("-Attachment-")?;

        // Extract counter and name
        let (counter_str, name) = rest.split_once('-').unwrap_or((rest, "file"));
        let counter = counter_str.parse().ok()?;

        Some(Self {
            item_id: item_id.to_string(),
            counter,
            name: name.to_string(),
            extension: extension.map(String::from),
        })
    }

    /// Returns the full filename string.
    pub fn to_filename(&self) -> String {
        self.extension.as_ref().map_or_else(
            || format!("{}-Attachment-{}-{}", self.item_id, self.counter, self.name),
            |ext| {
                format!(
                    "{}-Attachment-{}-{}.{}",
                    self.item_id, self.counter, self.name, ext
                )
            },
        )
    }

    /// Returns the prefix used to find attachments for an item.
    pub fn prefix_for_item(item_id: &str) -> String {
        format!("{item_id}-Attachment-")
    }
}

/// Result of processing a single attachment.
#[derive(Debug)]
pub enum AttachmentResult {
    /// URL was added directly to frontmatter
    UrlAdded(String),
    /// File was copied and added
    FileCopied { original: String, new_name: String },
    /// File was not found
    FileNotFound(String),
}

/// Processes a single attachment source (file path or URL).
///
/// - URLs are returned as-is for adding to frontmatter
/// - Files are copied to the item directory with a standardized name
///
/// Returns `AttachmentResult` indicating what happened.
pub fn process_attachment(
    source: &str,
    item: &mut crate::item::Item,
    item_dir: &Path,
    item_id: &str,
) -> Result<AttachmentResult> {
    use crate::item::is_url;

    if is_url(source) {
        item.add_attachment(source.to_string());
        return Ok(AttachmentResult::UrlAdded(source.to_string()));
    }

    // File attachment - resolve path
    let source_path = Path::new(source);
    let source_path = if source_path.is_relative() {
        std::env::current_dir()?.join(source_path)
    } else {
        source_path.to_path_buf()
    };

    if !source_path.exists() {
        return Ok(AttachmentResult::FileNotFound(
            source_path.display().to_string(),
        ));
    }

    let counter = item.next_attachment_counter();
    let new_filename = copy_attachment(&source_path, item_dir, item_id, counter)?;
    item.add_attachment(new_filename.clone());

    Ok(AttachmentResult::FileCopied {
        original: source.to_string(),
        new_name: new_filename,
    })
}

/// Copies a file as an attachment to the item's directory.
///
/// Returns the new filename using the standard attachment naming convention.
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

    let extension = source.extension().and_then(|s| s.to_str());

    // Create slugified name
    let slug = slugify(original_name);
    let slug_part = if slug.is_empty() { "file" } else { &slug };

    // Build filename using the struct
    let attachment = AttachmentFileName::new(item_id, counter, slug_part, extension);
    let new_filename = attachment.to_filename();

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
/// Looks for files matching the attachment naming convention.
pub fn find_attachment_files(item_dir: &Path, item_id: &str) -> Vec<PathBuf> {
    let prefix = AttachmentFileName::prefix_for_item(item_id);

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
/// Returns a list of warnings for any attachments that failed to move.
fn move_attachments(src_dir: &Path, dest_dir: &Path, item_path: &Path) -> Vec<String> {
    let mut warnings = Vec::new();

    // Extract item ID from the item filename
    let Some(item_id) = item_path
        .file_name()
        .and_then(|s| s.to_str())
        .and_then(id::extract_from_filename)
    else {
        return warnings; // Can't determine ID, skip attachment move
    };

    let attachments = find_attachment_files(src_dir, item_id);

    for attachment_path in attachments {
        if let Some(filename) = attachment_path.file_name() {
            let dest_path = dest_dir.join(filename);
            // Use git mv for tracked files, falls back to rename
            if let Err(e) = git::move_file(&attachment_path, &dest_path) {
                warnings.push(format!(
                    "Failed to move attachment {}: {}",
                    attachment_path.display(),
                    e
                ));
            }
        }
    }

    warnings
}

// Tests for storage are in tests/integration.rs as they require
// a full test harness with project setup.
