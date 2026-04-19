use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::link::{self, LinkState};
use crate::os::{self, Os};
use crate::registry;
use crate::style::Icons;

pub fn run(dotfiles_override: Option<&Path>, icons: Icons) -> Result<ExitCode> {
    let cfg_path = Config::default_path()?;
    let cfg = Config::load(&cfg_path)?;
    let root = super::util::effective_root(&cfg, dotfiles_override)?;
    let reg = registry::load(&root)?;
    let current_os = Os::current();

    let enabled = cfg.enabled();
    if enabled.is_empty() {
        println!("no tools enabled");
        return Ok(ExitCode::SUCCESS);
    }

    let mut any_bad = false;

    for name in &enabled {
        let Some(tool) = reg.tools.get(name) else {
            println!("[missing from registry] {name}");
            any_bad = true;
            continue;
        };
        println!("{name}:");
        for link in &tool.links {
            let resolved = link
                .resolve(&root, current_os, os::expand)
                .with_context(|| format!("resolving link for {name}"))?;
            let Some(items) = resolved else {
                continue;
            };
            for item in items {
                let state = link::inspect(&item.src, &item.dst)
                    .with_context(|| format!("inspecting {}", item.dst.display()))?;
                let (badge, is_bad) = match &state {
                    LinkState::CorrectLink => (icons.ok, false),
                    LinkState::Missing => (icons.missing, true),
                    LinkState::WrongLink { .. } => (icons.wrong, true),
                    LinkState::ExistingFile | LinkState::ExistingDir => (icons.conflict, true),
                };
                if is_bad {
                    any_bad = true;
                }
                println!("  {badge}  {}", item.dst.display());
            }
        }
    }

    Ok(if any_bad {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}
