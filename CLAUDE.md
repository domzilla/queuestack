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

### Before Any Commit
Always run the full quality gate:
```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo build && cargo test
```

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
│       ├── update.rs       # qstack update --id <id>
│       └── close.rs        # qstack close/reopen
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
- `clap` - CLI argument parsing (derive)
- `serde` + `serde_yml` - YAML frontmatter serialization
- `toml` - Config file parsing
- `chrono` - Date/time handling
- `anyhow` + `thiserror` - Error handling
- `walkdir` - Directory traversal
- `dirs` - Home directory lookup
- `rand` - Random ID generation
- `comfy-table` - Table output
- `owo-colors` - Colored terminal output

## CLI Commands
```bash
qstack init                                    # Initialize project
qstack new "Title" --label bug --category bugs # Create item
qstack list --open --sort date                 # List items
qstack list --id 260109                        # Show item details
qstack update --id 260109 --title "New Title"  # Update item
qstack close --id 260109                       # Archive item
qstack reopen --id 260109                      # Restore item
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

## Style & Conventions
Follow the Rust style guide: `~/Agents/Style/rust-style-guide.md`

References:
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/)
