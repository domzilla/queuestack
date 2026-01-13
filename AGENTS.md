# qstack

## Project Overview
`qstack` is a minimal, scriptable task and issue tracker optimized for agent-driven project management. Items are stored as plain Markdown files, making them human-readable, grep-friendly, and easy to integrate into any workflow.

## Tech Stack
- **Language**: Rust (stable, minimum 1.75)
- **Build**: cargo
- **Test**: cargo test
- **Linting**: clippy (pedantic)
- **Formatting**: rustfmt

## Code Quality (MANDATORY)

### Pre-commit Hook
A git pre-commit hook enforces the quality gate automatically. Install it after cloning:
```bash
./scripts/install-hooks.sh
```

The hook runs on every commit:
```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo build && cargo test
```

To bypass temporarily (not recommended): `git commit --no-verify`

## Project Structure
```
qstack/
├── src/
│   ├── main.rs             # CLI entry point (clap derive)
│   ├── lib.rs              # Library root, public API
│   ├── constants.rs        # Shared constants
│   ├── editor.rs           # Editor launch logic
│   ├── ui.rs               # UI utilities
│   ├── id/
│   │   ├── mod.rs          # ID generator with pattern parsing
│   │   └── base32.rs       # Base32 encoder
│   ├── item/
│   │   ├── mod.rs          # Item struct & Status enum
│   │   ├── parser.rs       # YAML frontmatter parsing
│   │   ├── search.rs       # Search/filter logic
│   │   └── slug.rs         # Title slugification
│   ├── config/
│   │   ├── mod.rs          # Merged config resolver
│   │   ├── global.rs       # ~/.qstack handling
│   │   └── project.rs      # .qstack handling
│   ├── storage/
│   │   ├── mod.rs          # File operations, ID lookup
│   │   └── git.rs          # git mv integration
│   ├── tui/
│   │   ├── mod.rs          # TUI module root
│   │   ├── terminal.rs     # Terminal setup/teardown
│   │   ├── event.rs        # Input event handling
│   │   ├── screens/
│   │   │   ├── mod.rs
│   │   │   ├── select.rs   # Item selection screen
│   │   │   ├── prompt.rs   # Text input prompt
│   │   │   └── wizard.rs   # New item wizard
│   │   └── widgets/
│   │       ├── mod.rs
│   │       ├── select_list.rs
│   │       ├── multi_select.rs
│   │       ├── text_input.rs
│   │       └── text_area.rs
│   └── commands/
│       ├── mod.rs          # Command dispatch & shared types
│       ├── init.rs         # qstack init
│       ├── new.rs          # qstack new <title>
│       ├── list.rs         # qstack list [filters] (also --labels, --categories, --attachments, --meta)
│       ├── search.rs       # qstack search <query>
│       ├── update.rs       # qstack update --id <id>
│       ├── close.rs        # qstack close/reopen
│       ├── attach.rs       # qstack attachments add/remove
│       ├── setup.rs        # qstack setup (one-time setup)
│       └── completions.rs  # qstack completions <shell>
├── scripts/
│   └── install-hooks.sh    # Git hooks installer
├── tests/
│   ├── common/mod.rs       # Test utilities & harness
│   ├── init.rs
│   ├── new.rs
│   ├── list.rs
│   ├── search.rs
│   ├── update.rs
│   ├── close.rs
│   ├── labels.rs
│   ├── categories.rs
│   ├── attach.rs
│   ├── config.rs
│   ├── edge_cases.rs
│   └── output_format.rs    # Non-interactive output format tests
├── Cargo.toml
├── Cargo.lock
├── rustfmt.toml
└── .gitignore
```

## Key Files
- Entry point: `src/main.rs`
- Library root: `src/lib.rs`
- CLI commands: `src/commands/*.rs`
- Item schema: `src/item/mod.rs`
- Config schema: `src/config/*.rs`
- TUI components: `src/tui/*.rs`

## Dependencies
- `clap` + `clap_complete` - CLI argument parsing (derive) + shell completions
- `serde` + `serde_yml` - YAML frontmatter serialization
- `toml` - Config file parsing
- `chrono` - Date/time handling
- `anyhow` + `thiserror` - Error handling
- `walkdir` - Directory traversal
- `dirs` - Home directory lookup
- `rand` - Random ID generation
- `owo-colors` - Colored terminal output
- `ratatui` + `crossterm` - Terminal UI (interactive selection, wizard)
- `unicode-width` - Display width calculation for CJK/emoji alignment

## CLI Commands
```bash
qstack init                                    # Initialize project
qstack new "Title" --label bug urgent --category bugs # Create item
qstack new                                     # Launch wizard
qstack new "Title" --no-interactive            # Create without editor
qstack new "Title" -i                          # Force editor open
qstack list --open --sort date                 # List items
qstack list --label bug --author "John"        # Filter items
qstack list --category bugs                    # Filter by category
qstack list --labels                           # List all unique labels
qstack list --categories                       # List all unique categories
qstack list --attachments --id 260109          # List attachments for item
qstack list --attachments --file qstack/260109-*.md  # Use file path instead of ID
qstack list --meta --id 260109                 # Show item metadata/frontmatter
qstack search "query"                          # Search and select
qstack search "bug" --full-text --no-interactive  # Full-text search
qstack update --id 260109 --title "New Title"  # Update item
qstack update --id 26 --label urgent           # Partial ID match
qstack update --id 26 --remove-label urgent    # Remove label
qstack update --id 26 --remove-category        # Move to qstack root
qstack update --file path/to/item.md --title X # Update by file path
qstack close --id 260109                       # Archive item
qstack close --file qstack/260109-*.md         # Close by file path
qstack reopen --id 260109                      # Restore item
qstack reopen --file qstack/archive/260109-*.md  # Reopen by file path
qstack attachments add --id 260109 file.png    # Add file attachment
qstack attachments add --file path/to/item.md file.png  # Add by file path
qstack attachments add --id 260109 https://... # Add URL attachment
qstack attachments remove --id 260109 1        # Remove by index
qstack setup                                   # One-time setup
qstack completions zsh                         # Generate completions
```

### --file Option (Scriptability)
Commands that accept `--id` also accept `--file` as an alternative. This enables:
- Shell tab completion for file paths
- Piping file paths from other commands (e.g., `qstack list | xargs -I{} qstack list --meta --file {}`)
- Working with items without knowing their ID

The `--id` and `--file` options are mutually exclusive.

## Non-Interactive Output Format
All non-interactive outputs (`--no-interactive` flag) must be **plain, scriptable, line-separated lists**:

- **No ANSI color codes** - Output must be plain text without escape sequences
- **No headers or labels** - Just the data, one item per line
- **No explanatory messages** - Except for empty results ("No items found.")
- **Newline-terminated** - Every output ends with a newline for proper piping

### Output Formats by Command

| Command | Output Format |
|---------|--------------|
| `list --no-interactive` | File paths, one per line |
| `list --labels --no-interactive` | `label (count)` per line |
| `list --categories --no-interactive` | `category (count)` per line |
| `list --attachments --id <ID>` | Attachment names/URLs, one per line |
| `list --meta --id <ID>` | `key: value` per line (YAML-like) |

Tests for output format compliance are in `tests/output_format.rs`.

## Shell Completions
Shell completion scripts are generated statically by `clap_complete`. When adding, removing, or modifying CLI commands or arguments, users must regenerate completions by running `qstack setup` again. This should be noted in release notes when CLI changes occur.

## Storage Layout
```
project-root/
├── .qstack             # Project config (TOML)
└── qstack/             # Item storage
    ├── .archive/       # Closed items (hidden, preserves category structure)
    │   ├── bugs/       # Archived items from bugs category
    │   │   └── 260109-02F7K9M-fix-login-styling.md
    │   └── 260109-02F8L1P-old-task.md  # Archived uncategorized item
    ├── bugs/           # Category subdirectory
    │   ├── 260109-02F7K9M-fix-login-styling.md
    │   └── 260109-02F7K9M-Attachment-1-screenshot.png
    └── 260109-02F8L1P-add-dark-mode.md  # Uncategorized item in root
```

**Category**: Derived from folder path, NOT stored in item metadata. Moving an item to a different folder changes its category.

## Item File Format
```yaml
---
id: 260109-02F7K9M
title: Fix Login Styling
author: Dominic Rodemer
created_at: 2026-01-09T12:34:56Z
status: open
labels:
  - bug
  - ui
attachments:
  - 260109-02F7K9M-Attachment-1-screenshot.png
  - https://github.com/org/repo/issues/42
---

Description and notes go here in Markdown.
```

Note: Category is NOT stored in frontmatter - it's derived from the item's folder location.

**Label/Category Normalization**: Non-alphanumeric characters (except `-` and `_`) in labels and categories are silently replaced with hyphens (`my label` → `my-label`, `level1/level2` → `level1-level2`).

## ID Generation
Default pattern `%y%m%d-%T%RRR` produces IDs like `260109-0A2B3C4`:
- `%y%m%d` - Date (YYMMDD)
- `%T` - 4-char encoded time (seconds since midnight)
- `%RRR` - 3 random chars

## Error Handling
- Use `anyhow::Result` for application errors
- Provide context with `.context("message")?`
- Colored error output via `owo-colors`

## Config System
Both global (`~/.qstack`) and project (`.qstack`) configs support the same 7 options.
Project values override global values when set.

| Option | Type | Default |
|--------|------|---------|
| `user_name` | `Option<String>` | None |
| `use_git_user` | `bool` | `true` |
| `editor` | `Option<String>` | None |
| `interactive` | `bool` | `true` |
| `id_pattern` | `String` | `"%y%m%d-%T%RRR"` |
| `stack_dir` | `String` | `"qstack"` |
| `archive_dir` | `String` | `".archive"` |

When adding a new config option:
1. Add the field to both `GlobalConfig` and `ProjectConfig`
2. Add resolution logic in `Config` (merged config) - project overrides global
3. Update both `save_with_comments()` methods to include documentation
4. Update test harness builders in `tests/common/mod.rs`

## Style & Conventions
Follow the Rust style guide: `~/Agents/Style/rust-style-guide.md`

References:
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/)
