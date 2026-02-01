# queuestack

A minimal, scriptable task and issue tracker optimized for agent-driven project management.

Items are stored as plain Markdown files—human-readable, grep-friendly, and easy to integrate into any workflow.

## Features

- **Plain text storage** — Items are Markdown files you can read, edit, and search with standard tools
- **Scriptable** — Every command works non-interactively for automation and CI/CD pipelines
- **Interactive TUI** — Arrow-key navigation, filter overlay, action menu, and a wizard for creating items
- **Templates** — Create reusable item patterns and instantiate new items from them
- **Attachments** — Attach files or URLs to any item
- **Categories & Labels** — Organize items in subdirectories and tag them
- **Git-aware** — Uses `git mv` when renaming to preserve history
- **Shell completions** — Tab completion for Bash, Zsh, Fish, and PowerShell

## Installation

### Homebrew (macOS)

```bash
brew tap domzilla/tap
brew install queuestack
```

### From Source

```bash
cargo install --path .
```

### Post-Install Setup

Run the one-time setup to create your config and install shell completions:

```bash
qs setup
```

## Quick Start

```bash
# Initialize a project
qs init

# Create items
qs new "Fix login bug" --label bug
qs new "Add dark mode" --label feature --category enhancements
qs new                                   # Launch wizard

# List and filter
qs list                                  # Interactive selection
qs list --label bug --sort date          # Filter and sort
qs list --category bugs                  # Filter by category
qs list --closed                         # Show archived items

# Search
qs search "login"                        # Search titles and IDs
qs search "memory" --full-text           # Search body content too

# Update
qs update --id 260109 --title "New title"
qs update --id 26 --label urgent         # Partial ID match
qs update --id 26 --remove-label bug     # Remove a label
qs update --id 26 --remove-category      # Move to queuestack root

# Attachments
qs attachments add --id 260109 screenshot.png
qs attachments add --id 260109 https://github.com/org/repo/issues/42
qs attachments list --id 260109

# Archive and restore
qs close --id 260109
qs reopen --id 260109

# Templates
qs new "Bug Report" --as-template          # Create a template
qs list --templates                        # List all templates
qs new "Login Bug" --from-template "Bug Report"  # Create from template
```

## Commands

| Command | Description |
|---------|-------------|
| `init` | Initialize a new queuestack project |
| `new [title]` | Create a new item (omit title for wizard) |
| `new --as-template` | Create a reusable template |
| `new --from-template <ref>` | Create item from template (by ID, title, or slug) |
| `list` | List items with filters and sorting |
| `list --templates` | List all templates |
| `list --labels` | List all labels in use |
| `list --categories` | List all categories in use |
| `search <query>` | Search by title, ID, or content |
| `update --id <id>` | Update title, labels, or category |
| `close --id <id>` | Archive an item |
| `reopen --id <id>` | Restore from archive |
| `attachments` | List, add, or remove attachments |
| `setup` | Configure queuestack and install completions |
| `completions <shell>` | Generate shell completion script |

Run `qs <command> --help` for detailed options.

## TUI Keybindings

### Item List (`qs list`)

| Key | Action |
|-----|--------|
| `↑`/`↓` or `j`/`k` | Navigate items |
| `Enter` | Open action menu for selected item |
| `f` | Open filter overlay |
| `c` | Clear active filter |
| `Esc` | Cancel / close overlay |

**Filter overlay** (`f`): Filter items by search text, labels, or category in real-time.

**Action menu** (`Enter`): Quick actions on the selected item — view, edit, close/reopen, delete.

### New Item Wizard (`qs new`)

| Key | Action |
|-----|--------|
| `Tab` | Next field |
| `Shift+Tab` | Previous field |
| `Ctrl+S` | Save and open editor |
| `Ctrl+Alt+S` | Save without opening editor |
| `Enter` | Confirm selection / add item |
| `Space` | Toggle label selection |
| `Esc` | Cancel |

The wizard has two panels: **Meta** (title, category, labels) and **Attachments**.

## Non-Interactive Mode

Every command supports `--no-interactive` for scripting:

```bash
# Create without opening editor
qs new "Automated task" --label bot --no-interactive

# List without selector
qs list --no-interactive

# Search and get results as text
qs search "bug" --no-interactive
```

## Storage Format

Items are Markdown files with YAML frontmatter:

```
queuestack/
├── 260109-0A2B3C4-fix-login-bug.md
├── 260109-0A2B3C4-fix-login-bug.attachments/
│   ├── 1-screenshot.png
│   └── 2-notes.md
├── bugs/
│   └── 260110-0B3C4D5-memory-leak.md
├── .archive/
│   └── 260108-0Z1Y2X3-old-task.md
└── .templates/
    └── 260107-0A1B2C3-bug-report.md
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
  - 1-screenshot.png
  - https://github.com/org/repo/issues/42
---

Description and notes in Markdown.

## Reproduction Steps
1. Go to login page
2. Enter invalid credentials
3. See console error
```

**Note:** Category is derived from the folder path, not stored in frontmatter. An item in `queuestack/bugs/` has category `bugs`. Status can be `open`, `closed`, or `template`. Attachments are stored in a sibling `.attachments/` directory.

## Configuration

Two config files (TOML format):

| File | Scope |
|------|-------|
| `~/.config/queuestack/config` | Global defaults (user name, editor, ID pattern) |
| `.queuestack` | Project overrides (queuestack directory, archive directory) |

Project settings override global settings.

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `user_name` | — | Author name for new items |
| `use_git_user` | `true` | Fall back to `git config user.name` |
| `editor` | `$EDITOR` | Editor command (supports args, e.g., `nvim -c ":normal G"`) |
| `interactive` | `true` | Enable TUI by default |
| `id_pattern` | `%y%m%d-%T%RRR` | ID format pattern |
| `stack_dir` | `queuestack` | Directory for items |
| `archive_dir` | `.archive` | Subdirectory for closed items |
| `template_dir` | `.templates` | Subdirectory for templates |

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

Completions are installed automatically by `qs setup`. Supported shells:

- Bash
- Zsh
- Fish
- PowerShell

After updating queuestack, run `qs setup` again to refresh completions.

## Integration Examples

### With grep

```bash
# Find all items mentioning "API"
grep -r "API" queuestack/

# Find urgent bugs
grep -l "urgent" queuestack/*.md | xargs grep -l "bug"
```

### With git

```bash
# See item history
git log --oneline -- queuestack/

# Who worked on what
git log --author="Alice" -- queuestack/
```

### With scripts

```bash
# Create items from a list
while read -r title; do
  qs new "$title" --label imported --no-interactive
done < tasks.txt

# Export open items
qs list --no-interactive | tail -n +3 > report.txt
```

## License

MIT
