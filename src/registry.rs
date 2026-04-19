use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::error::Error;
use crate::os::Os;

/// Top-level `dotup.toml` schema.
#[derive(Debug, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub tools: BTreeMap<String, Tool>,
}

#[derive(Debug, Deserialize)]
pub struct Tool {
    pub description: Option<String>,
    #[serde(default)]
    pub platforms: Option<Vec<String>>,
    #[serde(default)]
    pub links: Vec<LinkSpec>,
    #[serde(default)]
    pub post_apply: Vec<Hook>,
    #[serde(default)]
    pub script: Option<OsMap<String>>,
    #[serde(default)]
    pub unscript: Option<OsMap<String>>,
}

#[derive(Debug, Deserialize)]
pub struct LinkSpec {
    pub src: String,
    pub dst: OsMap<String>,
}

#[derive(Debug, Deserialize)]
pub struct Hook {
    pub run: Vec<String>,
    #[serde(default)]
    pub os: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct OsMap<T> {
    #[serde(default)]
    pub windows: Option<T>,
    #[serde(default)]
    pub linux: Option<T>,
}

impl<T> OsMap<T> {
    pub fn pick(&self, os: Os) -> Option<&T> {
        match os {
            Os::Windows => self.windows.as_ref(),
            Os::Linux => self.linux.as_ref(),
        }
    }
}

impl Tool {
    pub fn supports(&self, os: Os) -> bool {
        match &self.platforms {
            None => true,
            Some(list) => list.iter().any(|p| p == os.as_str()),
        }
    }
}

impl Hook {
    pub fn applies_to(&self, os: Os) -> bool {
        match &self.os {
            None => true,
            Some(list) => list.iter().any(|p| p == os.as_str()),
        }
    }
}

/// Load `<root>/dotup.toml`.
pub fn load(dotfiles_root: &Path) -> Result<Registry> {
    let path: PathBuf = dotfiles_root.join("dotup.toml");
    if !path.is_file() {
        return Err(Error::RegistryMissing {
            path: dotfiles_root.to_path_buf(),
        }
        .into());
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let reg: Registry =
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    Ok(reg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_minimal_registry() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("dotup.toml"),
            r#"
[tools.alacritty]
description = "terminal"
[[tools.alacritty.links]]
src = "alacritty"
dst.windows = "$APPDATA/alacritty"
dst.linux = "$HOME/.config/alacritty"
"#,
        )
        .unwrap();

        let reg = load(dir.path()).unwrap();
        let t = reg.tools.get("alacritty").unwrap();
        assert_eq!(t.description.as_deref(), Some("terminal"));
        assert_eq!(t.links.len(), 1);
        assert_eq!(t.links[0].src, "alacritty");
        assert_eq!(
            t.links[0].dst.pick(Os::Linux),
            Some(&"$HOME/.config/alacritty".to_string())
        );
    }

    #[test]
    fn load_registry_missing_errors() {
        let dir = tempdir().unwrap();
        let err = load(dir.path()).unwrap_err();
        assert!(err.to_string().contains("does not contain dotup.toml"));
    }
}
