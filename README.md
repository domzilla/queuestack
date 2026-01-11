# qstack

A minimal, scriptable task and issue tracker optimized for agent-driven project management. Items are stored as plain Markdown files, making them human-readable, grep-friendly, and easy to integrate into any workflow.

## Installation

```bash
cargo install --path .
```

**Required:** Run the one-time setup before using qstack:

```bash
qstack setup
```

This creates the global configuration (`~/.qstack`) and installs shell completions for tab completion of commands and arguments.

> **Note:** Run `qstack setup` again after updating qstack to refresh shell completions with any new commands or options.

## Quick Start

```bash
# Initialize a project
qstack init

# Create items
qstack new "Fix login bug" --label bug
qstack new "Add dark mode" --label feature --category enhancements

# List and search
qstack list
qstack search "login"

# Update and close
qstack update --id 260109 --label urgent
qstack close --id 260109
```

## Commands

| Command | Description |
|---------|-------------|
| `init` | Initialize a new qstack project |
| `new <title>` | Create a new item |
| `list` | List items with optional filters |
| `search <query>` | Search items by title or ID |
| `update --id <id>` | Update an item's metadata |
| `close --id <id>` | Archive an item |
| `reopen --id <id>` | Restore an archived item |
| `labels` | List all labels |
| `categories` | List all categories |
| `setup` | One-time setup (config + completions) |
| `completions <shell>` | Generate shell completion script |

Run `qstack <command> --help` for detailed usage.

## Storage Format

Items are stored as Markdown files with YAML frontmatter:

```yaml
---
id: 260109-02F7K9M
title: Fix Login Bug
author: Your Name
created_at: 2026-01-09T12:34:56Z
status: open
labels:
  - bug
category: bugs
---

Description and notes go here.
```

## Configuration

- `~/.qstack` — Global config (user name, editor, ID pattern)
- `.qstack` — Project config (stack directory, archive directory)

## License

MIT
