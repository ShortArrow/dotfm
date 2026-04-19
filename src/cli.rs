use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "dotup", version, about)]
pub struct Cli {
    /// Preview changes without touching the filesystem.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Verbose logging (equivalent to RUST_LOG=dotup=debug).
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Override dotfiles root (otherwise taken from config.toml).
    #[arg(long, global = true, value_name = "PATH")]
    pub dotfiles: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Cmd,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Create ~/.config/dotup/config.toml for this machine.
    Init {
        /// Overwrite the existing config.
        #[arg(long)]
        force: bool,
    },

    /// Enable tools on this machine.
    Add {
        /// Tool names to enable.
        #[arg(required = true)]
        tools: Vec<String>,

        /// Run `apply` immediately after enabling.
        #[arg(long)]
        apply: bool,
    },

    /// Disable tools on this machine (removes their symlinks).
    Remove {
        /// Tool names to disable.
        #[arg(required = true)]
        tools: Vec<String>,
    },

    /// Create or refresh symlinks for enabled tools.
    Apply {
        /// Apply only these tools (must already be enabled).
        tools: Vec<String>,

        /// Replace existing files/directories by backing them up as `.bak`.
        #[arg(long)]
        force: bool,
    },

    /// Show which symlinks are in sync.
    Status,

    /// List all tools declared in dotup.toml.
    List,
}
