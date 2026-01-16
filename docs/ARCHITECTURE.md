# qstack Architecture

This document describes the internal architecture of qstack, a minimal task and issue tracker designed for scriptability and agent-driven workflows.

## Design Principles

1. **Plain text first** — Items are Markdown files that can be read, edited, and searched with standard Unix tools
2. **Scriptable** — Every command works non-interactively via `--no-interactive` flag; items can be identified by `--id` or `--file` for shell completion and piping
3. **Git-aware** — File operations use `git mv` when available to preserve history
4. **Layered configuration** — Project settings override global defaults
5. **Minimal dependencies** — Only what's necessary for the task

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI Layer                               │
│                      (src/main.rs)                              │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  clap derive API: Commands → Subcommands → Args          │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Command Layer                              │
│                   (src/commands/*.rs)                           │
│  ┌─────┐ ┌─────┐ ┌──────┐ ┌────────┐ ┌──────┐ ┌───────┐ ┌──────┐│
│  │init │ │ new │ │ list │ │ search │ │update│ │ close │ │attach││
│  └─────┘ └─────┘ └──────┘ └────────┘ └──────┘ └───────┘ └──────┘│
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│   Config Layer  │ │   Item Layer    │ │  Storage Layer  │
│ (src/config/)   │ │  (src/item/)    │ │ (src/storage/)  │
│                 │ │                 │ │                 │
│ • GlobalConfig  │ │ • Frontmatter   │ │ • walk_items()  │
│ • ProjectConfig │ │ • Item          │ │ • find_by_id()  │
│ • Config (merged)││ • parser       ││ • create_item()│
│                 │ │ • search        │ │ • archive_item()│
│                 │ │ • slug          │ │ • Attachments   │
└─────────────────┘ └─────────────────┘ └─────────────────┘
              │               │               │
              └───────────────┼───────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Core Services                              │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐             │
│  │  ID Generator│ │ Git Integration│ │    Editor  │             │
│  │  (src/id/)   │ │ (storage/git) ││ (src/editor) │             │
│  └──────────────┘ └──────────────┘ └──────────────┘             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         TUI Layer                               │
│                       (src/tui/)                                │
│  ┌──────────────────┐  ┌──────────────────────────────────────┐ │
│  │     Screens      │  │              Widgets                 │ │
│  │ • select         │  │ • TextInput    • SelectList          │ │
│  │ • prompt         │  │ • TextArea     • MultiSelect         │ │
│  │ • wizard         │  │                                      │ │
│  └──────────────────┘  └──────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Module Breakdown

### Entry Point (`src/main.rs`)

The CLI entry point uses clap's derive API to define commands and arguments. Each command maps to a function in `src/commands/`.

```rust
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    New(NewArgs),
    List { ... },
    // ...
}
```

### Library Root (`src/lib.rs`)

Exports the public API for use by commands and tests:

```rust
pub mod commands;
pub mod config;
pub mod constants;
pub mod editor;
pub mod id;
pub mod item;
pub mod storage;
pub mod tui;
pub mod ui;
```

### Item Module (`src/item/`)

The core data model representing a task or issue.

#### `mod.rs` — Item and Frontmatter

```rust
pub struct Frontmatter {
    pub id: String,
    pub title: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub status: Status,
    pub labels: Vec<String>,
    pub attachments: Vec<String>,
}

pub struct Item {
    pub frontmatter: Frontmatter,
    pub body: String,
    pub path: Option<PathBuf>,
}
```

Key methods:
- `Item::load(path)` — Parse from disk
- `Item::save(path)` — Serialize to disk
- `Item::filename()` — Generate `{id}-{slug}.md`

#### `parser.rs` — YAML Frontmatter

Handles parsing and serializing Markdown files with YAML frontmatter:

```
---
id: 260109-0A2B3C4
title: Fix Login Bug
...
---

Body content here.
```

Uses `serde_yml` for YAML (de)serialization.

#### `search.rs` — Search and Filter (Single Source of Truth)

This module provides unified filtering logic used by both CLI commands and TUI.

**FilterCriteria** — Unified filter criteria:

```rust
pub struct FilterCriteria {
    pub search: String,           // Text search query
    pub labels: Vec<String>,      // Labels to filter by (OR logic)
    pub category: Option<String>, // Category filter
    pub author: Option<String>,   // Author filter
}
```

**Core filtering function:**

```rust
pub fn matches_filter(item: &Item, criteria: &FilterCriteria, item_category: Option<&str>) -> bool
```

**Reusable predicates** (for TUI which uses flattened `ItemInfo`):

```rust
pub fn matches_search_text(title: &str, id: &str, body: &str, query: &str) -> bool
pub fn matches_any_label(item_labels: &[String], filter_labels: &[String]) -> bool
pub fn matches_category_filter(item_category: Option<&str>, filter_category: &str) -> bool
pub fn matches_author_filter(item_author: &str, filter_author: &str) -> bool
```

**Simple query matching** (for search command):

```rust
pub fn matches_query(item: &Item, query: &str, full_text: bool) -> bool
```

#### `slug.rs` — Title Slugification

Converts titles to URL-safe filenames: `"Fix Login Bug"` → `"fix-login-bug"`

### Config Module (`src/config/`)

Two-tier configuration with project overriding global.

#### Resolution Order

For each setting, the system checks:
1. Project config (`.qstack`)
2. Global config (`~/.qstack`)
3. Default value

```rust
pub struct Config {
    global: GlobalConfig,
    project: ProjectConfig,
    project_root: PathBuf,
}

impl Config {
    pub fn id_pattern(&self) -> &str {
        self.project.id_pattern
            .as_deref()
            .unwrap_or(&self.global.id_pattern)
    }
}
```

#### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `user_name` | `Option<String>` | None | Author for new items |
| `use_git_user` | `bool` | `true` | Fall back to git config |
| `editor` | `Option<String>` | `$EDITOR` | Editor command |
| `interactive` | `bool` | `true` | TUI by default |
| `id_pattern` | `String` | `%y%m%d-%T%RRR` | ID format |
| `stack_dir` | `String` | `qstack` | Item directory |
| `archive_dir` | `String` | `.archive` | Archive subdirectory |
| `template_dir` | `String` | `.templates` | Template subdirectory |

### Storage Module (`src/storage/`)

File system operations for items.

#### `mod.rs` — Core Operations

```rust
// Walking items
pub fn walk_items(config: &Config) -> impl Iterator<Item = PathBuf>
pub fn walk_archived(config: &Config) -> impl Iterator<Item = PathBuf>
pub fn walk_templates(config: &Config) -> impl Iterator<Item = PathBuf>
pub fn walk_all(config: &Config) -> impl Iterator<Item = PathBuf>

// Category derivation
pub fn derive_category(config: &Config, path: &Path) -> Option<String>

// CRUD operations
pub fn find_by_id(config: &Config, partial_id: &str) -> Result<PathBuf>
pub fn create_item(config: &Config, item: &Item, category: Option<&str>) -> Result<PathBuf>
pub fn archive_item(config: &Config, path: &Path) -> Result<(PathBuf, Vec<String>)>
pub fn unarchive_item(config: &Config, path: &Path) -> Result<(PathBuf, Vec<String>)>
pub fn rename_item(path: &Path, new_filename: &str) -> Result<PathBuf>
pub fn move_to_category(config: &Config, path: &Path, category: Option<&str>) -> Result<...>

// Template operations
pub fn create_template(config: &Config, item: &Item, category: Option<&str>) -> Result<PathBuf>
pub fn find_template(config: &Config, reference: &str) -> Result<PathBuf>

// Internal helper (shared by archive/unarchive/move_to_category)
fn move_item_to_dir(config: &Config, path: &Path, dest_dir: &Path) -> Result<(PathBuf, Vec<String>)>
```

Uses `walkdir` crate for recursive directory traversal with depth limits.

#### Template Lookup

`find_template()` searches for templates using this priority order:

1. **ID match** — Partial, case-insensitive (e.g., `260109` matches `260109-0A2B3C4`)
2. **Title match** — Case-insensitive substring (e.g., `bug report` matches `Bug Report Template`)
3. **Slug match** — Extracted from filename, case-insensitive (e.g., `bug-report` matches `260109-0A2B3C4-bug-report.md`)

#### `ItemRef` — Flexible Item Identification

Commands that operate on a single item accept either `--id` or `--file`:

```rust
pub enum ItemRef {
    Id(String),
    File(PathBuf),
}

impl ItemRef {
    pub fn from_options(id: Option<String>, file: Option<PathBuf>) -> Result<Self>;
    pub fn resolve(&self, config: &Config) -> Result<LoadedItem>;
}
```

This enables:
- Shell tab completion for file paths
- Piping from `qstack list` output
- Working without knowing item IDs

#### Attachment Handling

Attachments follow the naming convention: `{item_id}-Attachment-{counter}-{name}.{ext}`

```rust
pub struct AttachmentFileName {
    pub item_id: String,
    pub counter: u32,
    pub name: String,
    pub extension: Option<String>,
}
```

Key functions:
- `process_attachment()` — Handle URL or file
- `copy_attachment()` — Copy file with standardized name
- `find_attachment_files()` — Find all attachments for an item
- `move_attachments()` — Move attachments with item

#### `git.rs` — Git Integration

Git-aware file operations that fall back to standard filesystem calls:

```rust
pub fn is_git_repo() -> bool
pub fn move_file(from: &Path, to: &Path) -> Result<()>  // Uses git mv if available
pub fn remove_file(path: &Path) -> Result<()>           // Uses git rm if available
pub fn user_name() -> Option<String>                    // git config user.name
```

### ID Module (`src/id/`)

Generates unique, sortable identifiers.

#### Pattern Tokens

| Token | Description | Example |
|-------|-------------|---------|
| `%y` | Year (2 digits) | `26` |
| `%m` | Month (01-12) | `01` |
| `%d` | Day (01-31) | `09` |
| `%j` | Day of year | `009` |
| `%T` | Time (4 chars) | `0A2B` |
| `%R` | Random char | `X` |
| `%%` | Literal `%` | `%` |

Default pattern `%y%m%d-%T%RRR` produces: `260109-0A2B3C4`

#### `base32.rs` — Encoding

Uses Crockford's Base32 alphabet (0-9, A-Z excluding I, L, O, U) for human-readable, unambiguous characters.

### TUI Module (`src/tui/`)

Interactive terminal UI using `ratatui` and `crossterm`.

#### Architecture

```
tui/
├── mod.rs          # TuiApp trait, run() function
├── terminal.rs     # Terminal setup/teardown (TerminalGuard)
├── event.rs        # Input event handling
├── screens/
│   ├── select.rs       # Item selection screen
│   ├── item_actions.rs # Interactive list with filter overlay & action menu
│   ├── prompt.rs       # Text input prompt
│   ├── confirm.rs      # Yes/no confirmation dialog
│   └── wizard.rs       # Multi-step new item wizard
└── widgets/
    ├── text_input.rs    # Single-line text input
    ├── text_area.rs     # Multi-line text editor
    ├── select_list.rs   # Single-select list
    ├── multi_select.rs  # Multi-select list
    ├── action_menu.rs   # Action menu overlay
    └── filter_overlay.rs # Filter input overlay
```

#### Item Actions Screen (`item_actions.rs`)

Full-featured interactive list for `qstack list` with:

- **Filter overlay** (`f` key) — Real-time filtering by search text, labels, category
- **Action menu** (`Enter` key) — Quick actions: view, edit, close/reopen, delete
- Uses shared filter predicates from `item/search.rs` for consistency with CLI

#### TuiApp Trait

All screens implement this trait:

```rust
pub trait TuiApp {
    type Output;
    fn handle_event(&mut self, event: &TuiEvent) -> Option<AppResult<Self::Output>>;
    fn render(&mut self, frame: &mut Frame);
}

pub fn run<A: TuiApp>(app: A) -> Result<Option<A::Output>>
```

#### TerminalGuard

RAII guard for terminal setup/teardown:

```rust
pub struct TerminalGuard { ... }

impl TerminalGuard {
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        // ...
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Restore terminal state
    }
}
```

### UI Module (`src/ui.rs`)

Shared UI utilities:

- **InteractiveArgs** — Resolves `--interactive` / `--no-interactive` flags with `is_enabled(config)` method
- **Selection dialogs** — `select_from_list()`, `select_item()` (formats items with columns for TUI)
- **Aggregation** — `count_by()`, `count_by_many()` for labels/categories
- **Output formatting** — `print_success()`, `print_warnings()`, `truncate()`

### Commands Module (`src/commands/`)

Each command is a separate file with an `execute()` function:

```rust
// src/commands/list.rs
pub fn execute(filter: ListOptions, interactive: InteractiveArgs) -> Result<()> {
    let config = Config::load()?;
    let items = load_and_filter(&config, &filter);

    if !interactive.should_run(&config) {
        // Non-interactive: print file paths
        for item in &items {
            println!("{}", item.path.display());
        }
    } else {
        // Interactive: TUI selection
    }
    Ok(())
}
```

#### Command Structure

| Command | File | Key Functions |
|---------|------|---------------|
| `init` | `init.rs` | Creates `.qstack` and qstack directory |
| `new` | `new.rs` | Creates item/template, `--as-template`, `--from-template`, wizard |
| `list` | `list.rs` | Lists items/templates (`--templates`), labels, categories, attachments, meta |
| `search` | `search.rs` | Query matching with full-text option |
| `update` | `update.rs` | Updates metadata, renames file |
| `close` | `close.rs` | Archives item (and `reopen`) |
| `attachments` | `attach.rs` | Add/remove attachments |
| `setup` | `setup.rs` | One-time config and completions |
| `completions` | `completions.rs` | Generate shell completion scripts |

### Constants (`src/constants.rs`)

Centralized magic values:

```rust
// File system
pub const ITEM_FILE_EXTENSION: &str = "md";
pub const DEFAULT_STACK_DIR: &str = "qstack";
pub const DEFAULT_ARCHIVE_DIR: &str = ".archive";
pub const MAX_SLUG_LENGTH: usize = 50;
pub const FRONTMATTER_DELIMITER: &str = "---";
pub const ATTACHMENT_INFIX: &str = "-Attachment-";

// UI
pub const UI_TITLE_TRUNCATE_LEN: usize = 40;
pub const UI_LABELS_TRUNCATE_LEN: usize = 20;
pub const UI_COL_ID_WIDTH: usize = 15;
pub const UI_COL_STATUS_WIDTH: usize = 6;

// Shell completions
pub const ZSH_COMPLETIONS_DIR: &str = ".zfunc";
// ...
```

### Editor Integration (`src/editor.rs`)

Launches user's editor with fallback chain:

1. `editor` config setting
2. `$VISUAL` environment variable
3. `$EDITOR` environment variable
4. `vi` (fallback)

Supports editor commands with arguments (e.g., `"code --wait"`).

## Data Flow Examples

### Creating an Item

```
User: qstack new "Fix bug" --label urgent

1. main.rs parses CLI args via clap
2. commands::new::execute() called
3. Config::load() merges global + project config
4. Config::user_name_or_prompt() resolves author
5. id::generate() creates unique ID
6. Item constructed with Frontmatter
7. storage::create_item() writes to disk
8. editor::open() launches editor (if interactive)
9. Print success message with path
```

### Interactive Selection

```
User: qstack list

1. Config::load()
2. storage::walk_items() finds all .md files
3. Item::load() parses each file
4. Filter and sort items
5. InteractiveArgs::should_run() checks flags + TTY
6. tui::run(SelectScreen) launches TUI
7. User navigates with arrow keys, selects with Enter
8. editor::open() opens selected item
```

### Archiving an Item

```
User: qstack close --id 260109

1. storage::find_by_id() locates item (partial match)
2. Item::load() parses file
3. item.set_status(Status::Closed)
4. storage::archive_item():
   a. derive_category() extracts category from path
   b. move_attachments() relocates attachment files
   c. git::move_file() moves item to .archive/{category}/
5. item.save() updates frontmatter
6. Print success message
```

### Reopening an Item

```
User: qstack reopen --id 260109

1. storage::find_by_id() locates item in archive
2. Item::load() parses file
3. item.set_status(Status::Open)
4. storage::unarchive_item():
   a. derive_category() extracts category from archive path
   b. move_attachments() relocates attachment files
   c. git::move_file() moves item back to {category}/
5. item.save() updates frontmatter
6. Print success message
```

### Creating from Template

```
User: qstack new "Login Bug" --from-template "Bug Report"

1. main.rs parses CLI args via clap
2. commands::new::execute_from_template() called
3. storage::find_template() locates template by:
   a. ID match (partial, case-insensitive)
   b. Title match (case-insensitive substring)
   c. Slug match (from filename)
4. Item::load() parses template
5. Inherit metadata:
   a. Labels merged (template + CLI, no duplicates)
   b. Category inherited if not specified on CLI
6. id::generate() creates unique ID for new item
7. storage::create_item() writes new item
8. copy_template_attachments():
   a. File attachments: copied to new item's directory
   b. URL attachments: added directly to frontmatter
9. editor::open() launches editor (if interactive)
10. Print success message with path
```

## File System Layout

```
project/
├── .qstack                 # Project config (TOML)
└── qstack/                 # qstack directory
    ├── .archive/           # Closed items (preserves category structure)
    │   ├── bugs/           # Archived items from bugs category
    │   │   └── 260108-...-old-bug.md
    │   └── 260108-...-old-task.md  # Archived uncategorized item
    ├── .templates/         # Templates (preserves category structure)
    │   ├── bugs/           # Templates in bugs category
    │   │   └── 260107-...-bug-report.md
    │   └── 260107-...-feature-request.md
    ├── bugs/               # Category subdirectory
    │   ├── 260109-...-fix-login.md
    │   └── 260109-...-Attachment-1-screenshot.png
    └── 260110-...-add-feature.md
```

**Category**: Derived from folder path, NOT stored in item metadata. Moving an item to a different folder changes its category. Archive and templates preserve category folder structure.

**Templates**: Items with `status: template` stored in `.templates/` directory. Used as patterns for creating new items.

## Testing Strategy

Tests are in `tests/` using a shared test harness:

```rust
// tests/common/mod.rs
pub struct TestEnv {
    temp_dir: TempDir,
    home_dir: PathBuf,
    project_dir: PathBuf,
}

impl TestEnv {
    pub fn run_qstack(&self, args: &[&str]) -> Output { ... }
    pub fn stack_path(&self) -> PathBuf { ... }
    // ...
}
```

Test categories:
- **Unit tests** — In-module `#[cfg(test)]` blocks
- **Integration tests** — Full CLI invocations in `tests/*.rs`
- **Edge cases** — Unicode, special characters, concurrent access

## Error Handling

Uses `anyhow` for application errors with context:

```rust
use anyhow::{Context, Result};

fn load_item(path: &Path) -> Result<Item> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read: {}", path.display()))?;
    // ...
}
```

Colored output via `owo-colors`:
- Success: green checkmark
- Warnings: yellow
- Errors: red with context chain

## Performance Considerations

1. **Lazy iteration** — `walk_items()` returns iterator, not collected Vec
2. **Partial ID matching** — Stops early on exact match
3. **Minimal parsing** — Only parse frontmatter when needed
4. **No indexing** — Items are scanned on demand (designed for small-to-medium projects)

## Extension Points

### Adding a New Command

1. Create `src/commands/foo.rs`
2. Add to `src/commands/mod.rs` exports
3. Add variant to `Commands` enum in `main.rs`
4. Implement `execute()` function
5. Add integration tests in `tests/foo.rs`

### Adding a Config Option

1. Add field to `GlobalConfig` and `ProjectConfig`
2. Add resolution method to `Config`
3. Update `save_with_comments()` in both config files
4. Update test harness builders
5. Document in README

### Adding a TUI Screen

1. Create struct implementing `TuiApp`
2. Implement `handle_event()` for input handling
3. Implement `render()` for display
4. Use `tui::run()` to execute
