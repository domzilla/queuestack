//! # Completions Command
//!
//! Generate shell completion scripts for various shells.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::io::{self, Write};

use anyhow::Result;
use clap::Command;
use clap_complete::{generate, Shell};

/// Arguments for the completions command
#[derive(Debug, Clone)]
pub struct CompletionsArgs {
    pub shell: Shell,
}

/// Generates shell completions and writes them to stdout.
/// The `cmd` parameter should be the CLI command (from `Cli::command()`).
pub fn execute(shell: Shell, cmd: &mut Command) -> Result<()> {
    let name = cmd.get_name().to_string();
    generate(shell, cmd, name, &mut io::stdout());
    io::stdout().flush()?;
    Ok(())
}

/// Generates shell completions and returns them as a string.
/// The `cmd` parameter should be a clone of the CLI command.
pub fn generate_to_string(shell: Shell, cmd: &mut Command) -> String {
    let name = cmd.get_name().to_string();
    let mut buf = Vec::new();
    generate(shell, cmd, name, &mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}
