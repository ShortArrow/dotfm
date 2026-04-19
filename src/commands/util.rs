use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::Config;
use crate::error::Error;
use crate::registry::Registry;

/// Resolve dotfiles root: `--dotfiles` flag wins over `config.toml`.
pub fn effective_root(cfg: &Config, override_: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = override_ {
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            std::env::current_dir()?.join(p)
        };
        return Ok(abs);
    }
    cfg.dotfiles_root()
        .ok_or_else(|| Error::DotfilesRootUnknown.into())
}

pub fn sorted_keys(reg: &Registry) -> String {
    let mut keys: Vec<&String> = reg.tools.keys().collect();
    keys.sort();
    keys.iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}
