use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use toml_edit::{Array, DocumentMut, Item, Value, value};

use crate::error::Error;
use crate::os;

/// In-memory representation of `~/.config/dotfm/config.toml` with format preservation.
#[derive(Debug)]
pub struct Config {
    pub path: PathBuf,
    doc: DocumentMut,
}

impl Config {
    /// Default location: `$XDG_CONFIG_HOME/dotfm/config.toml`,
    /// falling back to `$HOME/.config/dotfm/config.toml`
    /// (or `$USERPROFILE/.config/dotfm/config.toml` on Windows when `HOME` is unset).
    ///
    /// Can be overridden for testing with the `DOTFM_CONFIG` environment variable.
    pub fn default_path() -> Result<PathBuf> {
        if let Ok(p) = std::env::var("DOTFM_CONFIG") {
            return Ok(PathBuf::from(p));
        }
        os::expand("$XDG_CONFIG_HOME/dotfm/config.toml")
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.is_file() {
            return Err(Error::ConfigMissing {
                path: path.to_path_buf(),
            }
            .into());
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let doc: DocumentMut = text
            .parse()
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            doc,
        })
    }

    pub fn create(path: &Path, dotfiles_root: &Path, force: bool) -> Result<Self> {
        if path.exists() && !force {
            return Err(Error::ConfigExists {
                path: path.to_path_buf(),
            }
            .into());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let mut doc = DocumentMut::new();
        doc["dotfiles_root"] = value(dotfiles_root.to_string_lossy().into_owned());
        doc["enabled"] = Item::Value(Value::Array(Array::new()));
        std::fs::write(path, doc.to_string())
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            doc,
        })
    }

    pub fn save(&self) -> Result<()> {
        std::fs::write(&self.path, self.doc.to_string())
            .with_context(|| format!("writing {}", self.path.display()))?;
        Ok(())
    }

    pub fn dotfiles_root(&self) -> Option<PathBuf> {
        let s = self.doc.get("dotfiles_root")?.as_str()?;
        Some(PathBuf::from(s))
    }

    pub fn set_dotfiles_root(&mut self, path: &Path) {
        self.doc["dotfiles_root"] = value(path.to_string_lossy().into_owned());
    }

    pub fn enabled(&self) -> Vec<String> {
        let Some(array) = self.doc.get("enabled").and_then(|i| i.as_array()) else {
            return vec![];
        };
        array
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    }

    /// Returns true if the tool was newly added, false if already present.
    pub fn enable(&mut self, tool: &str) -> bool {
        let array = self.ensure_enabled_array();
        if array_contains_str(array, tool) {
            return false;
        }
        array.push(tool);
        sort_array(array);
        true
    }

    /// Returns true if the tool was removed, false if not present.
    pub fn disable(&mut self, tool: &str) -> bool {
        let array = self.ensure_enabled_array();
        let mut removed = false;
        let mut i = 0;
        while i < array.len() {
            if array.get(i).and_then(|v| v.as_str()) == Some(tool) {
                array.remove(i);
                removed = true;
            } else {
                i += 1;
            }
        }
        removed
    }

    fn ensure_enabled_array(&mut self) -> &mut Array {
        if !self.doc.contains_key("enabled") {
            self.doc["enabled"] = Item::Value(Value::Array(Array::new()));
        }
        self.doc["enabled"]
            .as_array_mut()
            .expect("enabled is array")
    }
}

fn array_contains_str(array: &Array, needle: &str) -> bool {
    array.iter().any(|v| v.as_str() == Some(needle))
}

fn sort_array(array: &mut Array) {
    let mut items: Vec<String> = array
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    items.sort();
    array.clear();
    for item in items {
        array.push(item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn create_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let root = dir.path();
        let cfg = Config::create(&path, root, false).unwrap();
        assert_eq!(cfg.enabled(), Vec::<String>::new());
        assert_eq!(cfg.dotfiles_root().unwrap(), root);

        let cfg2 = Config::load(&path).unwrap();
        assert_eq!(cfg2.dotfiles_root().unwrap(), root);
    }

    #[test]
    fn enable_disable_preserves_format() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "# leading comment\ndotfiles_root = \"/tmp/d\"\nenabled = [\n    \"a\",\n    \"b\",\n]\n",
        )
        .unwrap();

        let mut cfg = Config::load(&path).unwrap();
        assert!(cfg.enable("c"));
        assert!(!cfg.enable("a"));
        assert!(cfg.disable("b"));
        assert!(!cfg.disable("missing"));
        cfg.save().unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# leading comment"));
        assert_eq!(cfg.enabled(), vec!["a".to_string(), "c".to_string()]);
    }
}
