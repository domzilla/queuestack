//! # Setup Command
//!
//! One-time setup for qstack: creates global config and installs shell completions.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap_complete::Shell;
use owo_colors::OwoColorize;

use clap::Command;

use crate::config::GlobalConfig;

use super::completions::generate_to_string;

/// Executes the setup command.
/// The `cmd` parameter should be a clone of the CLI command for generating completions.
pub fn execute(cmd: &mut Command) -> Result<()> {
    eprintln!("{}\n", "Setting up qstack...".bold());

    // Step 1: Ensure global config exists
    setup_global_config()?;

    // Step 2: Install shell completions for detected shell
    setup_completions(cmd)?;

    eprintln!("\n{} Setup complete!", "✓".green().bold());

    Ok(())
}

/// Creates the global config file if it doesn't exist
fn setup_global_config() -> Result<()> {
    let config_path = GlobalConfig::path();

    match &config_path {
        Some(path) if path.exists() => {
            eprintln!(
                "{} Global config already exists: {}",
                "✓".green(),
                path.display()
            );
        }
        Some(path) => {
            // Loading will auto-create with comments
            GlobalConfig::load()?;
            eprintln!("{} Created global config: {}", "✓".green(), path.display());
        }
        None => {
            eprintln!(
                "{} Could not determine home directory, skipping global config",
                "⚠".yellow()
            );
        }
    }

    Ok(())
}

/// Detects the user's shell from environment
fn detect_shell() -> Option<Shell> {
    // Check $SHELL environment variable
    if let Ok(shell_path) = env::var("SHELL") {
        let shell_name = shell_path.rsplit('/').next().unwrap_or(&shell_path);
        return match shell_name {
            "zsh" => Some(Shell::Zsh),
            "bash" => Some(Shell::Bash),
            "fish" => Some(Shell::Fish),
            "elvish" => Some(Shell::Elvish),
            "powershell" | "pwsh" => Some(Shell::PowerShell),
            _ => None,
        };
    }
    None
}

/// Returns the completion file path for each shell
fn get_completion_path(shell: Shell) -> Option<PathBuf> {
    let home = dirs::home_dir()?;

    match shell {
        Shell::Zsh => {
            // Use ~/.zfunc for custom completions
            Some(home.join(".zfunc").join("_qstack"))
        }
        Shell::Bash => {
            // Use ~/.local/share/bash-completion/completions/
            Some(
                home.join(".local")
                    .join("share")
                    .join("bash-completion")
                    .join("completions")
                    .join("qstack"),
            )
        }
        Shell::Fish => {
            // Fish auto-loads from this directory
            Some(
                home.join(".config")
                    .join("fish")
                    .join("completions")
                    .join("qstack.fish"),
            )
        }
        Shell::Elvish => {
            // Elvish completions directory
            Some(
                home.join(".config")
                    .join("elvish")
                    .join("lib")
                    .join("qstack.elv"),
            )
        }
        Shell::PowerShell => {
            // PowerShell doesn't have a standard auto-load directory
            // User needs to add to profile manually
            None
        }
        _ => None,
    }
}

/// Returns shell rc file path for shells that need manual sourcing
fn get_rc_file_path(shell: Shell) -> Option<PathBuf> {
    let home = dirs::home_dir()?;

    match shell {
        Shell::Zsh => Some(home.join(".zshrc")),
        Shell::Bash => {
            // Prefer .bashrc, fall back to .bash_profile
            let bashrc = home.join(".bashrc");
            if bashrc.exists() {
                Some(bashrc)
            } else {
                Some(home.join(".bash_profile"))
            }
        }
        Shell::PowerShell => {
            // PowerShell profile location
            if cfg!(windows) {
                Some(
                    home.join("Documents")
                        .join("PowerShell")
                        .join("Microsoft.PowerShell_profile.ps1"),
                )
            } else {
                Some(
                    home.join(".config")
                        .join("powershell")
                        .join("Microsoft.PowerShell_profile.ps1"),
                )
            }
        }
        _ => None,
    }
}

/// Sets up shell completions for the detected shell
fn setup_completions(cmd: &mut Command) -> Result<()> {
    let Some(shell) = detect_shell() else {
        eprintln!("{} Could not detect shell from $SHELL", "⚠".yellow());
        eprintln!(
            "  Run {} to generate completions manually",
            "qstack completions <shell>".green()
        );
        return Ok(());
    };

    // Generate completions
    let completions = generate_to_string(shell, cmd);

    // Get installation path
    let Some(install_path) = get_completion_path(shell) else {
        // No auto-install path, just print instructions
        print_manual_instructions(shell);
        return Ok(());
    };

    // Create parent directories
    if let Some(parent) = install_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Write completions file
    let mut file = File::create(&install_path).with_context(|| {
        format!(
            "Failed to create completions file: {}",
            install_path.display()
        )
    })?;
    file.write_all(completions.as_bytes())
        .with_context(|| format!("Failed to write completions: {}", install_path.display()))?;

    eprintln!(
        "{} Installed {} completions: {}",
        "✓".green(),
        format!("{shell:?}").to_lowercase(),
        install_path.display()
    );

    // Print any additional setup instructions
    print_activation_instructions(shell, &install_path);

    Ok(())
}

/// Prints instructions for activating completions
fn print_activation_instructions(shell: Shell, install_path: &Path) {
    match shell {
        Shell::Zsh => {
            let rc_path = get_rc_file_path(shell);
            eprintln!(
                "\n  {} To enable completions, add to {}:",
                "→".cyan(),
                rc_path
                    .as_ref()
                    .map_or_else(|| "~/.zshrc".to_string(), |p| p.display().to_string())
            );
            eprintln!(
                "    {}",
                r"fpath=(~/.zfunc $fpath) && autoload -Uz compinit && compinit".dimmed()
            );
            eprintln!(
                "    {}",
                "Then restart your shell or run: source ~/.zshrc".dimmed()
            );
        }
        Shell::Bash => {
            // bash-completion usually auto-loads from this directory
            eprintln!(
                "\n  {} Completions installed. If not auto-loaded, add to ~/.bashrc:",
                "→".cyan(),
            );
            eprintln!(
                "    {}",
                format!("source {}", install_path.display()).dimmed()
            );
        }
        Shell::Fish => {
            // Fish auto-loads completions
            eprintln!(
                "\n  {} Completions will be loaded automatically on next shell start.",
                "→".cyan(),
            );
        }
        Shell::Elvish => {
            eprintln!("\n  {} Add to ~/.config/elvish/rc.elv:", "→".cyan(),);
            eprintln!("    {}", "use qstack".dimmed());
        }
        _ => {}
    }
}

/// Prints instructions for shells without auto-install
fn print_manual_instructions(shell: Shell) {
    eprintln!("\n  {} Manual setup required for {:?}", "→".cyan(), shell);

    match shell {
        Shell::PowerShell => {
            eprintln!(
                "    Add to your PowerShell profile ({}$PROFILE{}):",
                "$".dimmed(),
                "".dimmed()
            );
            eprintln!(
                "    {}",
                "Invoke-Expression (& qstack completions powershell | Out-String)".dimmed()
            );
        }
        _ => {
            eprintln!(
                "    Run: {} {} > <completions-file>",
                "qstack completions".green(),
                format!("{shell:?}").to_lowercase().cyan()
            );
        }
    }
}
