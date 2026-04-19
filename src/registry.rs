use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::error::Error;
use crate::os::Os;

/// Top-level `dotfm.toml` schema.
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
    /// Path to a tool-specific doctor script (relative to dotfiles_root),
    /// invoked by `dotfm doctor`. Windows scripts run under pwsh, Linux under bash.
    #[serde(default)]
    pub doctor: Option<OsMap<String>>,
}

#[derive(Debug, Deserialize)]
pub struct LinkSpec {
    pub src: LinkSrc,
    pub dst: OsMap<String>,
}

/// Link source: either a single file/directory path (string form),
/// or a directory plus an explicit include list (table form).
///
/// ```toml
/// # Single path (backwards compatible)
/// src = "starship/starship.toml"
///
/// # Directory with include list: each listed file is linked individually
/// # under the destination directory, keeping its name.
/// src = { dir = "code", include = ["settings.json", "keybindings.json"] }
/// ```
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum LinkSrc {
    /// Symlink `<dotfiles_root>/<path>` to `<dst>` verbatim.
    Path(String),
    /// Symlink each `<dotfiles_root>/<dir>/<include[i]>` to `<dst>/<include[i]>`.
    Expand {
        dir: String,
        #[serde(default)]
        include: Vec<String>,
    },
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

/// Resolved (src, dst) pair ready for link::ensure / inspect / remove.
#[derive(Debug)]
pub struct ResolvedLink {
    pub src: PathBuf,
    pub dst: PathBuf,
}

impl LinkSpec {
    /// Expand this link spec against the current OS.
    ///
    /// Returns `Ok(None)` when the link has no destination for this OS.
    /// `dst_expand` is injected so `os::expand` (which reads env vars) does not
    /// need to be re-imported here.
    pub fn resolve<F>(
        &self,
        dotfiles_root: &Path,
        os: Os,
        dst_expand: F,
    ) -> anyhow::Result<Option<Vec<ResolvedLink>>>
    where
        F: Fn(&str) -> anyhow::Result<PathBuf>,
    {
        let Some(dst_raw) = self.dst.pick(os) else {
            return Ok(None);
        };
        let dst_base = dst_expand(dst_raw)?;
        match &self.src {
            LinkSrc::Path(p) => Ok(Some(vec![ResolvedLink {
                src: dotfiles_root.join(p),
                dst: dst_base,
            }])),
            LinkSrc::Expand { dir, include } => {
                let src_dir = dotfiles_root.join(dir);
                let mut out = Vec::with_capacity(include.len());
                for name in include {
                    out.push(ResolvedLink {
                        src: src_dir.join(name),
                        dst: dst_base.join(name),
                    });
                }
                Ok(Some(out))
            }
        }
    }
}

/// Load `<root>/dotfm.toml`.
pub fn load(dotfiles_root: &Path) -> Result<Registry> {
    let path: PathBuf = dotfiles_root.join("dotfm.toml");
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
            dir.path().join("dotfm.toml"),
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
        match &t.links[0].src {
            LinkSrc::Path(p) => assert_eq!(p, "alacritty"),
            other => panic!("expected Path, got {other:?}"),
        }
        assert_eq!(
            t.links[0].dst.pick(Os::Linux),
            Some(&"$HOME/.config/alacritty".to_string())
        );
    }

    #[test]
    fn load_expand_src_form() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("dotfm.toml"),
            r#"
[tools.code]
[[tools.code.links]]
src = { dir = "code", include = ["settings.json", "keybindings.json"] }
dst.linux = "$XDG_CONFIG_HOME/Code/User"
"#,
        )
        .unwrap();

        let reg = load(dir.path()).unwrap();
        let link = &reg.tools.get("code").unwrap().links[0];
        match &link.src {
            LinkSrc::Expand { dir, include } => {
                assert_eq!(dir, "code");
                assert_eq!(include, &["settings.json", "keybindings.json"]);
            }
            other => panic!("expected Expand, got {other:?}"),
        }
    }

    #[test]
    fn load_registry_missing_errors() {
        let dir = tempdir().unwrap();
        let err = load(dir.path()).unwrap_err();
        assert!(err.to_string().contains("does not contain dotfm.toml"));
    }
}
