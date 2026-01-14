# qstack

A minimal, scriptable task and issue tracker optimized for agent-driven project management.

Items are stored as plain Markdown files—human-readable, grep-friendly, and easy to integrate into any workflow.

## Features

- **Plain text storage** — Items are Markdown files you can read, edit, and search with standard tools
- **Scriptable** — Every command works non-interactively for automation and CI/CD pipelines
- **Interactive TUI** — Arrow-key navigation, filter overlay, action menu, and a wizard for creating items
- **Attachments** — Attach files or URLs to any item
- **Categories & Labels** — Organize items in subdirectories and tag them
- **Git-aware** — Uses `git mv` when renaming to preserve history
- **Shell completions** — Tab completion for Bash, Zsh, Fish, and PowerShell

## Installation

### Homebrew (macOS)

```bash
brew tap domzilla/tap
brew install qstack
```

### From Source

```bash
cargo install --path .
```

### Post-Install Setup

Run the one-time setup to create your config and install shell completions:

```bash
qstack setup
```

## Quick Start

```bash
# Initialize a project
qstack init

# Create items
qstack new "Fix login bug" --label bug
qstack new "Add dark mode" --label feature --category enhancements
qstack new                                   # Launch wizard

# List and filter
qstack list                                  # Interactive selection
qstack list --label bug --sort date          # Filter and sort
qstack list --category bugs                  # Filter by category
qstack list --closed                         # Show archived items

# Search
qstack search "login"                        # Search titles and IDs
qstack search "memory" --full-text           # Search body content too

# Update
qstack update --id 260109 --title "New title"
qstack update --id 26 --label urgent         # Partial ID match
qstack update --id 26 --remove-label bug     # Remove a label
qstack update --id 26 --remove-category      # Move to qstack root

# Attachments
qstack attachments add --id 260109 screenshot.png
qstack attachments add --id 260109 https://github.com/org/repo/issues/42
qstack attachments list --id 260109

# Archive and restore
qstack close --id 260109
qstack reopen --id 260109
```

## Commands

| Command | Description |
|---------|-------------|
| `init` | Initialize a new qstack project |
| `new [title]` | Create a new item (omit title for wizard) |
| `list` | List items with filters and sorting |
| `list --labels` | List all labels in use |
| `list --categories` | List all categories in use |
| `search <query>` | Search by title, ID, or content |
| `update --id <id>` | Update title, labels, or category |
| `close --id <id>` | Archive an item |
| `reopen --id <id>` | Restore from archive |
| `attachments` | List, add, or remove attachments |
| `setup` | Configure qstack and install completions |
| `completions <shell>` | Generate shell completion script |

Run `qstack <command> --help` for detailed options.

## TUI Keybindings

When running `qstack list` interactively:

| Key | Action |
|-----|--------|
| `↑`/`↓` or `j`/`k` | Navigate items |
| `Enter` | Open action menu for selected item |
| `f` | Open filter overlay |
| `c` | Clear active filter |
| `Esc` | Cancel / close overlay |

**Filter overlay** (`f`): Filter items by search text, labels, or category in real-time.

**Action menu** (`Enter`): Quick actions on the selected item — view, edit, close/reopen, delete.

## Non-Interactive Mode

Every command supports `--no-interactive` for scripting:

```bash
# Create without opening editor
qstack new "Automated task" --label bot --no-interactive

# List without selector
qstack list --no-interactive

# Search and get results as text
qstack search "bug" --no-interactive
```

## Storage Format

Items are Markdown files with YAML frontmatter:

```
qstack/
├── 260109-0A2B3C4-fix-login-bug.md
├── bugs/
│   └── 260110-0B3C4D5-memory-leak.md
└── .archive/
    └── 260108-0Z1Y2X3-old-task.md
```

Each item:

```yaml
---
id: 260109-0A2B3C4
title: Fix Login Bug
author: Your Name
created_at: 2026-01-09T12:34:56Z
status: open
labels:
  - bug
  - urgent
attachments:
  - 260109-0A2B3C4-Attachment-1-screenshot.png
  - https://github.com/org/repo/issues/42
---

Description and notes in Markdown.

## Reproduction Steps
1. Go to login page
2. Enter invalid credentials
3. See console error
```

**Note:** Category is derived from the folder path, not stored in frontmatter. An item in `qstack/bugs/` has category `bugs`.

## Configuration

Two config files (TOML format):

| File | Scope |
|------|-------|
| `~/.qstack` | Global defaults (user name, editor, ID pattern) |
| `.qstack` | Project overrides (qstack directory, archive directory) |

Project settings override global settings.

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `user_name` | — | Author name for new items |
| `use_git_user` | `true` | Fall back to `git config user.name` |
| `editor` | `$EDITOR` | Editor for item creation |
| `interactive` | `true` | Enable TUI by default |
| `id_pattern` | `%y%m%d-%T%RRR` | ID format pattern |
| `stack_dir` | `qstack` | Directory for items |
| `archive_dir` | `.archive` | Subdirectory for closed items |

### ID Pattern Tokens

| Token | Description | Example |
|-------|-------------|---------|
| `%y` | Year (2 digits) | `26` |
| `%m` | Month | `01` |
| `%d` | Day of month | `09` |
| `%j` | Day of year | `009` |
| `%T` | Time (4 chars) | `0A2B` |
| `%R` | Random char | `X` |
| `%%` | Literal `%` | `%` |

## Shell Completions

Completions are installed automatically by `qstack setup`. Supported shells:

- Bash
- Zsh
- Fish
- PowerShell

After updating qstack, run `qstack setup` again to refresh completions.

## Integration Examples

### With grep

```bash
# Find all items mentioning "API"
grep -r "API" qstack/

# Find urgent bugs
grep -l "urgent" qstack/*.md | xargs grep -l "bug"
```

### With git

```bash
# See item history
git log --oneline -- qstack/

# Who worked on what
git log --author="Alice" -- qstack/
```

### With scripts

```bash
# Create items from a list
while read -r title; do
  qstack new "$title" --label imported --no-interactive
done < tasks.txt

# Export open items
qstack list --no-interactive | tail -n +3 > report.txt
```

## License

MIT
