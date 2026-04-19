use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::link::{self, Change};
use crate::os::{self, Os};
use crate::registry;
use crate::style::Icons;

pub fn run(
    dotfiles_override: Option<&Path>,
    tools: &[String],
    dry_run: bool,
    icons: Icons,
) -> Result<ExitCode> {
    let cfg_path = Config::default_path()?;
    let mut cfg = Config::load(&cfg_path)?;
    let root = super::util::effective_root(&cfg, dotfiles_override)?;
    let reg = registry::load(&root)?;
    let current_os = Os::current();

    let enabled = cfg.enabled();
    let mut any_skipped = false;

    for t in tools {
        if !enabled.contains(t) {
            println!("not enabled: {t}");
            continue;
        }
        let Some(tool) = reg.tools.get(t) else {
            println!("not in registry anymore: {t}");
            cfg.disable(t);
            continue;
        };

        for link in &tool.links {
            let resolved = link
                .resolve(&root, current_os, os::expand)
                .with_context(|| format!("resolving link for tool {t}"))?;
            let Some(items) = resolved else {
                continue;
            };
            for item in items {
                match link::remove_if_ours(&item.src, &item.dst, &root, dry_run)? {
                    Change::Removed => println!("  {}  {}", icons.removed, item.dst.display()),
                    Change::Skipped { reason } => {
                        any_skipped = true;
                        println!("  {}  {} ({reason})", icons.skipped, item.dst.display());
                    }
                    other => println!("  {other:?}  {}", item.dst.display()),
                }
            }
        }

        if tool.script.is_some() && tool.unscript.is_none() {
            eprintln!(
                "warning: tool `{t}` uses `script =` with no `unscript`; manual cleanup may be required"
            );
        }

        cfg.disable(t);
        println!("disabled {t}");
    }

    if !dry_run {
        cfg.save()?;
    }

    Ok(if any_skipped {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}
