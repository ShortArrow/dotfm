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
            let Some(dst_raw) = link.dst.pick(current_os) else {
                continue;
            };
            let dst =
                os::expand(dst_raw).with_context(|| format!("expanding dst for tool {}", t))?;
            let src = root.join(&link.src);
            match link::remove_if_ours(&src, &dst, &root, dry_run)? {
                Change::Removed => println!("  {}  {}", icons.removed, dst.display()),
                Change::Skipped { reason } => {
                    any_skipped = true;
                    println!("  {}  {} ({reason})", icons.skipped, dst.display());
                }
                other => println!("  {other:?}  {}", dst.display()),
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
