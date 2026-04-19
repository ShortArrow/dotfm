use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::error::Error;
use crate::registry;

pub fn run(dotfiles: Option<&Path>, force: bool) -> Result<ExitCode> {
    let path = Config::default_path()?;
    let root = resolve_dotfiles_root(dotfiles)?;
    let _cfg = Config::create(&path, &root, force)?;
    println!("created {}", path.display());
    println!("  dotfiles_root = {}", root.display());
    println!("run `dotup add <tool>...` then `dotup apply`.");
    Ok(ExitCode::SUCCESS)
}

/// Resolve the dotfiles root: explicit flag wins; otherwise cwd must contain dotup.toml.
pub fn resolve_dotfiles_root(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            std::env::current_dir().context("cwd")?.join(p)
        };
        // Sanity-check that the registry exists.
        registry::load(&abs)?;
        return Ok(abs);
    }
    let cwd = std::env::current_dir().context("cwd")?;
    if cwd.join("dotup.toml").is_file() {
        return Ok(cwd);
    }
    Err(Error::DotfilesRootUnknown.into())
}
