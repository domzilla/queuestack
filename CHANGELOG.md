# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.4] - 2026-02-01

### Changed
- Linux release tarball now has stable filename (`qs-linux-x86_64.tar.gz`) for version-agnostic download URL

## [0.5.3] - 2026-02-01

### Fixed
- TUI delete now falls back to `git rm` or permanent delete on Linux (where `trash` command is unavailable)

## [0.5.2] - 2026-02-01

### Added
- Linux/WSL support with pre-built x86_64 binaries in GitHub releases

## [0.5.1] - 2026-02-01

### Fixed
- TUI delete now also trashes the item's `.attachments/` directory

## [0.5.0] - 2026-02-01

### Changed
- Refactor attachment storage from flat files to sibling `.attachments/` directories
  - Old: `queuestack/260109-XXX-Attachment-1-screenshot.png` (alongside item)
  - New: `queuestack/260109-XXX-item-title.attachments/1-screenshot.png` (sibling directory)
- Simplify attachment filename format to `{counter}-{name}.{ext}` (e.g., `1-screenshot.png`)
- Update Homebrew publish system with improved post-publish commands and placeholders

## [0.4.1] - 2026-01-22

### Changed
- Migrate global config to XDG path `~/.config/queuestack/config`

## [0.4.0] - 2026-01-18

### Changed
- Rename project from qstack to queuestack (CLI command: `qs`)
- Remove legacy qstack paths from gitignore

## [0.3.0] - 2026-01-17

### Added
- Ctrl+Alt+S shortcut to save wizard without opening editor

### Changed
- Redesign wizard with two-panel layout and Tab navigation
- Remove Content step from wizard, open external editor instead
- Integrate selection/status info into widget titles
- Mute content of unfocused SelectList and MultiSelect widgets
- Add two empty lines after frontmatter in new items

### Fixed
- Support shell-quoted arguments in editor config

## [0.2.0] - 2026-01-16

### Added
- Template system for reusable item patterns with slug lookup and attachment inheritance
- Config validation and auto-update to setup command

### Changed
- Comment out all fields in project config template by default
- Update documentation for template system

## [0.1.3] - 2026-01-14

### Added
- Option+Arrow word navigation in content editor
- Show item ID in wizard header when editing
- Paste (CMD+V) support to all TUI text inputs

### Fixed
- Improve content editor UX
- Disable vim-style modal editing in content editor
- Enable soft line wrap in TUI content editor

## [0.1.2] - 2026-01-14

### Added
- GitHub Actions workflow for Homebrew bottles
- Homebrew installation documentation and release workflow

### Fixed
- Correct bottle filename format and tags for macOS builds

## [0.1.0] - 2026-01-14

### Added
- Initial CLI implementation with `new`, `list`, `update`, `close`, `search` commands
- Interactive TUI wizard for creating new items with ratatui
- File and URL attachment support for items
- Labels and categories for organizing items
- Interactive selection mode for list command
- Shell autocompletion support (bash, zsh, fish)
- Global and project-level configuration system
- Git integration for tracking item changes
- Colorized help output with clap styles
- Filter overlay and action menu popup in TUI
- Full UTF-8 support with proper CJK/emoji alignment
- Category filter and remove-label functionality
- `--file` option as alternative to `--id` for item operations

### Changed
- Normalize labels and categories to lowercase with hyphens
- Derive category from path instead of storing in frontmatter
- Replace table output with simple lists and TUI columns
- Display full IDs instead of date-only portion
- Consolidate labels, categories, and attachments into list command

### Fixed
- Use display width for proper column alignment with CJK characters and emoji
- Preserve selection state when cloning SelectList
- Respect --closed flag in labels/categories modes
- Skip git commands for untracked/ignored files
- Exit silently when user cancels selection with ESC

[Unreleased]: https://github.com/domzilla/queuestack/compare/v0.5.4...HEAD
[0.5.4]: https://github.com/domzilla/queuestack/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/domzilla/queuestack/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/domzilla/queuestack/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/domzilla/queuestack/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/domzilla/queuestack/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/domzilla/queuestack/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/domzilla/queuestack/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/domzilla/queuestack/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/domzilla/queuestack/compare/v0.1.3...v0.2.0
[0.1.3]: https://github.com/domzilla/queuestack/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/domzilla/queuestack/compare/v0.1.0...v0.1.2
[0.1.0]: https://github.com/domzilla/queuestack/releases/tag/v0.1.0
