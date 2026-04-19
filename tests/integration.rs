//! End-to-end tests for `dotfm`.
//!
//! Smoke tests (`help`, `version`) run on every platform.
//! The full init→add→apply→remove flow runs only on Unix where symlink creation
//! does not require elevation. The Windows path is exercised via the manual
//! smoke test documented in PRD.md / Step 9 of the implementation plan.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_lists_subcommands() {
    Command::cargo_bin("dotfm")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("remove"))
        .stdout(predicate::str::contains("apply"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("diff"));
}

#[test]
fn version_is_printed() {
    Command::cargo_bin("dotfm")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.0.6"));
}

#[cfg(unix)]
mod unix_flow {
    use std::fs;
    use std::path::Path;

    use assert_cmd::Command;
    use predicates::prelude::*;
    use tempfile::TempDir;

    /// Set up a fake dotfiles repo in a tempdir with two tools registered.
    fn fixture() -> (TempDir, TempDir) {
        let dotfiles = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();

        let alacritty_dir = dotfiles.path().join("alacritty");
        fs::create_dir_all(&alacritty_dir).unwrap();
        fs::write(alacritty_dir.join("alacritty.toml"), "# alacritty\n").unwrap();

        let starship_src = dotfiles.path().join("starship/starship.toml");
        fs::create_dir_all(starship_src.parent().unwrap()).unwrap();
        fs::write(&starship_src, "# starship\n").unwrap();

        let registry = format!(
            r#"
[tools.alacritty]
description = "Alacritty"
[[tools.alacritty.links]]
src = "alacritty"
dst.linux   = "{home}/.config/alacritty"
dst.windows = "$APPDATA/alacritty"

[tools.starship]
description = "Starship prompt"
[[tools.starship.links]]
src = "starship/starship.toml"
dst.linux   = "{home}/.config/starship.toml"
dst.windows = "$APPDATA/starship.toml"
"#,
            home = home.path().display()
        );
        fs::write(dotfiles.path().join("dotfm.toml"), registry).unwrap();

        (dotfiles, home)
    }

    fn dotfm(home: &Path, config: &Path) -> Command {
        let mut c = Command::cargo_bin("dotfm").unwrap();
        c.env("HOME", home)
            .env("USERPROFILE", home)
            .env("DOTFM_CONFIG", config)
            .env_remove("XDG_CONFIG_HOME");
        c
    }

    #[test]
    fn init_add_apply_status_remove_flow() {
        let (dotfiles, home) = fixture();
        let config_path = home.path().join(".config/dotfm/config.toml");

        // init
        dotfm(home.path(), &config_path)
            .args(["init", "--dotfiles"])
            .arg(dotfiles.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("created"));
        assert!(config_path.is_file());

        // list shows both tools.
        dotfm(home.path(), &config_path)
            .arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("alacritty"))
            .stdout(predicate::str::contains("starship"));

        // add
        dotfm(home.path(), &config_path)
            .args(["add", "alacritty", "starship"])
            .assert()
            .success();

        // apply
        dotfm(home.path(), &config_path)
            .arg("apply")
            .assert()
            .success();
        assert!(home.path().join(".config/alacritty").is_symlink());
        assert!(home.path().join(".config/starship.toml").is_symlink());

        // apply again — idempotent.
        dotfm(home.path(), &config_path)
            .arg("apply")
            .assert()
            .success()
            .stdout(predicate::str::contains("ok"));

        // status should report ok.
        dotfm(home.path(), &config_path)
            .arg("status")
            .assert()
            .success()
            .stdout(predicate::str::contains("ok"));

        // remove one tool and its symlink should disappear.
        dotfm(home.path(), &config_path)
            .args(["remove", "starship"])
            .assert()
            .success();
        assert!(!home.path().join(".config/starship.toml").exists());
        // alacritty should still be linked.
        assert!(home.path().join(".config/alacritty").is_symlink());
    }

    #[test]
    fn dry_run_does_not_touch_filesystem() {
        let (dotfiles, home) = fixture();
        let config_path = home.path().join(".config/dotfm/config.toml");

        dotfm(home.path(), &config_path)
            .args(["init", "--dotfiles"])
            .arg(dotfiles.path())
            .assert()
            .success();

        dotfm(home.path(), &config_path)
            .args(["--dry-run", "add", "alacritty"])
            .assert()
            .success();
        // With --dry-run, config should remain empty.
        let cfg_text = fs::read_to_string(&config_path).unwrap();
        assert!(
            !cfg_text.contains("alacritty"),
            "config should not persist enable in dry-run mode: {cfg_text}"
        );
    }

    #[test]
    fn add_unknown_tool_errors() {
        let (dotfiles, home) = fixture();
        let config_path = home.path().join(".config/dotfm/config.toml");

        dotfm(home.path(), &config_path)
            .args(["init", "--dotfiles"])
            .arg(dotfiles.path())
            .assert()
            .success();

        dotfm(home.path(), &config_path)
            .args(["add", "nope"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("unknown tool"));
    }

    /// post_apply hooks fire on `apply` and skip under `--dry-run`.
    #[test]
    fn post_apply_hook_runs_and_respects_dry_run() {
        let dotfiles = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let config_path = home.path().join(".config/dotfm/config.toml");

        let src = dotfiles.path().join("marker/marker.txt");
        fs::create_dir_all(src.parent().unwrap()).unwrap();
        fs::write(&src, "m").unwrap();
        let marker = home.path().join("marker_touched");

        let registry = format!(
            r#"
[tools.marker]
[[tools.marker.links]]
src = "marker/marker.txt"
dst.linux   = "{home}/.config/marker.txt"
dst.windows = "$APPDATA/marker.txt"

[[tools.marker.post_apply]]
run = ["sh", "-c", "touch {marker}"]
"#,
            home = home.path().display(),
            marker = marker.display()
        );
        fs::write(dotfiles.path().join("dotfm.toml"), registry).unwrap();

        dotfm(home.path(), &config_path)
            .args(["init", "--dotfiles"])
            .arg(dotfiles.path())
            .assert()
            .success();
        dotfm(home.path(), &config_path)
            .args(["add", "marker"])
            .assert()
            .success();

        // dry-run: shows the intent but does not touch the marker file
        dotfm(home.path(), &config_path)
            .args(["--dry-run", "apply"])
            .assert()
            .success()
            .stdout(predicate::str::contains("would run"));
        assert!(!marker.exists(), "post_apply must not run under --dry-run");

        // real apply: hook runs
        dotfm(home.path(), &config_path)
            .arg("apply")
            .assert()
            .success();
        assert!(marker.exists(), "post_apply did not create marker file");
    }

    /// A failing post_apply hook makes `apply` exit non-zero and stops that tool.
    #[test]
    fn failing_post_apply_hook_surfaces_error() {
        let dotfiles = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let config_path = home.path().join(".config/dotfm/config.toml");

        let src = dotfiles.path().join("a/file.txt");
        fs::create_dir_all(src.parent().unwrap()).unwrap();
        fs::write(&src, "x").unwrap();

        let registry = format!(
            r#"
[tools.a]
[[tools.a.links]]
src = "a/file.txt"
dst.linux   = "{home}/.config/a.txt"
dst.windows = "$APPDATA/a.txt"

[[tools.a.post_apply]]
run = ["sh", "-c", "exit 3"]
"#,
            home = home.path().display()
        );
        fs::write(dotfiles.path().join("dotfm.toml"), registry).unwrap();

        dotfm(home.path(), &config_path)
            .args(["init", "--dotfiles"])
            .arg(dotfiles.path())
            .assert()
            .success();
        dotfm(home.path(), &config_path)
            .args(["add", "a"])
            .assert()
            .success();

        dotfm(home.path(), &config_path)
            .arg("apply")
            .assert()
            .failure()
            .stderr(predicate::str::contains("hook failed"));
        // The link itself should still be created — ensure the tool did progress.
        assert!(home.path().join(".config/a.txt").is_symlink());
    }

    /// diff shows registry drift (available-but-disabled + orphan-enabled) and
    /// link drift (missing vs ok) with the expected exit codes.
    #[test]
    fn diff_reports_each_layer() {
        let (dotfiles, home) = fixture();
        let config_path = home.path().join(".config/dotfm/config.toml");

        dotfm(home.path(), &config_path)
            .args(["init", "--dotfiles"])
            .arg(dotfiles.path())
            .assert()
            .success();

        // Before add: both tools are available-but-disabled → drift on layer 1,
        // no link drift to report.
        dotfm(home.path(), &config_path)
            .arg("diff")
            .assert()
            .failure()
            .stdout(predicate::str::contains("available, not enabled"));

        dotfm(home.path(), &config_path)
            .args(["add", "alacritty"])
            .assert()
            .success();

        // After add but before apply: link drift.
        dotfm(home.path(), &config_path)
            .arg("diff")
            .assert()
            .failure()
            .stdout(predicate::str::contains("alacritty"));

        dotfm(home.path(), &config_path)
            .arg("apply")
            .assert()
            .success();

        // After apply: alacritty link is ok; starship still "available, not enabled".
        // Overall exit is still non-zero because of layer 1 drift.
        dotfm(home.path(), &config_path)
            .arg("diff")
            .assert()
            .failure()
            .stdout(predicate::str::contains("all enabled links in sync"));
    }

    /// `--content` prints a unified diff for a destination that is a plain file
    /// (not a symlink) and whose content differs from the source.
    #[test]
    fn diff_content_flag_shows_unified_diff() {
        let dotfiles = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let config_path = home.path().join(".config/dotfm/config.toml");

        let src = dotfiles.path().join("t/file.txt");
        fs::create_dir_all(src.parent().unwrap()).unwrap();
        fs::write(&src, "hello\nworld\n").unwrap();

        let dst = home.path().join(".config/t.txt");
        fs::create_dir_all(dst.parent().unwrap()).unwrap();
        fs::write(&dst, "hello\nthere\n").unwrap();

        let registry = format!(
            r#"
[tools.t]
[[tools.t.links]]
src = "t/file.txt"
dst.linux   = "{home}/.config/t.txt"
dst.windows = "$APPDATA/t.txt"
"#,
            home = home.path().display()
        );
        fs::write(dotfiles.path().join("dotfm.toml"), registry).unwrap();

        dotfm(home.path(), &config_path)
            .args(["init", "--dotfiles"])
            .arg(dotfiles.path())
            .assert()
            .success();
        dotfm(home.path(), &config_path)
            .args(["add", "t"])
            .assert()
            .success();

        dotfm(home.path(), &config_path)
            .args(["diff", "--content"])
            .assert()
            .failure()
            .stdout(predicate::str::contains("-world"))
            .stdout(predicate::str::contains("+there"));
    }

    /// `os = ["linux"]` filter on a hook is honored.
    #[test]
    fn post_apply_os_filter_is_respected() {
        let dotfiles = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let config_path = home.path().join(".config/dotfm/config.toml");

        let src = dotfiles.path().join("o/file.txt");
        fs::create_dir_all(src.parent().unwrap()).unwrap();
        fs::write(&src, "x").unwrap();

        let marker_linux = home.path().join("linux_touched");
        let marker_win = home.path().join("windows_touched");

        let registry = format!(
            r#"
[tools.o]
[[tools.o.links]]
src = "o/file.txt"
dst.linux   = "{home}/.config/o.txt"
dst.windows = "$APPDATA/o.txt"

[[tools.o.post_apply]]
run = ["sh", "-c", "touch {linux_marker}"]
os = ["linux"]

[[tools.o.post_apply]]
run = ["sh", "-c", "touch {win_marker}"]
os = ["windows"]
"#,
            home = home.path().display(),
            linux_marker = marker_linux.display(),
            win_marker = marker_win.display()
        );
        fs::write(dotfiles.path().join("dotfm.toml"), registry).unwrap();

        dotfm(home.path(), &config_path)
            .args(["init", "--dotfiles"])
            .arg(dotfiles.path())
            .assert()
            .success();
        dotfm(home.path(), &config_path)
            .args(["add", "o"])
            .assert()
            .success();
        dotfm(home.path(), &config_path)
            .arg("apply")
            .assert()
            .success();

        // On unix-run tests Os::current() == Linux, so only the linux hook fires.
        assert!(marker_linux.exists());
        assert!(!marker_win.exists());
    }
}
