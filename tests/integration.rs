//! End-to-end tests for `dotup`.
//!
//! Smoke tests (`help`, `version`) run on every platform.
//! The full init→add→apply→remove flow runs only on Unix where symlink creation
//! does not require elevation. The Windows path is exercised via the manual
//! smoke test documented in PRD.md / Step 9 of the implementation plan.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_lists_subcommands() {
    Command::cargo_bin("dotup")
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
        .stdout(predicate::str::contains("doctor"));
}

#[test]
fn version_is_printed() {
    Command::cargo_bin("dotup")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.0.3"));
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
        fs::write(dotfiles.path().join("dotup.toml"), registry).unwrap();

        (dotfiles, home)
    }

    fn dotup(home: &Path, config: &Path) -> Command {
        let mut c = Command::cargo_bin("dotup").unwrap();
        c.env("HOME", home)
            .env("USERPROFILE", home)
            .env("DOTUP_CONFIG", config)
            .env_remove("XDG_CONFIG_HOME");
        c
    }

    #[test]
    fn init_add_apply_status_remove_flow() {
        let (dotfiles, home) = fixture();
        let config_path = home.path().join(".config/dotup/config.toml");

        // init
        dotup(home.path(), &config_path)
            .args(["init", "--dotfiles"])
            .arg(dotfiles.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("created"));
        assert!(config_path.is_file());

        // list shows both tools.
        dotup(home.path(), &config_path)
            .arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("alacritty"))
            .stdout(predicate::str::contains("starship"));

        // add
        dotup(home.path(), &config_path)
            .args(["add", "alacritty", "starship"])
            .assert()
            .success();

        // apply
        dotup(home.path(), &config_path)
            .arg("apply")
            .assert()
            .success();
        assert!(home.path().join(".config/alacritty").is_symlink());
        assert!(home.path().join(".config/starship.toml").is_symlink());

        // apply again — idempotent.
        dotup(home.path(), &config_path)
            .arg("apply")
            .assert()
            .success()
            .stdout(predicate::str::contains("ok"));

        // status should report ok.
        dotup(home.path(), &config_path)
            .arg("status")
            .assert()
            .success()
            .stdout(predicate::str::contains("ok"));

        // remove one tool and its symlink should disappear.
        dotup(home.path(), &config_path)
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
        let config_path = home.path().join(".config/dotup/config.toml");

        dotup(home.path(), &config_path)
            .args(["init", "--dotfiles"])
            .arg(dotfiles.path())
            .assert()
            .success();

        dotup(home.path(), &config_path)
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
        let config_path = home.path().join(".config/dotup/config.toml");

        dotup(home.path(), &config_path)
            .args(["init", "--dotfiles"])
            .arg(dotfiles.path())
            .assert()
            .success();

        dotup(home.path(), &config_path)
            .args(["add", "nope"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("unknown tool"));
    }
}
