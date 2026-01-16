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
    constants::{ATTACHMENT_INFIX, ITEM_FILE_EXTENSION},
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

/// Walks all item files in the qstack directory.
///
/// Excludes items in the archive and template directories.
pub fn walk_items(config: &Config) -> impl Iterator<Item = PathBuf> {
    let archive_path = config.archive_path();
    let template_path = config.template_path();

    walk_markdown_files(config.stack_path(), 1, 3)
        .filter(move |p| !p.starts_with(&archive_path) && !p.starts_with(&template_path))
}

/// Walks all archived item files.
pub fn walk_archived(config: &Config) -> impl Iterator<Item = PathBuf> {
    walk_markdown_files(config.archive_path(), 1, 2)
}

/// Walks all template files.
pub fn walk_templates(config: &Config) -> impl Iterator<Item = PathBuf> {
    walk_markdown_files(config.template_path(), 1, 2)
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

/// Extracts the slug portion from a filename.
///
/// For a filename like `260116-188QYZ0-bug-report.md`, returns `"bug-report"`.
/// Returns `None` if the filename doesn't have a slug portion.
fn extract_slug_from_filename(filename: &str) -> Option<&str> {
    // Remove extension
    let stem = filename.strip_suffix(".md").unwrap_or(filename);

    // The ID is always at the start, followed by slug (if any)
    // Format: {date}-{time}{random}-{slug} or {date}-{time}{random} (no slug)
    // Find the second hyphen (after date portion) then find the next hyphen (after ID)
    let mut hyphen_count = 0;
    let mut last_hyphen_pos = None;

    for (i, c) in stem.char_indices() {
        if c == '-' {
            hyphen_count += 1;
            if hyphen_count == 2 {
                last_hyphen_pos = Some(i);
            } else if hyphen_count > 2 {
                // Found third hyphen - everything after second hyphen is slug
                return Some(&stem[last_hyphen_pos.unwrap() + 1..]);
            }
        }
    }

    // If we only found 2 hyphens, there's no slug (ID-only filename)
    None
}

/// Finds a template by reference (ID, title, or slug match).
///
/// Tries to match in order: ID (partial), title (case-insensitive substring),
/// slug (case-insensitive substring from filename).
/// Returns the full path to the template file.
pub fn find_template(config: &Config, reference: &str) -> Result<PathBuf> {
    let ref_upper = reference.to_uppercase();

    // Collect all templates
    let templates: Vec<PathBuf> = walk_templates(config).collect();

    // First, try to match by ID
    let id_matches: Vec<_> = templates
        .iter()
        .filter(|path| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .and_then(crate::id::extract_from_filename)
                .is_some_and(|id| id.to_uppercase().contains(&ref_upper))
        })
        .cloned()
        .collect();

    if id_matches.len() == 1 {
        return Ok(id_matches.into_iter().next().unwrap());
    }

    if id_matches.len() > 1 {
        let ids: Vec<_> = id_matches
            .iter()
            .filter_map(|p| p.file_stem().and_then(|s| s.to_str()))
            .collect();
        bail!(
            "Multiple templates match ID '{reference}':\n  {}",
            ids.join("\n  ")
        );
    }

    // No ID match - try title match
    let title_matches: Vec<_> = templates
        .iter()
        .filter(|path| {
            Item::load(path)
                .ok()
                .is_some_and(|item| item.title().to_uppercase().contains(&ref_upper))
        })
        .cloned()
        .collect();

    if title_matches.len() == 1 {
        return Ok(title_matches.into_iter().next().unwrap());
    }

    if title_matches.len() > 1 {
        let titles: Vec<_> = title_matches
            .iter()
            .filter_map(|p| Item::load(p).ok().map(|item| item.title().to_string()))
            .collect();
        bail!(
            "Multiple templates match title '{reference}':\n  {}",
            titles.join("\n  ")
        );
    }

    // No title match - try slug match
    let slug_matches: Vec<_> = templates
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .and_then(extract_slug_from_filename)
                .is_some_and(|slug| slug.to_uppercase().contains(&ref_upper))
        })
        .collect();

    match slug_matches.len() {
        0 => bail!("No template found matching '{reference}'"),
        1 => Ok(slug_matches.into_iter().next().unwrap()),
        _ => {
            let slugs: Vec<_> = slug_matches
                .iter()
                .filter_map(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .and_then(extract_slug_from_filename)
                        .map(String::from)
                })
                .collect();
            bail!(
                "Multiple templates match slug '{reference}':\n  {}",
                slugs.join("\n  ")
            );
        }
    }
}

/// Determines the target directory for an item based on its category.
pub fn target_directory(config: &Config, category: Option<&str>) -> PathBuf {
    category.map_or_else(|| config.stack_path(), |cat| config.category_path(cat))
}

/// Derives the category from an item's file path.
///
/// Returns `Some(category)` if the item is in a category subdirectory,
/// or `None` if it's in the root of qstack/archive/templates.
///
/// Works for active items (in `stack_path`), archived items (in `archive_path`),
/// and templates (in `template_path`).
pub fn derive_category(config: &Config, path: &Path) -> Option<String> {
    let stack_path = config.stack_path();
    let archive_path = config.archive_path();
    let template_path = config.template_path();

    // Canonicalize paths to handle symlinks (e.g., /var -> /private/var on macOS)
    let path = path.canonicalize().ok()?;
    let stack_path = stack_path.canonicalize().ok()?;
    let archive_path = archive_path.canonicalize().unwrap_or(archive_path);
    let template_path = template_path.canonicalize().unwrap_or(template_path);

    // Determine base path (template, archive, or qstack)
    let relative = if path.starts_with(&template_path) {
        path.strip_prefix(&template_path).ok()?
    } else if path.starts_with(&archive_path) {
        path.strip_prefix(&archive_path).ok()?
    } else if path.starts_with(&stack_path) {
        path.strip_prefix(&stack_path).ok()?
    } else {
        return None;
    };

    // Get parent directory relative to base
    let parent = relative.parent()?;

    // If parent is empty (item in root), no category
    if parent.as_os_str().is_empty() {
        return None;
    }

    // First component is the category
    let category = parent.iter().next()?.to_str()?;

    // Don't treat archive or template dir as category (shouldn't happen with new structure)
    if category == config.archive_dir() || category == config.template_dir() {
        return None;
    }

    Some(category.to_string())
}

/// Creates a new item file and returns its path.
pub fn create_item(config: &Config, item: &Item, category: Option<&str>) -> Result<PathBuf> {
    let dir = target_directory(config, category);

    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create directory: {}", dir.display()))?;

    let path = dir.join(item.filename());

    item.save(&path)?;

    Ok(path)
}

/// Creates a new template file and returns its path.
///
/// Templates are stored in the `.templates/` directory (or category subdirectory).
pub fn create_template(config: &Config, item: &Item, category: Option<&str>) -> Result<PathBuf> {
    let base = config.template_path();
    let dir = category.map_or_else(|| base.clone(), |cat| base.join(cat));

    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create template directory: {}", dir.display()))?;

    let path = dir.join(item.filename());

    item.save(&path)?;

    Ok(path)
}

/// Internal helper to move an item to a destination directory.
///
/// Handles: creating dest dir, moving attachments, moving file via git, cleanup.
fn move_item_to_dir(
    config: &Config,
    path: &Path,
    dest_dir: &Path,
) -> Result<(PathBuf, Vec<String>)> {
    let filename = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

    std::fs::create_dir_all(dest_dir)?;

    let dest = dest_dir.join(filename);

    // Short-circuit if already in correct location
    if path == dest {
        return Ok((dest, Vec::new()));
    }

    // Remember source directory for cleanup
    let src_dir = path.parent().map(Path::to_path_buf);

    // Move attachments first
    let warnings = src_dir
        .as_ref()
        .map_or_else(Vec::new, |dir| move_attachments(dir, dest_dir, path));

    git::move_file(path, &dest)?;

    // Clean up empty source directory if it was a category
    if let Some(src_dir) = src_dir {
        cleanup_empty_category_dir(config, &src_dir);
    }

    Ok((dest, warnings))
}

/// Moves an item to the archive.
///
/// Preserves category folder structure in archive.
/// Returns the new path and any warnings from moving attachments.
pub fn archive_item(config: &Config, path: &Path) -> Result<(PathBuf, Vec<String>)> {
    let category = derive_category(config, path);
    let archive_base = config.archive_path();
    let dest_dir = category
        .as_deref()
        .map_or_else(|| archive_base.clone(), |cat| archive_base.join(cat));

    move_item_to_dir(config, path, &dest_dir)
}

/// Moves an item from the archive back to qstack.
///
/// Derives category from archive path structure and restores to same category.
/// Returns the new path and any warnings from moving attachments.
pub fn unarchive_item(config: &Config, path: &Path) -> Result<(PathBuf, Vec<String>)> {
    let category = derive_category(config, path);
    let dest_dir = target_directory(config, category.as_deref());

    move_item_to_dir(config, path, &dest_dir)
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
    let dest_dir = target_directory(config, category);
    move_item_to_dir(config, path, &dest_dir)
}

/// Removes an empty category directory if it's safe to do so.
///
/// Only removes directories that:
/// - Are inside the qstack directory (including archive)
/// - Are not the qstack root or archive root
/// - Are empty
fn cleanup_empty_category_dir(config: &Config, dir: &Path) {
    let stack_path = config.stack_path();
    let archive_path = config.archive_path();

    // Never remove root directories
    if dir == stack_path || dir == archive_path {
        return;
    }

    // Only clean up directories inside qstack (which includes archive)
    if !dir.starts_with(&stack_path) {
        return;
    }

    // Check if directory is empty
    if let Ok(mut entries) = std::fs::read_dir(dir) {
        if entries.next().is_none() {
            // Directory is empty, remove it
            let _ = std::fs::remove_dir(dir);
        }
    }
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

        // Split by attachment infix
        let (item_id, rest) = stem.split_once(ATTACHMENT_INFIX)?;

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
            || {
                format!(
                    "{}{}{}-{}",
                    self.item_id, ATTACHMENT_INFIX, self.counter, self.name
                )
            },
            |ext| {
                format!(
                    "{}{}{}-{}.{}",
                    self.item_id, ATTACHMENT_INFIX, self.counter, self.name, ext
                )
            },
        )
    }

    /// Returns the prefix used to find attachments for an item.
    pub fn prefix_for_item(item_id: &str) -> String {
        format!("{item_id}{ATTACHMENT_INFIX}")
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
