use std::path::Path;
use std::process::{Command, ExitCode};

use anyhow::{Context, Result};

use crate::config::Config;
use crate::error::Error;
use crate::link::{self, Change};
use crate::os::{self, Os};
use crate::registry::{self, Tool};
use crate::style::Icons;

pub fn run(
    dotfiles_override: Option<&Path>,
    tools_filter: &[String],
    force: bool,
    dry_run: bool,
    icons: Icons,
) -> Result<ExitCode> {
    let cfg_path = Config::default_path()?;
    let cfg = Config::load(&cfg_path)?;
    let root = super::util::effective_root(&cfg, dotfiles_override)?;
    let reg = registry::load(&root)?;
    let current_os = Os::current();

    let enabled = cfg.enabled();
    let targets: Vec<&String> = if tools_filter.is_empty() {
        enabled.iter().collect()
    } else {
        for t in tools_filter {
            if !enabled.contains(t) {
                return Err(Error::NotEnabled { name: t.clone() }.into());
            }
        }
        tools_filter.iter().collect()
    };

    if targets.is_empty() {
        println!("no tools enabled. use `dotfm add <tool>` first.");
        return Ok(ExitCode::SUCCESS);
    }

    let mut failed = 0usize;

    for name in &targets {
        let Some(tool) = reg.tools.get(name.as_str()) else {
            eprintln!("warning: tool `{name}` not found in registry; skipping");
            failed += 1;
            continue;
        };
        if !tool.supports(current_os) {
            println!("skip {name} (no support for {})", current_os.as_str());
            continue;
        }

        println!("{} {name}", icons.tool_header);
        let ok = apply_tool(name, tool, &root, current_os, force, dry_run, icons)
            .map_err(|e| {
                eprintln!("  ! {name}: {e:#}");
            })
            .is_ok();
        if !ok {
            failed += 1;
        }
    }

    if failed > 0 {
        eprintln!("{failed} tool(s) failed");
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn apply_tool(
    name: &str,
    tool: &Tool,
    root: &Path,
    current_os: Os,
    force: bool,
    dry_run: bool,
    icons: Icons,
) -> Result<()> {
    for link in &tool.links {
        let resolved = link
            .resolve(root, current_os, os::expand)
            .with_context(|| format!("resolving link for {name}"))?;
        let Some(items) = resolved else {
            continue;
        };
        for item in items {
            if !item.src.exists() {
                anyhow::bail!("src does not exist: {}", item.src.display());
            }
            let change = link::ensure(&item.src, &item.dst, force, dry_run).with_context(|| {
                format!("link {} -> {}", item.dst.display(), item.src.display())
            })?;
            let badge = match change {
                Change::Noop => icons.noop,
                Change::Created => icons.link,
                Change::Updated => icons.relink,
                Change::BackedUpAndReplaced => icons.backup,
                Change::Removed => icons.removed,
                Change::Skipped { .. } => icons.skipped,
            };
            println!("  {badge}  {}", item.dst.display());
        }
    }

    // script delegation (run only if no explicit links)
    if let Some(script_map) = &tool.script {
        if let Some(script_rel) = script_map.pick(current_os) {
            let script_path = root.join(script_rel);
            if dry_run {
                println!("  would run script: {}", script_path.display());
            } else {
                run_script(&script_path, current_os)?;
            }
        }
    }

    // post_apply hooks
    for hook in &tool.post_apply {
        if !hook.applies_to(current_os) {
            continue;
        }
        if hook.run.is_empty() {
            continue;
        }
        if dry_run {
            println!("  would run: {}", hook.run.join(" "));
            continue;
        }
        println!("  hook: {}", hook.run.join(" "));
        let status = Command::new(&hook.run[0])
            .args(&hook.run[1..])
            .status()
            .with_context(|| format!("spawning {}", hook.run[0]))?;
        if !status.success() {
            anyhow::bail!("hook failed: {}", hook.run.join(" "));
        }
    }

    Ok(())
}

fn run_script(script: &Path, os: Os) -> Result<()> {
    let status = match os {
        Os::Windows => Command::new("pwsh")
            .args(["-NoLogo", "-NoProfile", "-File"])
            .arg(script)
            .status()
            .with_context(|| format!("spawning pwsh for {}", script.display()))?,
        Os::Linux => Command::new("bash")
            .arg(script)
            .status()
            .with_context(|| format!("spawning bash for {}", script.display()))?,
    };
    if !status.success() {
        anyhow::bail!("script {} exited {}", script.display(), status);
    }
    Ok(())
}
