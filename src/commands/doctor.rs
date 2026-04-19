use std::path::Path;
use std::process::{Command, ExitCode};

use anyhow::{Context, Result};

use crate::config::Config;
use crate::link::{self, LinkState};
use crate::os::{self, Os};
use crate::registry::{self, Registry, Tool};
use crate::style::Icons;

/// Exit codes:
///   0 — everything healthy
///   1 — at least one check failed
///   2 — a tool-specific doctor script exited non-zero
pub fn run(
    dotfiles_override: Option<&Path>,
    tools_filter: &[String],
    skip_generic: bool,
    icons: Icons,
) -> Result<ExitCode> {
    let cfg_path = Config::default_path()?;
    let cfg = Config::load(&cfg_path)?;
    let root = super::util::effective_root(&cfg, dotfiles_override)?;
    let reg = registry::load(&root)?;
    let current_os = Os::current();
    let enabled = cfg.enabled();

    let mut had_failure = false;
    let mut script_failed = false;

    if !skip_generic {
        println!("{} generic checks", icons.tool_header);
        had_failure |= !check_env(&root, icons)?;
        had_failure |= !check_enabled_links(&reg, &enabled, &root, current_os, icons)?;
        #[cfg(windows)]
        {
            had_failure |= !check_windows_developer_mode(icons)?;
        }
    }

    let targets: Vec<&String> = if tools_filter.is_empty() {
        enabled.iter().collect()
    } else {
        tools_filter.iter().collect()
    };

    for name in &targets {
        let Some(tool) = reg.tools.get(name.as_str()) else {
            println!(
                "{} {name}  {}  not in registry",
                icons.tool_header, icons.missing
            );
            had_failure = true;
            continue;
        };
        let Some(doctor_map) = &tool.doctor else {
            continue;
        };
        let Some(script_rel) = doctor_map.pick(current_os) else {
            continue;
        };

        println!("{} {name} doctor", icons.tool_header);
        match run_doctor_script(&root, script_rel, tool, current_os) {
            Ok(true) => println!("  {}  {name} doctor passed", icons.ok),
            Ok(false) => {
                println!("  {}  {name} doctor reported issues", icons.wrong);
                script_failed = true;
            }
            Err(e) => {
                eprintln!("  ! {name}: {e:#}");
                script_failed = true;
            }
        }
    }

    let exit = if script_failed {
        ExitCode::from(2)
    } else if had_failure {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    };
    Ok(exit)
}

/// Verify that essential environment variables resolve to usable paths.
fn check_env(root: &Path, icons: Icons) -> Result<bool> {
    let mut ok = true;

    let pairs = [
        ("HOME/USERPROFILE", "~"),
        ("XDG_CONFIG_HOME", "$XDG_CONFIG_HOME"),
    ];
    for (label, expr) in pairs {
        match os::expand(expr) {
            Ok(p) => {
                let mark = if p.exists() { icons.ok } else { icons.wrong };
                println!("  {mark}  {label}: {}", p.display());
                if !p.exists() {
                    ok = false;
                }
            }
            Err(e) => {
                println!("  {}  {label}: {e}", icons.wrong);
                ok = false;
            }
        }
    }

    let mark = if root.is_dir() { icons.ok } else { icons.wrong };
    println!("  {mark}  dotfiles_root: {}", root.display());
    if !root.is_dir() {
        ok = false;
    }

    Ok(ok)
}

/// Walk all enabled tools and verify each link is CorrectLink.
fn check_enabled_links(
    reg: &Registry,
    enabled: &[String],
    root: &Path,
    current_os: Os,
    icons: Icons,
) -> Result<bool> {
    let mut ok = true;
    if enabled.is_empty() {
        println!("  {}  no tools enabled", icons.skipped);
        return Ok(ok);
    }

    for name in enabled {
        let Some(tool) = reg.tools.get(name) else {
            println!(
                "  {}  {name} enabled but missing from registry",
                icons.wrong
            );
            ok = false;
            continue;
        };
        for link in &tool.links {
            let resolved = match link.resolve(root, current_os, os::expand) {
                Ok(Some(items)) => items,
                Ok(None) => continue,
                Err(e) => {
                    println!("  {}  {name}: {e:#}", icons.wrong);
                    ok = false;
                    continue;
                }
            };
            for item in resolved {
                match link::inspect(&item.src, &item.dst) {
                    Ok(LinkState::CorrectLink) => {}
                    Ok(state) => {
                        let badge = match state {
                            LinkState::Missing => icons.missing,
                            LinkState::WrongLink { .. } => icons.wrong,
                            LinkState::ExistingFile | LinkState::ExistingDir => icons.conflict,
                            LinkState::CorrectLink => unreachable!(),
                        };
                        println!("  {badge}  {name}: {}", item.dst.display());
                        ok = false;
                    }
                    Err(e) => {
                        println!(
                            "  {}  {name}: inspect {} failed: {e}",
                            icons.wrong,
                            item.dst.display()
                        );
                        ok = false;
                    }
                }
            }
        }
    }

    Ok(ok)
}

#[cfg(windows)]
fn check_windows_developer_mode(icons: Icons) -> Result<bool> {
    use std::process::Command;
    // Dev Mode exposes AllowDevelopmentWithoutDevLicense=1 under this key.
    let output = Command::new("reg")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock",
            "/v",
            "AllowDevelopmentWithoutDevLicense",
        ])
        .output();
    let enabled = match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            text.contains("0x1")
        }
        _ => false,
    };
    if enabled {
        println!(
            "  {}  Windows Developer Mode: enabled (symlink creation allowed)",
            icons.ok
        );
        Ok(true)
    } else {
        println!(
            "  {}  Windows Developer Mode: disabled — symlink creation requires elevation",
            icons.wrong
        );
        Ok(false)
    }
}

fn run_doctor_script(root: &Path, script_rel: &str, _tool: &Tool, os: Os) -> Result<bool> {
    let script_path = root.join(script_rel);
    if !script_path.exists() {
        anyhow::bail!("doctor script not found: {}", script_path.display());
    }
    let status = match os {
        Os::Windows => Command::new("pwsh")
            .args(["-NoLogo", "-NoProfile", "-File"])
            .arg(&script_path)
            .status()
            .with_context(|| format!("spawning pwsh for {}", script_path.display()))?,
        Os::Linux => Command::new("bash")
            .arg(&script_path)
            .status()
            .with_context(|| format!("spawning bash for {}", script_path.display()))?,
    };
    Ok(status.success())
}
