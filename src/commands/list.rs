use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;

use crate::config::Config;
use crate::os::Os;
use crate::registry;
use crate::style::Icons;

pub fn run(dotfiles_override: Option<&Path>, icons: Icons) -> Result<ExitCode> {
    let cfg = Config::default_path()
        .ok()
        .and_then(|p| Config::load(&p).ok());
    let root = match (&cfg, dotfiles_override) {
        (_, Some(p)) => p.to_path_buf(),
        (Some(c), None) => c
            .dotfiles_root()
            .ok_or_else(|| anyhow::anyhow!("dotfiles_root not set in config.toml"))?,
        (None, None) => {
            let cwd = std::env::current_dir()?;
            if cwd.join("dotup.toml").is_file() {
                cwd
            } else {
                anyhow::bail!(
                    "no config found and no dotfiles root supplied. pass --dotfiles <path>"
                );
            }
        }
    };
    let reg = registry::load(&root)?;
    let current_os = Os::current();
    let enabled: Vec<String> = cfg.as_ref().map(|c| c.enabled()).unwrap_or_default();

    for (name, tool) in &reg.tools {
        let mark = if enabled.contains(name) {
            icons.enabled
        } else {
            icons.disabled
        };
        let os_ok = tool.supports(current_os);
        let os_note = if os_ok { "" } else { " (not on this OS)" };
        let desc = tool.description.as_deref().unwrap_or("");
        println!("{mark} {name:<16} {desc}{os_note}");
    }

    Ok(ExitCode::SUCCESS)
}
