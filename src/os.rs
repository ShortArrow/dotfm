use std::path::PathBuf;

use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Os {
    Windows,
    Linux,
}

impl Os {
    pub fn current() -> Os {
        if cfg!(windows) {
            Os::Windows
        } else {
            Os::Linux
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Os::Windows => "windows",
            Os::Linux => "linux",
        }
    }
}

/// Expand `$VAR`, `${VAR}`, and leading `~` in a path string.
///
/// In addition to the environment, falls back to sensible defaults:
/// - `$XDG_CONFIG_HOME` -> `$HOME/.config` (or `$USERPROFILE/.config` on Windows) if unset
/// - `~` -> `$HOME` (or `$USERPROFILE` on Windows)
pub fn expand(path: &str) -> Result<PathBuf> {
    let mut out = shellexpand::env_with_context(path, |var| -> Result<Option<String>, String> {
        // First try the real environment.
        if let Ok(v) = std::env::var(var) {
            return Ok(Some(v));
        }
        // Fallbacks for variables that are commonly missing.
        match var {
            "XDG_CONFIG_HOME" => {
                let home = home_dir_string().ok_or_else(|| "no HOME/USERPROFILE".to_string())?;
                let mut p = PathBuf::from(home);
                p.push(".config");
                Ok(Some(p.to_string_lossy().into_owned()))
            }
            "HOME" => Ok(home_dir_string()),
            "USERPROFILE" => Ok(home_dir_string()),
            _ => Ok(None),
        }
    })
    .map_err(|e| anyhow::anyhow!("failed to expand variables in {path}: {e}"))?
    .into_owned();

    if out.starts_with('~') {
        let home = home_dir_string()
            .ok_or_else(|| anyhow::anyhow!("cannot expand ~ without HOME/USERPROFILE"))?;
        out.replace_range(..1, &home);
    }

    Ok(PathBuf::from(out))
}

fn home_dir_string() -> Option<String> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_home_tilde() {
        // SAFETY: single-threaded test; no other test touches HOME here.
        unsafe {
            std::env::set_var("HOME", "/home/me");
        }
        let p = expand("~/foo").unwrap();
        assert!(p.to_string_lossy().starts_with("/home/me"));
    }

    #[test]
    fn expand_env_var() {
        unsafe {
            std::env::set_var("DOTUP_TEST_VAR", "/tmp/x");
        }
        let p = expand("$DOTUP_TEST_VAR/y").unwrap();
        assert_eq!(p, PathBuf::from("/tmp/x/y"));
    }

    #[test]
    fn expand_xdg_fallback() {
        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
            std::env::set_var("HOME", "/home/me");
        }
        let p = expand("$XDG_CONFIG_HOME/app").unwrap();
        assert_eq!(p, PathBuf::from("/home/me/.config/app"));
    }
}
