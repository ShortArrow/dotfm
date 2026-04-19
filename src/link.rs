use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkState {
    Missing,
    CorrectLink,
    WrongLink { target: PathBuf },
    ExistingFile,
    ExistingDir,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Noop,
    Created,
    Updated,
    BackedUpAndReplaced,
    Removed,
    Skipped { reason: String },
}

/// Inspect the destination path relative to the expected source.
pub fn inspect(src: &Path, dst: &Path) -> std::io::Result<LinkState> {
    match fs::symlink_metadata(dst) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(LinkState::Missing),
        Err(e) => Err(e),
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                let actual = fs::read_link(dst)?;
                if paths_equal(&actual, src) {
                    Ok(LinkState::CorrectLink)
                } else {
                    Ok(LinkState::WrongLink { target: actual })
                }
            } else if meta.is_dir() {
                Ok(LinkState::ExistingDir)
            } else {
                Ok(LinkState::ExistingFile)
            }
        }
    }
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    // symlinks may store relative paths; canonicalize both when possible.
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

/// Ensure `dst` is a symlink pointing at `src`. Idempotent.
pub fn ensure(src: &Path, dst: &Path, force: bool, dry_run: bool) -> Result<Change> {
    let state = inspect(src, dst).with_context(|| format!("inspecting {}", dst.display()))?;
    match state {
        LinkState::CorrectLink => Ok(Change::Noop),
        LinkState::Missing => {
            if !dry_run {
                create_parent(dst)?;
                create_symlink(src, dst)?;
            }
            Ok(Change::Created)
        }
        LinkState::WrongLink { .. } => {
            if !dry_run {
                fs::remove_file(dst)
                    .with_context(|| format!("removing stale symlink {}", dst.display()))?;
                create_parent(dst)?;
                create_symlink(src, dst)?;
            }
            Ok(Change::Updated)
        }
        LinkState::ExistingFile | LinkState::ExistingDir => {
            if !force {
                return Err(Error::DestinationOccupied {
                    path: dst.to_path_buf(),
                }
                .into());
            }
            if !dry_run {
                let backup = backup_path(dst);
                fs::rename(dst, &backup).with_context(|| {
                    format!("backing up {} to {}", dst.display(), backup.display())
                })?;
                create_parent(dst)?;
                create_symlink(src, dst)?;
            }
            Ok(Change::BackedUpAndReplaced)
        }
    }
}

/// Remove `dst` only if it is a symlink whose target is inside `dotfiles_root`.
pub fn remove_if_ours(
    _src: &Path,
    dst: &Path,
    dotfiles_root: &Path,
    dry_run: bool,
) -> Result<Change> {
    let meta = match fs::symlink_metadata(dst) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Change::Skipped {
                reason: format!("{} does not exist", dst.display()),
            });
        }
        other => other.with_context(|| format!("stat {}", dst.display()))?,
    };
    if !meta.file_type().is_symlink() {
        return Ok(Change::Skipped {
            reason: format!("{} is not a symlink", dst.display()),
        });
    }

    let target = fs::read_link(dst).with_context(|| format!("reading link {}", dst.display()))?;
    let canonical_target = fs::canonicalize(&target).unwrap_or(target);
    let canonical_root =
        fs::canonicalize(dotfiles_root).unwrap_or_else(|_| dotfiles_root.to_path_buf());

    if !canonical_target.starts_with(&canonical_root) {
        return Ok(Change::Skipped {
            reason: format!(
                "{} points outside dotfiles_root ({})",
                dst.display(),
                canonical_target.display()
            ),
        });
    }

    if !dry_run {
        fs::remove_file(dst).with_context(|| format!("removing {}", dst.display()))?;
    }
    Ok(Change::Removed)
}

fn backup_path(dst: &Path) -> PathBuf {
    let mut p = dst.as_os_str().to_os_string();
    p.push(".bak");
    PathBuf::from(p)
}

fn create_parent(dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating parent {}", parent.display()))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn create_symlink(src: &Path, dst: &Path) -> Result<()> {
    std::os::unix::fs::symlink(src, dst)
        .with_context(|| format!("creating symlink {} -> {}", dst.display(), src.display()))
}

#[cfg(windows)]
fn create_symlink(src: &Path, dst: &Path) -> Result<()> {
    let is_dir = src.is_dir();
    let result = if is_dir {
        std::os::windows::fs::symlink_dir(src, dst)
    } else {
        std::os::windows::fs::symlink_file(src, dst)
    };
    result.map_err(|e| {
        let raw = e.raw_os_error();
        // ERROR_PRIVILEGE_NOT_HELD = 1314
        if raw == Some(1314) {
            anyhow::anyhow!(
                "creating symlink {} -> {}: Windows requires Developer Mode or an elevated (Admin) shell to create symlinks. \
                 Enable Developer Mode in Settings, or run dotfm from an elevated PowerShell.",
                dst.display(),
                src.display()
            )
        } else {
            anyhow::Error::from(e)
                .context(format!("creating symlink {} -> {}", dst.display(), src.display()))
        }
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn inspect_states() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::write(&src, "x").unwrap();

        let missing = dir.path().join("missing");
        assert_eq!(inspect(&src, &missing).unwrap(), LinkState::Missing);

        let correct = dir.path().join("correct");
        std::os::unix::fs::symlink(&src, &correct).unwrap();
        assert_eq!(inspect(&src, &correct).unwrap(), LinkState::CorrectLink);

        let other_src = dir.path().join("other");
        std::fs::write(&other_src, "y").unwrap();
        let wrong = dir.path().join("wrong");
        std::os::unix::fs::symlink(&other_src, &wrong).unwrap();
        assert!(matches!(
            inspect(&src, &wrong).unwrap(),
            LinkState::WrongLink { .. }
        ));

        let file = dir.path().join("file");
        std::fs::write(&file, "z").unwrap();
        assert_eq!(inspect(&src, &file).unwrap(), LinkState::ExistingFile);

        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        assert_eq!(inspect(&src, &subdir).unwrap(), LinkState::ExistingDir);
    }

    #[test]
    fn ensure_creates_and_is_idempotent() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::write(&src, "x").unwrap();
        let dst = dir.path().join("child/dst");

        let ch = ensure(&src, &dst, false, false).unwrap();
        assert_eq!(ch, Change::Created);

        let ch = ensure(&src, &dst, false, false).unwrap();
        assert_eq!(ch, Change::Noop);
    }

    #[test]
    fn ensure_force_backs_up() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::write(&src, "x").unwrap();
        let dst = dir.path().join("dst");
        std::fs::write(&dst, "existing").unwrap();

        let err = ensure(&src, &dst, false, false).unwrap_err();
        assert!(err.to_string().contains("exists"));

        let ch = ensure(&src, &dst, true, false).unwrap();
        assert_eq!(ch, Change::BackedUpAndReplaced);
        assert!(dir.path().join("dst.bak").exists());
    }

    #[test]
    fn remove_skips_foreign_link() {
        let dir = tempdir().unwrap();
        let foreign = dir.path().join("foreign");
        std::fs::write(&foreign, "f").unwrap();
        let dst = dir.path().join("dst");
        std::os::unix::fs::symlink(&foreign, &dst).unwrap();
        let dotfiles_root = dir.path().join("dotfiles");
        std::fs::create_dir(&dotfiles_root).unwrap();

        let ch = remove_if_ours(&foreign, &dst, &dotfiles_root, false).unwrap();
        assert!(matches!(ch, Change::Skipped { .. }));
        assert!(dst.exists());
    }

    #[test]
    fn remove_deletes_our_link() {
        let dir = tempdir().unwrap();
        let dotfiles_root = dir.path().join("dotfiles");
        std::fs::create_dir(&dotfiles_root).unwrap();
        let src = dotfiles_root.join("src");
        std::fs::write(&src, "x").unwrap();
        let dst = dir.path().join("dst");
        std::os::unix::fs::symlink(&src, &dst).unwrap();

        let ch = remove_if_ours(&src, &dst, &dotfiles_root, false).unwrap();
        assert_eq!(ch, Change::Removed);
        assert!(!dst.exists());
    }
}
