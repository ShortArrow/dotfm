# Changelog

All notable changes to dotfm are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

While the project is in `0.0.x`, minor and patch releases may both introduce
breaking changes to the TOML schema or CLI surface; once a `0.1.0` is cut,
regular semver rules apply.

## [Unreleased]

Nothing yet.

## [0.0.6] – 2026-04-19

### Added
- `dotfm diff [tool...]` command, showing three drift layers in order:
  registry vs. enabled list, expected vs. actual symlink state, and
  (with `--content`) a unified diff when a destination is a plain file
  whose content differs from the source. Binary files are detected via
  a NUL-byte heuristic and skipped in content mode.
- `similar` dependency for unified-diff generation.
- Integration tests covering each layer of `diff` and the `--content` flag.

### Changed
- Renamed crate, binary, and config location from **dotup** to **dotfm**:
  `dotup` was already taken on crates.io. Along with the rename:
  - `~/.config/dotup/config.toml` → `~/.config/dotfm/config.toml`
  - `dotup.toml` registry filename → `dotfm.toml`
  - `DOTUP_CONFIG` test env var → `DOTFM_CONFIG`
- GitHub repository renamed `ShortArrow/dotup` → `ShortArrow/dotfm`.

### Infrastructure
- `Cargo.toml`: full `authors`, `homepage`, `documentation`, and `exclude`
  so `cargo publish --dry-run` passes cleanly.
- CI workflow: `fmt --check` + `clippy -D warnings` on Linux, plus
  `cargo test` on Linux / macOS / Windows.
- Release workflow: `v*` tag builds four targets (Linux x86_64, Windows
  MSVC, macOS Intel, macOS Apple Silicon) with SHA256SUMS and
  auto-generated notes. Termux (aarch64-linux-android) is intentionally
  not prebuilt — `cargo install dotfm` from source works on Termux.

## [0.0.5] – 2026-04-19

### Changed
- `dotfm doctor` now runs **generic environment checks only** by default;
  per-tool `doctor` scripts used to always run, which made the command
  slow (for example, a full Windows environment scan is ~3 s).
  - Pass explicit tool names to run those doctors: `dotfm doctor glazewm`.
  - Pass `--all` to run every enabled tool's doctor.
  - `--no-generic` remains available to skip the environment checks.

## [0.0.4] – 2026-04-19

### Added
- Polymorphic `src` in `dotfm.toml`. `src = "<path>"` keeps its prior
  meaning (link one file or directory). The new table form
  `src = { dir = "<dir>", include = ["a", "b"] }` links each named file
  inside `<dir>` individually under the destination directory, keeping
  its filename. Useful when the destination (e.g. VS Code's `User/`)
  also contains files you don't want to manage.
- Unit test covering the expand form.

### Changed
- Link-resolution logic now flows through `LinkSpec::resolve`, which all
  of `apply` / `remove` / `status` / `doctor` use.

## [0.0.3] – 2026-04-19

### Added
- `dotfm doctor [tool...]` command:
  - **Generic checks**: `HOME` / `USERPROFILE` / `XDG_CONFIG_HOME`
    resolve, `dotfiles_root` exists, every enabled tool's links are
    `CorrectLink`, and (on Windows) Developer Mode is enabled.
  - **Tool-specific** delegation: `[tools.<name>.doctor]` in `dotfm.toml`
    points at a script run under `pwsh` on Windows and `bash` on Linux.
- Exit codes: `0` healthy, `1` generic-check failure, `2` doctor script
  failure.
- `--no-generic` flag to skip environment checks.

## [0.0.2] – 2026-04-19

### Added
- Nerd Font output mode.
  - `NERD_FONT=1` environment variable switches `--icons=auto` (default)
    to Nerd Font glyphs.
  - `--icons nerd` / `--icons plain` override the env var.
  - A terminal cannot advertise which font it is rendering, so
    auto-detection is intentionally absent — selection is strictly opt-in.
- `src/style.rs` with `Icons` struct plumbed through every command.

## [0.0.1] – 2026-04-19

Initial MVP.

### Added
- `dotfm init [--dotfiles <path>]` to create `~/.config/dotfm/config.toml`.
- `dotfm add <tool>...` / `dotfm remove <tool>...` to manage the per-host
  enabled list; format-preserving via `toml_edit`.
- `dotfm apply [tool...]` with `--force` (back up conflicts to `.bak`)
  and `--dry-run`.
- `dotfm status` and `dotfm list`.
- `dotfm.toml` registry schema:
  - `[tools.<name>]` with optional `description`, `platforms`,
    `[[links]]`, `[[post_apply]]`, `script`, `unscript`.
  - Environment variable expansion in `dst`: `$HOME`, `$USERPROFILE`,
    `$APPDATA`, `$LOCALAPPDATA`, `$XDG_CONFIG_HOME` (fallback
    `$HOME/.config`), `~`.
- Windows symlink creation surfaces `ERROR_PRIVILEGE_NOT_HELD` (1314)
  with a Developer Mode / Admin hint.
- Script delegation: `[tools.<name>.script]` runs legacy
  `setup.ps1` / `setup.sh` via `pwsh` or `bash`.
- `DOTFM_CONFIG` environment variable to override the config path
  (used in tests).

Pre-0.0.6 releases were not tagged individually during the `dotup → dotfm`
transition; the single `v0.0.6` tag collects everything that shipped that day.
Future releases will have their own tags.

[Unreleased]: https://github.com/ShortArrow/dotfm/compare/v0.0.6...HEAD
[0.0.6]: https://github.com/ShortArrow/dotfm/releases/tag/v0.0.6
