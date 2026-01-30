# queuestack

## Project Overview
`queuestack` is a minimal, scriptable task and issue tracker optimized for agent-driven project management. Items are stored as plain Markdown files, making them human-readable, grep-friendly, and easy to integrate into any workflow.

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
queuestack/
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
│   │   ├── search.rs       # Search/filter logic (single source of truth for CLI & TUI)
│   │   └── slug.rs         # Title slugification
│   ├── config/
│   │   ├── mod.rs          # Merged config resolver
│   │   ├── global.rs       # ~/.config/queuestack/config handling
│   │   └── project.rs      # .queuestack handling
│   ├── storage/
│   │   ├── mod.rs          # File operations, ID lookup
│   │   └── git.rs          # git mv integration
│   ├── tui/
│   │   ├── mod.rs          # TUI module root
│   │   ├── terminal.rs     # Terminal setup/teardown
│   │   ├── event.rs        # Input event handling
│   │   ├── screens/
│   │   │   ├── mod.rs
│   │   │   ├── select.rs       # Item selection screen
│   │   │   ├── item_actions.rs # Interactive list with filter overlay & action menu
│   │   │   ├── prompt.rs       # Text input prompt
│   │   │   ├── confirm.rs      # Yes/no confirmation dialog
│   │   │   └── wizard.rs       # Two-panel new item wizard (Meta + Attachments)
│   │   └── widgets/
│   │       ├── mod.rs
│   │       ├── select_list.rs
│   │       ├── multi_select.rs
│   │       ├── text_input.rs
│   │       ├── action_menu.rs
│   │       └── filter_overlay.rs
│   └── commands/
│       ├── mod.rs          # Command dispatch & shared types
│       ├── init.rs         # qs init
│       ├── new.rs          # qs new <title>
│       ├── list.rs         # qs list [filters] (also --labels, --categories, --attachments, --meta)
│       ├── search.rs       # qs search <query>
│       ├── update.rs       # qs update --id <id>
│       ├── close.rs        # qs close/reopen
│       ├── attach.rs       # qs attachments add/remove
│       ├── setup.rs        # qs setup (one-time setup)
│       └── completions.rs  # qs completions <shell>
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
│   ├── template.rs         # Template feature tests
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
- Filter logic: `src/item/search.rs`

## Key Internal Types
- `FilterCriteria` (`item/search.rs`) — Unified filter criteria for item filtering (search, labels, category, author). Used by both CLI commands and TUI.
- `ListOptions` (`commands/list.rs`) — CLI flags for `list` command (status, sort, labels/categories mode).
- `InteractiveArgs` (`ui.rs`) — Resolves `--interactive`/`--no-interactive` flags with `is_enabled(config)` method.

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
- `shlex` - Shell-style argument parsing for editor command

## CLI Commands
```bash
qs init                                    # Initialize project
qs new "Title" --label bug urgent --category bugs # Create item
qs new                                     # Launch wizard
qs new "Title" --no-interactive            # Create without editor
qs new "Title" -i                          # Force editor open
qs list --open --sort date                 # List items
qs list --label bug --author "John"        # Filter items
qs list --category bugs                    # Filter by category
qs list --labels                           # List all unique labels
qs list --categories                       # List all unique categories
qs list --attachments --id 260109          # List attachments for item
qs list --attachments --file queuestack/260109-*.md  # Use file path instead of ID
qs list --meta --id 260109                 # Show item metadata/frontmatter
qs search "query"                          # Search and select
qs search "bug" --full-text --no-interactive  # Full-text search
qs update --id 260109 --title "New Title"  # Update item
qs update --id 26 --label urgent           # Partial ID match
qs update --id 26 --remove-label urgent    # Remove label
qs update --id 26 --remove-category        # Move to queuestack root
qs update --file path/to/item.md --title X # Update by file path
qs close --id 260109                       # Archive item
qs close --file queuestack/260109-*.md     # Close by file path
qs reopen --id 260109                      # Restore item
qs reopen --file queuestack/archive/260109-*.md  # Reopen by file path
qs attachments add --id 260109 file.png    # Add file attachment
qs attachments add --file path/to/item.md file.png  # Add by file path
qs attachments add --id 260109 https://... # Add URL attachment
qs attachments remove --id 260109 1        # Remove by index
qs setup                                   # One-time setup
qs completions zsh                         # Generate completions

# Templates
qs new "Bug Report" --as-template          # Create a template
qs new "Bug Report" --as-template --category bugs  # Template in category
qs list --templates                        # List all templates (interactive)
qs list --templates --no-interactive       # List template file paths
qs new "My Bug" --from-template "Bug Report"  # Create from template by title
qs new "My Bug" --from-template bug-report # Create from template by slug
qs new "My Bug" --from-template 260109     # Create from template by ID
qs new --from-template                     # Template selection TUI
```

### Template System

Templates are reusable item patterns with `status: template`. They live in the `.templates/` directory (configurable via `template_dir`).

**Creating templates:**
- `--as-template` flag creates a template instead of a regular item
- Templates can have labels, categories, attachments, and body content

**Using templates:**
- `--from-template [REF]` creates an item from a template
- Reference can be: ID (partial match), title (case-insensitive), or slug (from filename)
- Omitting the reference shows an interactive template selector

**Inheritance from templates:**
- Labels: Merged (template labels + CLI labels, no duplicates)
- Category: Inherited if not specified on CLI
- Attachments: File attachments are copied, URLs are added directly
- Body content: Copied from template

**Template lookup order:**
1. ID match (partial, case-insensitive)
2. Title match (case-insensitive, contains)
3. Slug match (from filename, case-insensitive)

### --file Option (Scriptability)
Commands that accept `--id` also accept `--file` as an alternative. This enables:
- Shell tab completion for file paths
- Piping file paths from other commands (e.g., `qs list | xargs -I{} qs list --meta --file {}`)
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
Shell completion scripts are generated statically by `clap_complete`. When adding, removing, or modifying CLI commands or arguments, users must regenerate completions by running `qs setup` again. This should be noted in release notes when CLI changes occur.

## Storage Layout
```
project-root/
├── .queuestack             # Project config (TOML)
└── queuestack/             # Item storage
    ├── .archive/       # Closed items (hidden, preserves category structure)
    │   ├── bugs/       # Archived items from bugs category
    │   │   └── 260109-02F7K9M-fix-login-styling.md
    │   └── 260109-02F8L1P-old-task.md  # Archived uncategorized item
    ├── .templates/     # Templates (hidden, preserves category structure)
    │   ├── bugs/       # Templates in bugs category
    │   │   └── 260109-02F7K9M-bug-report.md
    │   └── 260109-02F8L1P-feature-request.md  # Uncategorized template
    ├── bugs/           # Category subdirectory
    │   ├── 260109-02F7K9M-fix-login-styling.md
    │   └── 260109-02F7K9M-Attachment-1-screenshot.png
    └── 260109-02F8L1P-add-dark-mode.md  # Uncategorized item in root
```

**Category**: Derived from folder path, NOT stored in item metadata. Moving an item to a different folder changes its category.

**Templates**: Stored in `.templates/` directory with `status: template`. Category structure mirrors the main item storage.

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

**Status values:** `open`, `closed`, `template`

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
Both global (`~/.config/queuestack/config`) and project (`.queuestack`) configs support the same 8 options.
Project values override global values when set.

| Option | Type | Default |
|--------|------|---------|
| `user_name` | `Option<String>` | None |
| `use_git_user` | `bool` | `true` |
| `editor` | `Option<String>` | None (supports shell quoting, e.g., `nvim -c ":normal G"`) |
| `interactive` | `bool` | `true` |
| `id_pattern` | `String` | `"%y%m%d-%T%RRR"` |
| `stack_dir` | `String` | `"queuestack"` |
| `archive_dir` | `String` | `".archive"` |
| `template_dir` | `String` | `".templates"` |

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

## Changelog (MANDATORY)
**All important code changes** (fixes, additions, deletions, changes) have to written to CHANGELOG.md.
Changelog format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Before writing to CHANGELOG.md:**
1. Check for new release tags: `git tag --sort=-creatordate | head -1`
2. Release tags are prefixed with `v` (e.g., `v2.0.1`)
3. If a new tag exists that isn't in CHANGELOG.md, create a new version section with that tag's version and date, moving relevant [Unreleased] content under it

## Homebrew Distribution

queuestack is distributed via Homebrew tap: `domzilla/tap`

### Release Workflow

Releases are automated via GitHub Actions (`.github/workflows/release.yml`):

1. **Bump version** in `Cargo.toml`
2. **Commit**: `git commit -am "Bump version to X.Y.Z"`
3. **Release**: `homebrew-publish` (tags and pushes)
4. **CI automatically**:
   - Builds bottles for macOS arm64 and x86_64
   - Creates GitHub release with bottles attached
   - Updates formula in `domzilla/homebrew-tap` with SHA256 hashes

### Related Repositories

| Repository | Purpose |
|------------|---------|
| `domzilla/homebrew-tap` | Homebrew tap with queuestack formula |
| Local: `/Users/dom/GIT/Homebrew/homebrew-tap` | Local checkout of tap |

### Configuration Files

| File | Purpose |
|------|---------|
| `.homebrew-publish` | Config for `homebrew-publish` script |
| `.github/workflows/release.yml` | GitHub Actions bottle build workflow |

### GitHub Secrets Required

| Secret | Purpose |
|--------|---------|
| `HOMEBREW_TAP_TOKEN` | PAT with write access to `homebrew-tap` repo |

### Manual Formula Update

If CI fails, manually update the formula:
1. Get source SHA256: `curl -sL <tarball_url> | shasum -a 256`
2. Edit `/Users/dom/GIT/Homebrew/homebrew-tap/Formula/queuestack.rb`
3. Commit and push the tap repo
