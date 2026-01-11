# qstack

A CLI-based task and issue tracker that follows a "documentation as code" philosophy. Items are stored as Markdown files with YAML frontmatter, organized in a directory structure within a Git repository.

## Installation

```bash
cargo install --path .
```

After installation, run the one-time setup to enable shell completions:

```bash
qstack setup
```

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
