//! # qstack CLI
//!
//! Command-line interface for the qstack task/issue tracker.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use anyhow::Result;
use clap::builder::{styling::AnsiColor, Styles};
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;

use clap::CommandFactory;
use clap_complete::Shell;
use qstack::commands::{
    self, AttachAddArgs, AttachRemoveArgs, AttachmentsArgs, CategoriesArgs, LabelsArgs, ListFilter,
    NewArgs, SearchArgs, SortBy, UpdateArgs,
};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().bold())
    .usage(AnsiColor::Yellow.on_default().bold())
    .literal(AnsiColor::Green.on_default())
    .placeholder(AnsiColor::Cyan.on_default());

// Help text colorization macros
macro_rules! h {
    ($s:expr) => {
        concat!("\x1b[1;33m", $s, "\x1b[0m")
    };
}

macro_rules! c {
    ($s:expr) => {
        concat!("\x1b[32m", $s, "\x1b[0m")
    };
}

macro_rules! a {
    ($s:expr) => {
        concat!("\x1b[36m", $s, "\x1b[0m")
    };
}

macro_rules! global_help {
    () => {
        concat!(
            h!("Configuration Files:"),
            "\n  ",
            "~/.qstack          Global configuration (user name, editor, ID pattern)\n  ",
            ".qstack            Project configuration (stack directory, archive directory)\n\n",
            h!("ID Pattern Tokens:"),
            "\n  ",
            "%y  Year (2 digits)           %m  Month (01-12)\n  ",
            "%d  Day of month (01-31)      %j  Day of year (001-366)\n  ",
            "%T  Time as Base32 (4 chars)  %R  Random Base32 char (repeat: %RRR)\n  ",
            "%%  Literal percent sign\n\n",
            h!("Getting Started:"),
            "\n  ",
            c!("qstack init"),
            "                    Initialize project in current directory\n  ",
            c!("qstack new "),
            a!("\"My first task\""),
            "     Create a new item\n  ",
            c!("qstack list"),
            "                    List all open items\n  ",
            c!("qstack close --id "),
            a!("<ID>"),
            "         Close an item\n\n",
            h!("Learn more:"),
            "\n  ",
            c!("qstack "),
            a!("<COMMAND>"),
            c!(" --help"),
            "        Show detailed help for a command"
        )
    };
}

#[derive(Parser)]
#[command(name = "qstack")]
#[command(author = "Dominic Rodemer")]
#[command(version)]
#[command(styles = STYLES)]
#[command(about = "CLI-based task and issue tracker with documentation-as-code philosophy")]
#[command(
    long_about = "qstack is a CLI-based task and issue tracker that follows a documentation-as-code \
philosophy. Items are stored as Markdown files with YAML frontmatter in a Git-friendly \
directory structure, making them easy to version, search, and collaborate on.

Each item gets a unique ID, a slugified filename, and can be organized into categories. \
The tool integrates with Git for seamless version control and supports customizable \
ID patterns using Crockford's Base32 encoding."
)]
#[command(after_help = global_help!())]
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
        after_help = concat!(
            h!("Examples:"), "\n  ",
            c!("qstack init"), "                     Initialize in current directory\n  ",
            c!("cd myproject && qstack init"), "     Initialize in a specific project\n\n",
            h!("Note:"), " Run this command once per project, typically at the repository root."
        )
    )]
    Init,

    /// Create a new item
    #[command(
        long_about = "Create a new item with the given title.\n\n\
Generates a unique ID, creates a Markdown file with YAML frontmatter, and opens \
your editor based on the 'interactive' config setting (default: true). Use -i to \
force the editor to open, or --no-interactive to skip it.\n\n\
The filename is derived from the ID and a slugified title (e.g., '260109-0A2B-fix-login-bug.md').\n\n\
The author is determined from (in order):\n  \
1. user_name in ~/.qstack\n  \
2. git config user.name (if use_git_user is true)\n  \
3. Interactive prompt (saved to ~/.qstack for future use)",
        after_help = concat!(
            h!("Examples:"), "\n  ",
            c!("qstack new "), a!("\"Fix login bug\""), "\n  ",
            c!("qstack new "), a!("\"Add dark mode\""), c!(" --label "), a!("feature ui"), "\n  ",
            c!("qstack new "), a!("\"Memory leak\""), c!(" --label "), a!("bug urgent"), c!(" --category "), a!("bugs"), "\n  ",
            c!("qstack new "), a!("\"Bug report\""), c!(" --attachment "), a!("screenshot.png debug.log"), "\n  ",
            c!("qstack new "), a!("\"Quick note\""), c!(" --no-interactive"), "       Skip editor\n\n",
            h!("Output:"), " Prints the relative path to the created file."
        )
    )]
    New {
        /// Title of the item
        title: String,

        /// Labels/tags for the item (multiple values allowed)
        #[arg(short, long, num_args = 1..)]
        label: Vec<String>,

        /// Category subdirectory for the item
        #[arg(short, long)]
        category: Option<String>,

        /// Files or URLs to attach (multiple values allowed)
        #[arg(short, long, num_args = 1..)]
        attachment: Vec<String>,

        /// Force interactive mode (open editor)
        #[arg(short = 'i', long, conflicts_with = "no_interactive")]
        interactive: bool,

        /// Force non-interactive mode (don't open editor)
        #[arg(long)]
        no_interactive: bool,
    },

    /// List items
    #[command(
        long_about = "List items in the current project.\n\n\
Shows all open items in a table format. Based on the 'interactive' config setting \
(default: true), presents a selector to choose an item to open. Use -i to force \
interactive selection, or --no-interactive to just display the table.\n\n\
Use filters to narrow down results.",
        after_help = concat!(
            h!("Examples:"), "\n  ",
            c!("qstack list"), "                        List items, select one to open\n  ",
            c!("qstack list --no-interactive"), "       Just show the table\n  ",
            c!("qstack list --closed"), "               List archived/closed items\n  ",
            c!("qstack list --label "), a!("bug"), "            Filter by label\n  ",
            c!("qstack list --author "), a!("\"John\""), "        Filter by author\n  ",
            c!("qstack list --sort "), a!("date"), "            Sort by creation date\n\n",
            h!("Interactive mode:"), " Use arrow keys to navigate, Enter to select, Esc to cancel."
        )
    )]
    List {
        /// Show only open items
        #[arg(long, help = "Show only open items (default if no status filter)")]
        open: bool,

        /// Show only closed items
        #[arg(long, help = "Show only closed/archived items")]
        closed: bool,

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

        /// Force interactive mode (show selector)
        #[arg(short = 'i', long, conflicts_with = "no_interactive")]
        interactive: bool,

        /// Force non-interactive mode (just show table)
        #[arg(long)]
        no_interactive: bool,
    },

    /// Search for items and interactively select one to open
    #[command(
        long_about = "Search for items by title or ID.\n\n\
Performs a case-insensitive substring search against item titles and IDs. Based on \
the 'interactive' config setting (default: true), presents a selector for matches. \
Use -i to force interactive selection, or --no-interactive to just list results.\n\n\
Search behavior:\n  \
- Single match: opens the item directly (in interactive mode)\n  \
- Multiple matches: shows interactive selector or lists results\n  \
- No matches: returns an error\n\n\
Use --full-text to also search within the markdown body content.",
        after_help = concat!(
            h!("Examples:"), "\n  ",
            c!("qstack search "), a!("\"login bug\""), "             Search and select interactively\n  ",
            c!("qstack search "), a!("\"260109\""), "                Search by ID\n  ",
            c!("qstack search "), a!("\"auth\""), c!(" --full-text"), "      Include body content in search\n  ",
            c!("qstack search "), a!("\"bug\""), c!(" --no-interactive"), "  Just list matching items\n  ",
            c!("qstack search "), a!("\"old task\""), c!(" --closed"), "     Search in archived items\n\n",
            h!("Interactive mode:"), " Use arrow keys to navigate, Enter to select, Esc to cancel."
        )
    )]
    Search {
        /// Search query (matches against title and ID)
        query: String,

        /// Also search in item body content
        #[arg(long, help = "Include body content in search")]
        full_text: bool,

        /// Force interactive mode (show selector)
        #[arg(short = 'i', long, conflicts_with = "no_interactive")]
        interactive: bool,

        /// Force non-interactive mode (just list matching items)
        #[arg(long)]
        no_interactive: bool,

        /// Search in closed/archived items
        #[arg(long, help = "Search in closed/archived items instead of open")]
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
        after_help = concat!(
            h!("Examples:"), "\n  ",
            c!("qstack update --id "), a!("260109"), c!(" --title "), a!("\"New title\""), "\n  ",
            c!("qstack update --id "), a!("2601"), c!(" --label "), a!("urgent"), c!(" --label "), a!("p1"), "\n  ",
            c!("qstack update --id "), a!("260109"), c!(" --category "), a!("bugs"), "\n  ",
            c!("qstack update --id "), a!("260109"), c!(" --no-category"), "          Move to stack root\n  ",
            c!("qstack update --id "), a!("26"), c!(" --title "), a!("\"Fix\""), c!(" --label "), a!("done"), "  Multiple updates\n\n",
            h!("Note:"), " The --id flag supports partial matching for convenience."
        )
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
        after_help = concat!(
            h!("Examples:"), "\n  ",
            c!("qstack close --id "), a!("260109"), "              Close by full ID\n  ",
            c!("qstack close --id "), a!("2601"), "                Close by partial ID\n  ",
            c!("qstack list --closed"), "                  View closed items\n  ",
            c!("qstack reopen --id "), a!("260109"), "             Reopen if needed"
        )
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
        after_help = concat!(
            h!("Examples:"), "\n  ",
            c!("qstack reopen --id "), a!("260109"), "             Reopen by full ID\n  ",
            c!("qstack reopen --id "), a!("2601"), "               Reopen by partial ID\n  ",
            c!("qstack list"), "                           Verify item is back in open list"
        )
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

    /// List all labels used across items
    #[command(
        long_about = "List all unique labels used across items with counts.\n\n\
Shows a table of all labels and how many items use each one. Based on the 'interactive' \
config setting (default: true), you can select a label to see all items with that label, \
then select an item to open. Use -i to force interactive mode, or --no-interactive to \
just display the table.",
        after_help = concat!(
            h!("Examples:"), "\n  ",
            c!("qstack labels"), "                         List all labels\n  ",
            c!("qstack labels --no-interactive"), "        Just show the table\n  ",
            c!("qstack labels -i"), "                      Force interactive selection"
        )
    )]
    Labels {
        /// Force interactive mode (show selector)
        #[arg(short = 'i', long, conflicts_with = "no_interactive")]
        interactive: bool,

        /// Force non-interactive mode (just show table)
        #[arg(long)]
        no_interactive: bool,
    },

    /// List all categories used across items
    #[command(
        long_about = "List all unique categories used across items with counts.\n\n\
Shows a table of all categories and how many items are in each one. Based on the 'interactive' \
config setting (default: true), you can select a category to see all items in it, then select \
an item to open. Use -i to force interactive mode, or --no-interactive to just display the table.",
        after_help = concat!(
            h!("Examples:"), "\n  ",
            c!("qstack categories"), "                     List all categories\n  ",
            c!("qstack categories --no-interactive"), "    Just show the table\n  ",
            c!("qstack categories -i"), "                  Force interactive selection"
        )
    )]
    Categories {
        /// Force interactive mode (show selector)
        #[arg(short = 'i', long, conflicts_with = "no_interactive")]
        interactive: bool,

        /// Force non-interactive mode (just show table)
        #[arg(long)]
        no_interactive: bool,
    },

    /// Manage item attachments (list, add, remove)
    #[command(
        long_about = "Manage attachments for items.\n\n\
Attachments can be files (copied to item directory) or URLs (stored as references). \
File attachments are renamed to follow the pattern: {ID}-Attachment-{N}-{name}.{ext}",
        after_help = concat!(
            h!("Examples:"), "\n  ",
            c!("qstack attachments list --id "), a!("260109"), "\n  ",
            c!("qstack attachments add --id "), a!("260109"), " ", a!("screenshot.png"), "\n  ",
            c!("qstack attachments add --id "), a!("260109"), " ", a!("https://github.com/issue/42"), "\n  ",
            c!("qstack attachments remove --id "), a!("260109"), " ", a!("1"), " ", a!("2")
        )
    )]
    Attachments {
        #[command(subcommand)]
        action: AttachmentsAction,
    },

    /// One-time setup: create global config and install shell completions
    #[command(
        long_about = "One-time setup for qstack.\n\n\
This command helps you get started with qstack by:\n  \
1. Creating the global configuration file (~/.qstack) if it doesn't exist\n  \
2. Detecting your shell from $SHELL and installing tab completions\n\n\
Run this once after installing qstack to enable tab completion for commands and arguments.\n\n\
The setup is idempotent - running it multiple times is safe and will just overwrite \
the completion script with the latest version.",
        after_help = concat!(
            h!("Examples:"), "\n  ",
            c!("qstack setup"), "           Run one-time setup\n\n",
            h!("Supported shells:"), " zsh, bash, fish, elvish, powershell\n\n",
            h!("Note:"), " If shell detection fails, use ", c!("qstack completions <shell>"), " manually."
        )
    )]
    Setup,

    /// Generate shell completion scripts
    #[command(
        long_about = "Generate shell completion scripts for various shells.\n\n\
Outputs the completion script to stdout. Redirect to a file or source directly \
in your shell configuration.\n\n\
For automatic installation, use 'qstack setup' instead.",
        after_help = concat!(
            h!("Examples:"), "\n  ",
            c!("qstack completions zsh"), " > ~/.zfunc/_qstack\n  ",
            c!("qstack completions bash"), " > ~/.local/share/bash-completion/completions/qstack\n  ",
            c!("qstack completions fish"), " > ~/.config/fish/completions/qstack.fish\n\n",
            h!("For zsh:"), " Add to ~/.zshrc:\n  ",
            "fpath=(~/.zfunc $fpath) && autoload -Uz compinit && compinit\n\n",
            h!("For bash:"), " Add to ~/.bashrc:\n  ",
            "source ~/.local/share/bash-completion/completions/qstack"
        )
    )]
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

/// Subcommands for the attachments command
#[derive(Subcommand)]
enum AttachmentsAction {
    /// List attachments for an item
    #[command(
        after_help = concat!(
            h!("Output:"), " Table with index, type (file/url), and attachment path or URL."
        )
    )]
    List {
        /// Item ID (partial match supported)
        #[arg(long, required = true)]
        id: String,
    },

    /// Add file or URL attachments to an item
    #[command(
        after_help = concat!(
            h!("Note:"), " Files are copied to the item directory. URLs are stored as references."
        )
    )]
    Add {
        /// Item ID (partial match supported)
        #[arg(long, required = true)]
        id: String,

        /// Files or URLs to attach
        #[arg(required = true)]
        sources: Vec<String>,
    },

    /// Remove attachments from an item by index
    #[command(
        after_help = concat!(
            h!("Note:"), " Use ", c!("qstack attachments list --id <ID>"), " to see indices."
        )
    )]
    Remove {
        /// Item ID (partial match supported)
        #[arg(long, required = true)]
        id: String,

        /// Attachment indices to remove (1-based)
        #[arg(required = true)]
        indices: Vec<usize>,
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
            attachment,
            interactive,
            no_interactive,
        } => commands::new(NewArgs {
            title,
            labels: label,
            category,
            attachments: attachment,
            interactive,
            no_interactive,
        }),

        Commands::List {
            open,
            closed,
            label,
            author,
            sort,
            interactive,
            no_interactive,
        } => commands::list(&ListFilter {
            open,
            closed,
            label,
            author,
            sort,
            interactive,
            no_interactive,
        }),

        Commands::Search {
            query,
            full_text,
            interactive,
            no_interactive,
            closed,
        } => commands::search(&SearchArgs {
            query,
            full_text,
            interactive,
            no_interactive,
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

        Commands::Labels {
            interactive,
            no_interactive,
        } => commands::labels(&LabelsArgs {
            interactive,
            no_interactive,
        }),

        Commands::Categories {
            interactive,
            no_interactive,
        } => commands::categories(&CategoriesArgs {
            interactive,
            no_interactive,
        }),

        Commands::Attachments { action } => match action {
            AttachmentsAction::List { id } => commands::attachments(&AttachmentsArgs { id }),
            AttachmentsAction::Add { id, sources } => {
                commands::attach_add(&AttachAddArgs { id, sources })
            }
            AttachmentsAction::Remove { id, indices } => {
                commands::attach_remove(&AttachRemoveArgs { id, indices })
            }
        },

        Commands::Setup => {
            let mut cmd = Cli::command();
            commands::setup(&mut cmd)
        }

        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            commands::completions(shell, &mut cmd)
        }
    }
}
