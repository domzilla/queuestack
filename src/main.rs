//! # qstack CLI
//!
//! Command-line interface for the qstack task/issue tracker.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use anyhow::Result;
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;

use qstack::commands::{self, ListFilter, NewArgs, SortBy, UpdateArgs};

#[derive(Parser)]
#[command(name = "qstack")]
#[command(author = "Dominic Rodemer")]
#[command(version)]
#[command(about = "CLI-based task and issue tracker with documentation-as-code philosophy")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new qstack project in the current directory
    Init,

    /// Create a new item
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
    List {
        /// Show only open items
        #[arg(long)]
        open: bool,

        /// Show only closed items
        #[arg(long)]
        closed: bool,

        /// Show details of a specific item by ID
        #[arg(long)]
        id: Option<String>,

        /// Filter by label
        #[arg(long)]
        label: Option<String>,

        /// Filter by author
        #[arg(long)]
        author: Option<String>,

        /// Sort order
        #[arg(long, value_enum, default_value = "id")]
        sort: SortBy,
    },

    /// Update an existing item
    Update {
        /// Item ID (partial match supported)
        #[arg(long, required = true)]
        id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// Add labels (can be specified multiple times)
        #[arg(long)]
        label: Vec<String>,

        /// Move to category
        #[arg(long)]
        category: Option<String>,

        /// Remove from category (move to root)
        #[arg(long)]
        no_category: bool,
    },

    /// Close an item (move to archive)
    Close {
        /// Item ID (partial match supported)
        #[arg(long, required = true)]
        id: String,
    },

    /// Reopen a closed item (move from archive)
    Reopen {
        /// Item ID (partial match supported)
        #[arg(long, required = true)]
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
