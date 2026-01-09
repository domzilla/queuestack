//! # qstack CLI
//!
//! Command-line interface for the qstack task/issue tracker.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use anyhow::Result;
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;

use qstack::commands::{self, GetArgs, ListFilter, NewArgs, SortBy, UpdateArgs};

const GLOBAL_HELP: &str = "\
Configuration Files:
  ~/.qstack          Global configuration (user name, editor, ID pattern)
  .qstack            Project configuration (stack directory, archive directory)

ID Pattern Tokens (for id_pattern config):
  %y  Year (2 digits)           %m  Month (01-12)
  %d  Day of month (01-31)      %j  Day of year (001-366)
  %T  Time as Base32 (4 chars)  %R  Random Base32 char (repeat: %RRR)
  %%  Literal percent sign

Getting Started:
  qstack init                    Initialize project in current directory
  qstack new \"My first task\"     Create a new item
  qstack list                    List all open items
  qstack close --id <ID>         Close an item

Learn more:
  qstack <COMMAND> --help        Show detailed help for a command";

#[derive(Parser)]
#[command(name = "qstack")]
#[command(author = "Dominic Rodemer")]
#[command(version)]
#[command(about = "CLI-based task and issue tracker with documentation-as-code philosophy")]
#[command(
    long_about = "qstack is a CLI-based task and issue tracker that follows a documentation-as-code \
philosophy. Items are stored as Markdown files with YAML frontmatter in a Git-friendly \
directory structure, making them easy to version, search, and collaborate on.

Each item gets a unique ID, a slugified filename, and can be organized into categories. \
The tool integrates with Git for seamless version control and supports customizable \
ID patterns using Crockford's Base32 encoding."
)]
#[command(after_help = GLOBAL_HELP)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new qstack project in the current directory
    #[command(
        long_about = "Initialize a new qstack project in the current directory.\n\n\
Creates a .qstack configuration file and the stack directory structure. \
The configuration file contains all available options with detailed comments.\n\n\
Directory structure created:\n  \
.qstack              Project configuration file\n  \
qstack/              Stack directory for items\n  \
qstack/archive/      Archive directory for closed items",
        after_help = "Examples:\n  \
qstack init                     Initialize in current directory\n  \
cd myproject && qstack init     Initialize in a specific project\n\n\
Note: Run this command once per project, typically at the repository root."
    )]
    Init,

    /// Create a new item
    #[command(
        long_about = "Create a new item with the given title.\n\n\
Generates a unique ID, creates a Markdown file with YAML frontmatter, and optionally \
opens your editor. The filename is derived from the ID and a slugified version of \
the title (e.g., '260109-0A2B-fix-login-bug.md').\n\n\
The author is determined from (in order):\n  \
1. user_name in ~/.qstack\n  \
2. git config user.name (if use_git_user is true)\n  \
3. Interactive prompt (saved to ~/.qstack for future use)",
        after_help = "Examples:\n  \
qstack new \"Fix login bug\"\n  \
qstack new \"Add dark mode\" --label feature --label ui\n  \
qstack new \"Memory leak in parser\" --label bug --category bugs\n  \
qstack new \"Update docs\" -l docs -c documentation\n\n\
Output: Prints the relative path to the created file."
    )]
    New {
        /// Title of the item
        title: String,

        /// Labels/tags for the item (can be specified multiple times)
        #[arg(short, long)]
        label: Vec<String>,

        /// Category subdirectory for the item
        #[arg(short, long)]
        category: Option<String>,
    },

    /// List items
    #[command(
        long_about = "List items in the current project.\n\n\
By default, shows all open items in a table format with ID, title, author, date, \
and labels. Use filters to narrow down results or --id to show full details of \
a specific item.\n\n\
The --id option supports partial matching - you only need to provide enough \
characters to uniquely identify an item (e.g., '260109' or even '2601').",
        after_help = "Examples:\n  \
qstack list                        List all open items\n  \
qstack list --closed               List archived/closed items\n  \
qstack list --open --closed        List all items (open and closed)\n  \
qstack list --label bug            Filter by label\n  \
qstack list --author \"John\"        Filter by author\n  \
qstack list --sort date            Sort by creation date\n  \
qstack list --sort title           Sort alphabetically by title\n  \
qstack list --id 260109            Show details of item matching ID\n  \
qstack list --id 26                Show item if ID prefix is unique"
    )]
    List {
        /// Show only open items
        #[arg(long, help = "Show only open items (default if no status filter)")]
        open: bool,

        /// Show only closed items
        #[arg(long, help = "Show only closed/archived items")]
        closed: bool,

        /// Show details of a specific item by ID
        #[arg(long, help = "Show full details of item (partial ID match supported)")]
        id: Option<String>,

        /// Filter by label
        #[arg(long, help = "Filter items containing this label")]
        label: Option<String>,

        /// Filter by author
        #[arg(long, help = "Filter items by author name (substring match)")]
        author: Option<String>,

        /// Sort order
        #[arg(
            long,
            value_enum,
            default_value = "id",
            help = "Sort order: id, date, or title"
        )]
        sort: SortBy,
    },

    /// Get the first matching item and open it
    #[command(
        long_about = "Get the first matching item and optionally open it in your editor.\n\n\
Retrieves a single item based on filters and sort order, outputs its path, and opens \
it in your configured editor (unless suppressed with --no-open).\n\n\
This is useful for quickly jumping to the most relevant item - for example, getting \
the oldest open bug or the most recently created task.",
        after_help = "Examples:\n  \
qstack get                             Get first item (by ID), open in editor\n  \
qstack get --no-open                   Get first item, print path only\n  \
qstack get --sort date                 Get most recently created item\n  \
qstack get --label bug                 Get first item with 'bug' label\n  \
qstack get --label bug --sort date     Get most recent bug\n  \
qstack get --closed                    Get first closed/archived item\n  \
qstack get --author \"John\"             Get first item by author\n\n\
Output: Prints the relative path to the item file."
    )]
    Get {
        /// Filter by label
        #[arg(long, help = "Filter items containing this label")]
        label: Option<String>,

        /// Filter by author
        #[arg(long, help = "Filter items by author name")]
        author: Option<String>,

        /// Sort order (determines which item is "first")
        #[arg(
            long,
            value_enum,
            default_value = "id",
            help = "Sort order: id, date, or title"
        )]
        sort: SortBy,

        /// Don't open the item in editor
        #[arg(long, help = "Don't open item in editor (just print path)")]
        no_open: bool,

        /// Get from closed/archived items
        #[arg(long, help = "Get from closed/archived items instead of open")]
        closed: bool,
    },

    /// Update an existing item
    #[command(
        long_about = "Update an existing item's metadata.\n\n\
Modify the title, labels, or category of an item. If the title changes, the file \
is renamed to reflect the new slug. In Git repositories, uses 'git mv' to preserve \
history.\n\n\
Labels are additive - new labels are added without removing existing ones. \
To modify labels directly, edit the Markdown file.",
        after_help = "Examples:\n  \
qstack update --id 260109 --title \"New title\"\n  \
qstack update --id 2601 --label urgent --label p1\n  \
qstack update --id 260109 --category bugs\n  \
qstack update --id 260109 --no-category          Move to stack root\n  \
qstack update --id 26 --title \"Fix\" --label done  Multiple updates\n\n\
Note: The --id flag supports partial matching for convenience."
    )]
    Update {
        /// Item ID (partial match supported)
        #[arg(
            long,
            required = true,
            help = "Item ID to update (partial match supported)"
        )]
        id: String,

        /// New title
        #[arg(long, help = "New title (renames file if changed)")]
        title: Option<String>,

        /// Add labels (can be specified multiple times)
        #[arg(long, help = "Add label(s) to item (can repeat)")]
        label: Vec<String>,

        /// Move to category
        #[arg(long, help = "Move item to category subdirectory")]
        category: Option<String>,

        /// Remove from category (move to root)
        #[arg(long, help = "Remove from category (move to stack root)")]
        no_category: bool,
    },

    /// Close an item (move to archive)
    #[command(
        long_about = "Close an item by moving it to the archive directory.\n\n\
Sets the item's status to 'closed' and moves it from the stack directory to the \
archive subdirectory. In Git repositories, uses 'git mv' to preserve history.\n\n\
Closed items are excluded from 'qstack list' by default (use --closed to see them).",
        after_help = "Examples:\n  \
qstack close --id 260109              Close by full ID\n  \
qstack close --id 2601                Close by partial ID\n  \
qstack list --closed                  View closed items\n  \
qstack reopen --id 260109             Reopen if needed"
    )]
    Close {
        /// Item ID (partial match supported)
        #[arg(
            long,
            required = true,
            help = "Item ID to close (partial match supported)"
        )]
        id: String,
    },

    /// Reopen a closed item (move from archive)
    #[command(
        long_about = "Reopen a closed item by moving it back from the archive.\n\n\
Sets the item's status to 'open' and moves it from the archive directory back \
to the stack (or its original category). In Git repositories, uses 'git mv' to \
preserve history.",
        after_help = "Examples:\n  \
qstack reopen --id 260109             Reopen by full ID\n  \
qstack reopen --id 2601               Reopen by partial ID\n  \
qstack list                           Verify item is back in open list"
    )]
    Reopen {
        /// Item ID (partial match supported)
        #[arg(
            long,
            required = true,
            help = "Item ID to reopen (partial match supported)"
        )]
        id: String,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{} {err:#}", "error:".red().bold());
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => commands::init(),

        Commands::New {
            title,
            label,
            category,
        } => commands::new(NewArgs {
            title,
            labels: label,
            category,
        }),

        Commands::List {
            open,
            closed,
            id,
            label,
            author,
            sort,
        } => commands::list(&ListFilter {
            open,
            closed,
            id,
            label,
            author,
            sort,
        }),

        Commands::Get {
            label,
            author,
            sort,
            no_open,
            closed,
        } => commands::get(&GetArgs {
            label,
            author,
            sort,
            no_open,
            closed,
        }),

        Commands::Update {
            id,
            title,
            label,
            category,
            no_category,
        } => commands::update(UpdateArgs {
            id,
            title,
            labels: label,
            category,
            clear_category: no_category,
        }),

        Commands::Close { id } => commands::execute_close(&id),

        Commands::Reopen { id } => commands::execute_reopen(&id),
    }
}
