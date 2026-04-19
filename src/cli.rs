use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::style::IconMode;

#[derive(Debug, Parser)]
#[command(name = "dotfm", version, about)]
pub struct Cli {
    /// Preview changes without touching the filesystem.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Verbose logging (equivalent to RUST_LOG=dotfm=debug).
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Override dotfiles root (otherwise taken from config.toml).
    #[arg(long, global = true, value_name = "PATH")]
    pub dotfiles: Option<PathBuf>,

    /// Output icon set. `auto` honors the NERD_FONT environment variable.
    #[arg(long, global = true, value_enum, default_value_t = IconMode::Auto)]
    pub icons: IconMode,

    #[command(subcommand)]
    pub command: Cmd,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Create ~/.config/dotfm/config.toml for this machine.
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

    /// List all tools declared in dotfm.toml.
    List,

    /// Show what `apply` would change: registry drift, link drift, and file content drift.
    Diff {
        /// Limit to these tools (default: all enabled).
        tools: Vec<String>,

        /// Show unified diff when a destination is a regular file that differs from its src.
        /// Skips binary files.
        #[arg(long)]
        content: bool,
    },

    /// Run health checks: symlink drift, Windows Developer Mode, and
    /// per-tool doctor scripts declared in dotfm.toml.
    ///
    /// With no arguments, only the fast generic checks run. Pass tool names
    /// to invoke those tools' doctor scripts, or `--all` to run every
    /// enabled tool's doctor.
    Doctor {
        /// Run these tools' doctor scripts (in addition to generic checks).
        tools: Vec<String>,

        /// Run every enabled tool's doctor script.
        #[arg(long, conflicts_with = "tools")]
        all: bool,

        /// Skip the generic environment checks.
        #[arg(long)]
        no_generic: bool,
    },
}
