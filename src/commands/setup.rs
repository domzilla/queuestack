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

use crate::ui::select_from_list;

use anyhow::{Context, Result};
use clap_complete::Shell;
use owo_colors::OwoColorize;

use clap::Command;

use crate::{
    config::GlobalConfig,
    constants::{
        BASHRC_FILE, BASH_COMPLETIONS_DIR, BASH_COMPLETION_FILE, BASH_PROFILE_FILE,
        ELVISH_COMPLETIONS_DIR, ELVISH_COMPLETION_FILE, FISH_COMPLETIONS_DIR, FISH_COMPLETION_FILE,
        POWERSHELL_CONFIG_DIR_UNIX, POWERSHELL_CONFIG_DIR_WINDOWS, POWERSHELL_PROFILE_FILE,
        ZSHRC_FILE, ZSH_COMPLETIONS_DIR, ZSH_COMPLETION_FILE,
    },
};

use super::completions::generate_to_string;

/// Executes the setup command.
///
/// The `cmd` parameter should be a clone of the CLI command for generating completions.
/// The `shell_override` parameter allows explicit shell specification, bypassing detection.
pub fn execute(cmd: &mut Command, shell_override: Option<Shell>) -> Result<()> {
    eprintln!("{}\n", "Setting up qstack...".bold());

    // Step 1: Ensure global config exists
    setup_global_config()?;

    // Step 2: Install shell completions for detected shell
    setup_completions(cmd, shell_override)?;

    eprintln!("\n{} Setup complete!", "✓".green().bold());

    Ok(())
}

/// Creates the global config file if it doesn't exist
fn setup_global_config() -> Result<()> {
    let path = GlobalConfig::path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    if GlobalConfig::create_default_if_missing()? {
        eprintln!("{} Created global config: {}", "✓".green(), path.display());
    } else {
        eprintln!(
            "{} Global config already exists: {}",
            "✓".green(),
            path.display()
        );
    }

    Ok(())
}

/// Detects the user's shell or prompts them to select one.
///
/// Detection uses `$SHELL` (the user's configured login shell). If detection
/// fails or the shell is unrecognized, prompts the user interactively.
fn detect_or_prompt_shell() -> Result<Shell> {
    // Try $SHELL environment variable (user's configured login shell)
    if let Ok(shell_path) = env::var("SHELL") {
        let name = shell_path.rsplit('/').next().unwrap_or(&shell_path);
        // Handle login shell prefix (e.g., "-zsh")
        let name = name.strip_prefix('-').unwrap_or(name);

        if let Some(shell) = match name {
            "zsh" => Some(Shell::Zsh),
            "bash" => Some(Shell::Bash),
            "fish" => Some(Shell::Fish),
            "elvish" => Some(Shell::Elvish),
            "powershell" | "pwsh" => Some(Shell::PowerShell),
            _ => None,
        } {
            return Ok(shell);
        }
    }

    // Detection failed, prompt interactively
    prompt_shell_selection()
}

/// Prompts the user to select their shell interactively.
fn prompt_shell_selection() -> Result<Shell> {
    let shells = ["zsh", "bash", "fish", "elvish", "powershell"];

    let selection = select_from_list("Which shell do you use?", &shells)?;

    Ok(match selection {
        0 => Shell::Zsh,
        1 => Shell::Bash,
        2 => Shell::Fish,
        3 => Shell::Elvish,
        4 => Shell::PowerShell,
        _ => unreachable!(),
    })
}

/// Returns the completion file path for each shell
fn get_completion_path(shell: Shell) -> Option<PathBuf> {
    let home = dirs::home_dir()?;

    match shell {
        Shell::Zsh => Some(home.join(ZSH_COMPLETIONS_DIR).join(ZSH_COMPLETION_FILE)),
        Shell::Bash => Some(home.join(BASH_COMPLETIONS_DIR).join(BASH_COMPLETION_FILE)),
        Shell::Fish => Some(home.join(FISH_COMPLETIONS_DIR).join(FISH_COMPLETION_FILE)),
        Shell::Elvish => Some(
            home.join(ELVISH_COMPLETIONS_DIR)
                .join(ELVISH_COMPLETION_FILE),
        ),
        // PowerShell doesn't have a standard auto-load directory
        _ => None,
    }
}

/// Returns shell rc file path for shells that need manual sourcing
fn get_rc_file_path(shell: Shell) -> Option<PathBuf> {
    let home = dirs::home_dir()?;

    match shell {
        Shell::Zsh => Some(home.join(ZSHRC_FILE)),
        Shell::Bash => {
            // Prefer .bashrc, fall back to .bash_profile
            let bashrc = home.join(BASHRC_FILE);
            if bashrc.exists() {
                Some(bashrc)
            } else {
                Some(home.join(BASH_PROFILE_FILE))
            }
        }
        Shell::PowerShell => {
            if cfg!(windows) {
                Some(
                    home.join(POWERSHELL_CONFIG_DIR_WINDOWS)
                        .join(POWERSHELL_PROFILE_FILE),
                )
            } else {
                Some(
                    home.join(POWERSHELL_CONFIG_DIR_UNIX)
                        .join(POWERSHELL_PROFILE_FILE),
                )
            }
        }
        _ => None,
    }
}

/// Sets up shell completions for the specified or detected shell.
fn setup_completions(cmd: &mut Command, shell_override: Option<Shell>) -> Result<()> {
    let shell = match shell_override {
        Some(s) => s,
        None => detect_or_prompt_shell()?,
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
