# qstack

## Project Overview
`qstack` is a CLI-based task and issue tracker that follows a "documentation as code" philosophy. It stores items as Markdown files with YAML frontmatter, organized in a directory structure within a Git repository. It is designed to be human-readable, grep-friendly, and fully integrated with standard developer workflows. The text-first approach makes it especially versatile in agentic driven workflows.

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
│   ├── editor.rs           # Editor launch logic
│   ├── id/
│   │   ├── mod.rs          # ID generator with pattern parsing
│   │   └── base32.rs       # Crockford's Base32 encoder
│   ├── item/
│   │   ├── mod.rs          # Item struct & Status enum
│   │   ├── parser.rs       # YAML frontmatter parsing
│   │   └── slug.rs         # Title slugification
│   ├── config/
│   │   ├── mod.rs          # Merged config resolver
│   │   ├── global.rs       # ~/.qstack handling
│   │   └── project.rs      # .qstack handling
│   ├── storage/
│   │   ├── mod.rs          # File operations, ID lookup
│   │   └── git.rs          # git mv integration
│   └── commands/
│       ├── mod.rs          # Command dispatch
│       ├── init.rs         # qstack init
│       ├── new.rs          # qstack new <title>
│       ├── list.rs         # qstack list [filters]
│       ├── search.rs       # qstack search <query>
│       ├── update.rs       # qstack update --id <id>
│       ├── close.rs        # qstack close/reopen
│       ├── labels.rs       # qstack labels
│       ├── categories.rs   # qstack categories
│       ├── setup.rs        # qstack setup (one-time setup)
│       └── completions.rs  # qstack completions <shell>
├── scripts/
│   └── install-hooks.sh    # Git hooks installer
├── tests/
│   ├── harness.rs          # Test utilities
│   └── integration.rs      # Integration tests
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
- `dialoguer` - Interactive terminal prompts (search selection)

## CLI Commands
```bash
qstack init                                    # Initialize project
qstack new "Title" --label bug --category bugs # Create item
qstack new "Title" --no-interactive            # Create without opening editor
qstack new "Title" -i                          # Force open editor even if config says no
qstack list --open --sort date                 # List items
qstack search "query"                          # Search and select interactively
qstack search "bug" --full-text --no-interactive  # Full-text search, list results
qstack update --id 260109 --title "New Title"  # Update item
qstack close --id 260109                       # Archive item
qstack reopen --id 260109                      # Restore item
qstack labels                                  # List all labels (interactive selection)
qstack categories                              # List all categories (interactive selection)
qstack setup                                   # One-time setup (config + completions)
qstack completions zsh                         # Generate shell completions
```

## Storage Layout
```
project-root/
├── .qstack             # Project config (TOML)
└── qstack/             # Item storage
    ├── archive/        # Closed items
    ├── bugs/           # Category subdirectory
    │   └── 260109-02F7K9M-fix-login-styling.md
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
---

Description and notes go here in Markdown.
```

## ID Generation
Uses Crockford's Base32 (0-9, A-Z excluding I,L,O,U) with pattern `%y%m%d-%T%RRR`:
- `%y%m%d` - Date (YYMMDD)
- `%T` - 4-char Base32 time (seconds since midnight)
- `%RRR` - 3 random Base32 chars

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
4. Update test harness builders in `tests/harness.rs`

## Style & Conventions
Follow the Rust style guide: `~/Agents/Style/rust-style-guide.md`

References:
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/)
