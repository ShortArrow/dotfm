use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;

use crate::commands::apply;
use crate::config::Config;
use crate::error::Error;
use crate::registry;

pub fn run(
    dotfiles_override: Option<&Path>,
    tools: &[String],
    then_apply: bool,
    dry_run: bool,
    force_on_apply: bool,
) -> Result<ExitCode> {
    let cfg_path = Config::default_path()?;
    let mut cfg = Config::load(&cfg_path)?;
    let root = super::util::effective_root(&cfg, dotfiles_override)?;
    let reg = registry::load(&root)?;

    for t in tools {
        if !reg.tools.contains_key(t) {
            return Err(Error::UnknownTool {
                name: t.clone(),
                available: super::util::sorted_keys(&reg),
            }
            .into());
        }
    }

    let mut added = vec![];
    let mut skipped = vec![];
    for t in tools {
        if cfg.enable(t) {
            added.push(t.clone());
        } else {
            skipped.push(t.clone());
        }
    }

    if !dry_run {
        cfg.save()?;
    }

    for t in &added {
        println!("enabled {t}");
    }
    for t in &skipped {
        println!("already enabled: {t}");
    }

    if then_apply {
        return apply::run(dotfiles_override, tools, force_on_apply, dry_run);
    }

    if !added.is_empty() {
        println!("run `dotup apply` to create symlinks.");
    }
    Ok(ExitCode::SUCCESS)
}
