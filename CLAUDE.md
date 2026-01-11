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
│       ├── list.rs         # qstack list [filters]
│       ├── search.rs       # qstack search <query>
│       ├── update.rs       # qstack update --id <id>
│       ├── close.rs        # qstack close/reopen
│       ├── labels.rs       # qstack labels
│       ├── categories.rs   # qstack categories
│       ├── attachments.rs  # qstack attachments (list/add/remove)
│       ├── attach.rs       # Attachment helpers
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
│   └── edge_cases.rs
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
- `comfy-table` - Table output
- `owo-colors` - Colored terminal output
- `ratatui` + `crossterm` - Terminal UI (interactive selection, wizard)

## CLI Commands
```bash
qstack init                                    # Initialize project
qstack new "Title" --label bug --category bugs # Create item
qstack new                                     # Launch wizard
qstack new "Title" --no-interactive            # Create without editor
qstack new "Title" -i                          # Force editor open
qstack list --open --sort date                 # List items
qstack list --label bug --author "John"        # Filter items
qstack search "query"                          # Search and select
qstack search "bug" --full-text --no-interactive  # Full-text search
qstack update --id 260109 --title "New Title"  # Update item
qstack update --id 26 --label urgent           # Partial ID match
qstack close --id 260109                       # Archive item
qstack reopen --id 260109                      # Restore item
qstack labels                                  # List all labels
qstack categories                              # List all categories
qstack attachments list --id 260109            # List attachments
qstack attachments add --id 260109 file.png    # Add file attachment
qstack attachments add --id 260109 https://... # Add URL attachment
qstack attachments remove --id 260109 1        # Remove by index
qstack setup                                   # One-time setup
qstack completions zsh                         # Generate completions
```

## Shell Completions
Shell completion scripts are generated statically by `clap_complete`. When adding, removing, or modifying CLI commands or arguments, users must regenerate completions by running `qstack setup` again. This should be noted in release notes when CLI changes occur.

## Storage Layout
```
project-root/
├── .qstack             # Project config (TOML)
└── qstack/             # Item storage
    ├── archive/        # Closed items
    ├── bugs/           # Category subdirectory
    │   ├── 260109-02F7K9M-fix-login-styling.md
    │   └── 260109-02F7K9M-Attachment-1-screenshot.png
    └── 260109-02F8L1P-add-dark-mode.md
```

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
category: bugs
attachments:
  - 260109-02F7K9M-Attachment-1-screenshot.png
  - https://github.com/org/repo/issues/42
---

Description and notes go here in Markdown.
```

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
| `archive_dir` | `String` | `"archive"` |

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
