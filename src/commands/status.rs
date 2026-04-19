use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::link::{self, LinkState};
use crate::os::{self, Os};
use crate::registry;

pub fn run(dotfiles_override: Option<&Path>) -> Result<ExitCode> {
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
            let Some(dst_raw) = link.dst.pick(current_os) else {
                continue;
            };
            let dst = os::expand(dst_raw).with_context(|| format!("expanding dst for {name}"))?;
            let src = root.join(&link.src);
            let state = link::inspect(&src, &dst)
                .with_context(|| format!("inspecting {}", dst.display()))?;
            let (badge, is_bad) = match &state {
                LinkState::CorrectLink => ("ok     ", false),
                LinkState::Missing => ("missing", true),
                LinkState::WrongLink { .. } => ("wrong  ", true),
                LinkState::ExistingFile => ("conflict(file)", true),
                LinkState::ExistingDir => ("conflict(dir) ", true),
            };
            if is_bad {
                any_bad = true;
            }
            println!("  [{badge}] {}", dst.display());
        }
    }

    Ok(if any_bad {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}
