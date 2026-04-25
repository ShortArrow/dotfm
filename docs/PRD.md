# dotfm — Product Requirements Document

**Status:** Living document. Tracks the implementation as of v0.0.6.
**Current release:** 0.0.6 (pre-alpha). See [CHANGELOG.md](../CHANGELOG.md) for the
per-version history. Sections marked **(planned)** are not yet implemented.

---

## 1. Overview

`dotfm` is a command-line tool that lets a user declare, per machine, which tools
from a shared dotfiles repository are active on that machine, and materializes
them as symbolic links. It replaces hand-written `setup.sh` / `setup.ps1` scripts
scattered across a dotfiles repo with a single declarative TOML registry plus a
small stateful CLI.

## 2. Problem statement

A typical power-user dotfiles repository contains per-tool directories
(`alacritty/`, `nvim/`, `git/`, ...) and a `setup.sh` or `setup.ps1` in each. In
the motivating repo this amounts to **40 shell scripts** (19 PowerShell, 21 Bash)
with substantial duplication:

- Each script re-implements "remove target if it exists, create a symlink from
  dotfiles".
- Windows and Linux need separate scripts because of path and command differences.
- There is no mechanism to say "on this laptop I only want alacritty and git;
  on that desktop I want everything".
- Adding a new tool means writing two more scripts.

Existing managers do not solve all of the above simultaneously:

- **chezmoi** defaults to file copying rather than symlinking; enforces
  filename prefixes (`dot_`, `private_`, `executable_`, `run_`); models at the
  file level rather than the tool level; per-machine differences are expressed
  as Go templates rather than a declarative tool list.
- **GNU Stow** is symlink-first but Unix-only, has no per-host tool list, and
  does not cross platforms.
- **dotbot** is YAML-driven and cross-platform, but does not offer CLI-driven
  enable/disable of tools per machine; users split YAML files themselves.

`dotfm` targets the intersection these tools miss.

## 3. Goals

- **One registry.** Replace scattered `setup.sh` / `setup.ps1` files with one
  `dotfm.toml` registry.
- **Per-machine tool list.** Different machines check out the same repo and
  maintain independent enabled sets.
- **Symlinks by default.** Editing a file in the dotfiles repo takes effect
  immediately without running `apply`.
- **Tool-unit mental model.** `add`, `remove`, `apply` operate on tools, not
  on individual files.
- **Cross-platform.** Windows, Linux (WSL treated as Linux), and macOS are
  exercised in CI. Single Rust binary.
- **Idempotent `apply`.** Running it twice produces the same result.
  `--dry-run` previews changes without touching the filesystem.
- **Graceful degradation.** Tools with exotic setup logic (curl-downloaded
  installers, root-owned files) can still be delegated to an existing shell
  script from the registry entry.

## 4. Non-goals

- **No templating.** Go templates, handlebars, etc. If users need conditional
  file contents, they should split into separate files and enable the right
  tools per machine.
- **No encryption / secret management.** No password-manager integration.
- **No cloud sync.** No hosted state.
- **No management of unregistered files.** If it is not in `dotfm.toml`,
  `dotfm` does not touch it.
- **No package management.** `dotfm` configures; it does not install software.

## 5. Target users

- Developers who maintain one dotfiles repo across multiple machines (desktop,
  laptop, work laptop, WSL, home server, Termux).
- Users comfortable with a single declarative config file and a CLI — not a GUI.
- Users who use **different subsets of tools** on different machines and
  currently resolve this with per-machine branches, comments, or manual
  intervention.
- Users who prefer **symlink-based** dotfiles so the repo stays the single
  source of truth and edits require no apply step.

## 6. User stories

- Setting up a new machine: run
  `git clone <dotfiles> && dotfm init && dotfm add <tools> && dotfm apply`
  and have all config in place.
- On a minimal machine: enable only a subset of tools without editing shared
  repo files or branching.
- Add a new tool by adding one entry to `dotfm.toml` — no new shell script.
- `dotfm apply` is safe to run any number of times without producing duplicate
  entries, broken links, or surprise overwrites.
- Stop using a tool on this machine: `dotfm remove <tool>` strips the symlinks
  and updates the enabled list atomically.
- Keep awkward legacy setup (e.g. tmux theme downloader) as a plain `setup.sh`
  and have `dotfm apply` invoke it.
- Preview what `dotfm apply` would change before committing — including the
  unified diff between the source and a destination that is currently a plain
  file (`dotfm diff --content`).
- On Windows, symlink-permission errors are surfaced clearly (Developer Mode
  or Admin required), not silently swallowed.
- Run a quick health check before reporting "dotfiles broken" to a coworker:
  `dotfm doctor` for environment, `dotfm doctor <tool>` for a tool's own
  doctor script.

## 7. Functional requirements

### 7.1 `dotfm init`

- Creates `~/.config/dotfm/config.toml` if absent. Fails politely if present
  (suggest `--force`).
- Records `dotfiles_root` from `--dotfiles <path>` flag or from the current
  working directory if it looks like a dotfiles repo (contains `dotfm.toml`).
- Writes a starter file with empty `enabled = []`.

### 7.2 `dotfm add <tool>...`

- Validates each tool exists in `dotfm.toml`. If unknown, exits non-zero and
  prints available tools.
- Adds tools to `enabled` in `config.toml`, preserving TOML formatting and
  existing comments (uses `toml_edit`).
- Does **not** apply automatically. Prints a hint:
  `run 'dotfm apply' to create symlinks`.
- An `--apply` flag runs `apply` immediately after.

### 7.3 `dotfm remove <tool>...`

- For each tool: finds its declared links in `dotfm.toml`, removes each
  symlink if and only if it still points inside `dotfiles_root` (safety: never
  delete a link the user has retargeted).
- Removes the tool from `enabled` in `config.toml`.
- If a tool is listed as `script = ...` (legacy delegation) with no
  corresponding `unscript`, prints a warning that manual cleanup may be
  required but still updates `enabled`.

### 7.4 `dotfm apply [tool...]`

- Default: apply all tools in `enabled`.
- Argument form: apply only the specified tools (each must appear in `enabled`).
- For each tool, for each declared link:
  1. Inspect current state of the destination (missing / correct link / wrong
     link / existing file / existing directory).
  2. If `correct link`, do nothing.
  3. If `wrong link` or `missing`, create the link.
  4. If `existing file` or `existing directory`, abort that link with a clear
     error unless `--force` is given, in which case back up with a `.bak`
     suffix and replace.
- Then run any `post_apply` hooks (array of command tokens) for that tool.
- For tools declared with `script` only, run the platform-appropriate script
  (`pwsh` on Windows, `bash` on Linux).
- Continues to the next tool on failure, accumulates results, exits non-zero
  if any tool failed.
- Supports `--dry-run` and `--verbose`.

### 7.5 `dotfm status`

- Lists each enabled tool with a per-link state badge (`ok` / `missing` /
  `wrong` / `conflict`).
- Exits zero only if all links are `ok`.

### 7.6 `dotfm list`

- Prints all tools from `dotfm.toml` plus a marker for those in `enabled`,
  and `(not on this OS)` for tools whose `platforms` excludes the current OS.

### 7.7 `dotfm diff [tool...]`

Three drift layers, printed in order:

1. **Registry drift**: enabled tools missing from the registry, or registry
   tools available on this OS but not in `enabled` (informational).
2. **Link drift**: per-link state for each enabled tool. Reports
   `expected ↔ actual` for `WrongLink`, marks file/directory `conflict`s.
3. **Content drift** (with `--content`): for each `ExistingFile` conflict,
   prints a unified diff between the source file and the destination.
   Binary files are detected via NUL-byte heuristic and skipped.

Exit code: 0 if no drift in layers 2/3, 1 otherwise. Layer 1
"available, not enabled" is **informational**, not drift.

### 7.8 `dotfm doctor [tool...] [--all] [--no-generic]`

- **Generic checks** (always, unless `--no-generic`):
  - `HOME` / `USERPROFILE` / `XDG_CONFIG_HOME` resolve to existing paths.
  - `dotfiles_root` is a directory.
  - Every enabled tool's links are `CorrectLink`.
  - On Windows: Developer Mode is enabled
    (`AllowDevelopmentWithoutDevLicense` registry value).
- **Per-tool doctor scripts** (opt-in):
  - With no positional args, **no** tool scripts run (fast default).
  - With `<tool>...`, runs those tools' `doctor` scripts.
  - With `--all`, runs every enabled tool's `doctor` script.
  - Scripts are run via `pwsh -NoLogo -NoProfile -File` on Windows and `bash`
    on Linux, the same convention as `script`.
- Exit code: 0 healthy, 1 generic-check failed, 2 doctor script failed.

### 7.9 `dotfm.toml` schema

```toml
[tools.<name>]
description = "..."                       # optional, shown by `list`
platforms = ["windows", "linux"]          # optional; defaults to both

# String form: link one file or directory at <src> to <dst>.
[[tools.<name>.links]]
src = "<path relative to dotfiles_root>"
dst.windows = "$APPDATA/..."              # env vars expanded at apply time
dst.linux   = "$XDG_CONFIG_HOME/..."

# Table form: link multiple files individually under <dst>.
[[tools.<name>.links]]
src = { dir = "<subdir>", include = ["a.json", "b.json"] }
dst.windows = "$APPDATA/SomeApp/User"
dst.linux   = "$XDG_CONFIG_HOME/SomeApp/User"

[[tools.<name>.post_apply]]
run = ["cmd", "arg1", "arg2"]
os  = ["linux"]                           # optional OS filter

[tools.<name>.script]                     # legacy-script delegation
windows = "<path to .ps1>"                # executed with pwsh
linux   = "<path to .sh>"                 # executed with bash

[tools.<name>.unscript]                   # planned: optional cleanup script
windows = "<path to .ps1>"
linux   = "<path to .sh>"

[tools.<name>.doctor]                     # tool-specific health check
windows = "<path to .ps1>"
linux   = "<path to .sh>"
```

Environment variables supported in `dst`: `$HOME`, `$USERPROFILE`, `$APPDATA`,
`$LOCALAPPDATA`, `$XDG_CONFIG_HOME` (with fallback to `$HOME/.config` on
Linux or `$USERPROFILE/.config` on Windows), `~`.

### 7.10 `config.toml` schema

```toml
dotfiles_root = "~/dotfiles"
enabled = ["tool1", "tool2"]
```

Lives at `$XDG_CONFIG_HOME/dotfm/config.toml` (falling back to
`$HOME/.config/dotfm/config.toml`). The `DOTFM_CONFIG` environment variable
overrides this path, primarily for tests.

### 7.11 Output styling: Nerd Font icons

- `dotfm` ships ASCII glyphs by default and Nerd Font glyphs as opt-in.
- `--icons auto|nerd|plain` (global flag, default `auto`).
- `auto` honors the `NERD_FONT` environment variable; any non-empty,
  non-falsy value (`1`, `true`, etc.) selects Nerd Font output.
- A terminal cannot advertise which font is rendering, so auto-detection is
  intentionally absent; selection is strictly opt-in.

## 8. Non-functional requirements

- **Idempotency.** Running `dotfm apply` N times has the same effect as
  running it once.
- **Dry-run.** `--dry-run` never touches the filesystem; output is a plan.
- **Safety.** `remove` only deletes links that point inside `dotfiles_root`;
  it never `rm -rf`s files owned by the user.
- **Single binary.** Distributed as one Rust binary per target. No runtime
  dependencies beyond the OS itself.
- **Startup time.** `dotfm status` and `dotfm list` under 100 ms on a warm
  cache with a registry of ~50 tools.
- **Error messages.** Every failure names the tool, the link (src/dst), and
  the cause. No bare `io::Error` at the CLI boundary.
- **Windows symlink privilege.** If symlink creation fails because Developer
  Mode is not enabled and the process is not elevated, the error explains
  both options explicitly.
- **Logging.** `--verbose` uses `tracing`; `RUST_LOG` is honored.

## 9. Distribution and platform support

- **Pre-built binaries** are produced by the `release` workflow on every
  `v*` git tag, for: `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`,
  `x86_64-apple-darwin`, `aarch64-apple-darwin`. Archives include README and
  both license files; `SHA256SUMS` accompanies the release.
- **Termux (aarch64-linux-android)** is **not** prebuilt. The `cross` 0.2.x
  Android NDK image fails to link `libunwind`, and a custom NDK pipeline
  costs more than it saves. Termux users build from source:
  `pkg install rust && cargo install dotfm`.
- **macOS** is now a first-class CI platform (`cargo test` runs on
  `macos-latest`). Binaries are released for both Intel and Apple Silicon.

## 10. Out of scope (reiteration)

- Templating engines.
- Encryption / secret managers.
- Cloud / online state.
- Installing binaries / package management.
- Backup beyond `--force` producing `.bak` files.
- Removing `.bak` files automatically.

## 11. Success metrics

- The motivating `dotfiles` repo's `setup.sh` + `setup.ps1` count drops from
  **40 to under 10** (only true edge cases retained via delegation).
- New tool onboarding requires **≤ 1 commit to dotfm.toml** (no new shell
  script in the common case).
- First-time machine setup: `git clone` → `dotfm init` → `dotfm add ...` →
  `dotfm apply` completes under 30 seconds on a representative machine.
- `dotfm apply` exit code 0 on a freshly applied machine and again immediately
  after (proves idempotency in practice).

## 12. Open questions

- **`dotfm add --all`.** Enable everything in the registry — useful shortcut
  or footgun? Currently not implemented.
- **`unscript` for `script`-delegated tools.** Schema slot is reserved but
  there is no execution code yet; tools using `script` print a manual-cleanup
  warning on `remove` instead.
- **`status <tool>`.** `dotfm status` does not currently accept positional
  arguments; should it, mirroring `diff <tool>`?
- **`fm` short binary alias.** Considered (`[[bin]] name = "fm"`) but
  deferred until 0.0.x stabilizes.
- **Glob in `include`.** The `Expand` form requires explicit filenames;
  glob (`*.json`) would be ergonomic but adds a dependency. Deferred.

### Resolved

- ~~**Registry file name.**~~ `dotfm.toml` at repo root.
- ~~**`.bak` cleanup.**~~ Leave them; document.
- ~~**Hook timing.**~~ Always run `post_apply` on every `apply`.
- ~~**License.**~~ MIT OR Apache-2.0 (Rust ecosystem convention).
- ~~**Crate name.**~~ Originally `dotup` (taken on crates.io), now `dotfm`.
  GitHub repo and binary follow.

## 13. Appendix: comparison matrix

| Requirement                                  | chezmoi               | Stow | dotbot       | dotfm              |
| -------------------------------------------- | --------------------- | ---- | ------------ | ------------------ |
| Symlink by default                           | No                    | Yes  | Yes          | **Yes**            |
| No filename prefix rules                     | No (`dot_`, `private_`) | Yes  | Yes          | **Yes**            |
| Declarative per-host tool list               | Templates             | No   | Split YAMLs  | **Yes (TOML list)** |
| Tool-unit `add` / `remove`                   | No                    | No   | No           | **Yes**            |
| Cross-platform (Win + Linux + macOS)         | Yes                   | No   | Yes          | **Yes**            |
| Single binary                                | Yes                   | No   | Python       | **Yes**            |
| Built-in `diff` with content unified-diff    | `chezmoi diff`        | No   | No           | **Yes**            |
| Tool-specific `doctor` script delegation     | No                    | No   | No           | **Yes**            |
