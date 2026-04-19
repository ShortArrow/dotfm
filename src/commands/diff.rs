//! `dotfm diff` — show what `apply` would change.
//!
//! Three layers, printed in order so the user can scan top-down:
//!
//! 1. **Registry drift** — tools registered in `dotfm.toml` but not in
//!    `enabled`, or in `enabled` but missing from the registry.
//! 2. **Link drift** — per-link state for each enabled tool (missing /
//!    wrong / conflict / ok).
//! 3. **Content drift** — when a destination is a regular file that differs
//!    from its source, print a unified diff (only with `--content`).
//!
//! Exit code is 0 when nothing differs, 1 otherwise.

use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use similar::{ChangeTag, TextDiff};

use crate::config::Config;
use crate::link::{self, LinkState};
use crate::os::{self, Os};
use crate::registry;
use crate::style::Icons;

pub fn run(
    dotfiles_override: Option<&Path>,
    tools_filter: &[String],
    show_content: bool,
    icons: Icons,
) -> Result<ExitCode> {
    let cfg_path = Config::default_path()?;
    let cfg = Config::load(&cfg_path)?;
    let root = super::util::effective_root(&cfg, dotfiles_override)?;
    let reg = registry::load(&root)?;
    let current_os = Os::current();
    let enabled = cfg.enabled();

    let mut any_drift = false;

    // ---- Layer 1: registry drift ----
    println!("{} registry drift", icons.tool_header);
    let orphaned_in_enabled: Vec<&String> = enabled
        .iter()
        .filter(|t| !reg.tools.contains_key(*t))
        .collect();
    let available_but_disabled: Vec<&String> = reg
        .tools
        .keys()
        .filter(|name| {
            !enabled.contains(name)
                && reg
                    .tools
                    .get(*name)
                    .map(|t| t.supports(current_os))
                    .unwrap_or(false)
        })
        .collect();

    if orphaned_in_enabled.is_empty() && available_but_disabled.is_empty() {
        println!("  {}  enabled list matches registry", icons.ok);
    } else {
        for t in &orphaned_in_enabled {
            println!(
                "  {}  {t}: enabled but missing from registry",
                icons.missing
            );
            any_drift = true;
        }
        for t in &available_but_disabled {
            println!("  {}  {t}: available, not enabled", icons.skipped);
        }
    }

    // ---- Layer 2: link drift ----
    println!();
    println!("{} link drift", icons.tool_header);

    let targets: Vec<&String> = if !tools_filter.is_empty() {
        tools_filter.iter().collect()
    } else {
        enabled.iter().collect()
    };

    if targets.is_empty() {
        println!("  {}  no tools to check", icons.skipped);
    }

    // For content diffs we collect ExistingFile conflicts first so we don't
    // interleave their diffs with the link drift summary.
    let mut content_targets: Vec<(String, std::path::PathBuf, std::path::PathBuf)> = Vec::new();

    for name in &targets {
        let Some(tool) = reg.tools.get(name.as_str()) else {
            continue;
        };
        if !tool.supports(current_os) {
            continue;
        }
        let mut tool_drift = false;
        let mut tool_output = Vec::<String>::new();

        for link in &tool.links {
            let resolved = link
                .resolve(&root, current_os, os::expand)
                .with_context(|| format!("resolving link for {name}"))?;
            let Some(items) = resolved else { continue };
            for item in items {
                let state = link::inspect(&item.src, &item.dst)
                    .with_context(|| format!("inspecting {}", item.dst.display()))?;
                match state {
                    LinkState::CorrectLink => {}
                    LinkState::Missing => {
                        tool_output.push(format!("  {}  {}", icons.missing, item.dst.display()));
                        tool_drift = true;
                    }
                    LinkState::WrongLink { target } => {
                        tool_output.push(format!(
                            "  {}  {}\n      expected: {}\n      actual:   {}",
                            icons.wrong,
                            item.dst.display(),
                            item.src.display(),
                            target.display()
                        ));
                        tool_drift = true;
                    }
                    LinkState::ExistingFile => {
                        tool_output.push(format!(
                            "  {}  {} (file, not a symlink)",
                            icons.conflict,
                            item.dst.display()
                        ));
                        tool_drift = true;
                        content_targets.push((
                            name.to_string(),
                            item.src.clone(),
                            item.dst.clone(),
                        ));
                    }
                    LinkState::ExistingDir => {
                        tool_output.push(format!(
                            "  {}  {} (directory, not a symlink)",
                            icons.conflict,
                            item.dst.display()
                        ));
                        tool_drift = true;
                    }
                }
            }
        }

        if tool_drift {
            println!("{name}:");
            for line in tool_output {
                println!("{line}");
            }
            any_drift = true;
        }
    }

    if !any_drift && targets.iter().any(|t| reg.tools.contains_key(t.as_str())) {
        println!("  {}  all enabled links in sync", icons.ok);
    }

    // ---- Layer 3: content drift (opt-in) ----
    if show_content {
        println!();
        println!("{} content drift", icons.tool_header);
        if content_targets.is_empty() {
            println!("  {}  no file conflicts to diff", icons.skipped);
        } else {
            for (name, src, dst) in content_targets {
                print_content_diff(&name, &src, &dst)?;
            }
        }
    } else if !content_targets.is_empty() {
        println!();
        println!(
            "note: {} file conflict(s) have content diffs; pass --content to see them.",
            content_targets.len()
        );
    }

    Ok(if any_drift {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// Emit a unified-ish diff between `src` (expected) and `dst` (actual file).
/// Returns early with a note if either file is binary or unreadable.
fn print_content_diff(tool: &str, src: &Path, dst: &Path) -> Result<()> {
    println!("--- {tool}: {} (expected)", src.display());
    println!("+++ {tool}: {} (actual)", dst.display());

    let src_bytes = std::fs::read(src).with_context(|| format!("reading {}", src.display()))?;
    let dst_bytes = std::fs::read(dst).with_context(|| format!("reading {}", dst.display()))?;

    if is_binary(&src_bytes) || is_binary(&dst_bytes) {
        println!("  (binary files, diff suppressed)");
        return Ok(());
    }

    let src_text = String::from_utf8_lossy(&src_bytes);
    let dst_text = String::from_utf8_lossy(&dst_bytes);
    let diff = TextDiff::from_lines(src_text.as_ref(), dst_text.as_ref());

    for change in diff.iter_all_changes() {
        let sigil = match change.tag() {
            ChangeTag::Equal => continue,
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
        };
        print!("{sigil}{}", change);
    }
    Ok(())
}

/// Heuristic: any NUL byte in the first 8KB marks the file as binary.
fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|&b| b == 0)
}
