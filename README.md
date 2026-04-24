# dotfm

[![crates.io](https://img.shields.io/crates/v/dotfm.svg)](https://crates.io/crates/dotfm)
[![CI](https://github.com/ShortArrow/dotfm/actions/workflows/ci.yml/badge.svg)](https://github.com/ShortArrow/dotfm/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/dotfm.svg)](#license)

> A declarative, symlink-based dotfiles manager for humans who think in **tools**, not files.

> **⚠ Pre-alpha (0.0.x).** The TOML schema may change without notice; error
> codes and output format are not yet stable. Use for personal experiments
> only; don't hard-wire it into production provisioning scripts yet.

---

## Why dotfm?

Existing dotfiles managers each enforce their own mental model:

| Tool | Mental model | Symlink by default | Per-host tool list | Windows |
|---|---|---|---|---|
| [chezmoi](https://www.chezmoi.io/) | File unit, templates | No (copy mode) | Via Go templates | Yes |
| [GNU Stow](https://www.gnu.org/software/stow/) | Package unit, symlinks | Yes | No | No |
| [dotbot](https://github.com/anishathalye/dotbot) | YAML task list | Yes | Via split YAMLs | Yes |

`dotfm` is opinionated about three things the above compromise on:

1. **Tool unit, not file unit.** You `dotfm add alacritty`, not `dotfm add ~/.config/alacritty/alacritty.toml`. All files that belong to a tool move as one.
2. **Symlinks are the default.** Edit files in your dotfiles repo and changes take effect immediately. No `apply` step needed just to propagate edits.
3. **Per-host tool list is a first-class concept.** Each machine declares *which tools it uses* in a single TOML list. No templates, no conditionals — just `enabled = ["alacritty", "mise", "nvim"]`.

## Installation

```sh
# From crates.io
cargo install dotfm

# From source (latest main)
cargo install --git https://github.com/ShortArrow/dotfm

# Prebuilt binaries
# https://github.com/ShortArrow/dotfm/releases
```

## Quick start

```sh
# 1. Clone your dotfiles repo (which contains a dotfm.toml at its root)
git clone https://github.com/<you>/dotfiles ~/dotfiles

# 2. Initialize dotfm on this machine
dotfm init --dotfiles ~/dotfiles

# 3. Enable the tools you want on this machine
dotfm add alacritty mise nvim git

# 4. Create all the symlinks
dotfm apply

# Later: check drift, remove a tool, etc.
dotfm status
dotfm remove nvim
dotfm apply
```

## Commands

| Command | Description |
|---|---|
| `dotfm init` | Create `~/.config/dotfm/config.toml` and record the dotfiles repo path. |
| `dotfm add <tool>...` | Add tools to this machine's enabled list. |
| `dotfm remove <tool>...` | Remove tools: strip symlinks and remove from enabled list. |
| `dotfm apply [tool...]` | Create/update symlinks for enabled tools. Idempotent. |
| `dotfm status` | Show which tools are enabled and whether their symlinks are in sync. |
| `dotfm list` | List all tools defined in the dotfiles repo's `dotfm.toml`. |
| `dotfm doctor [tool...]` | Run environment checks (always). Tool-specific `doctor` scripts only run when a tool is named or `--all` is passed. |
| `dotfm diff [tool...]` | Show drift in three layers: registry vs enabled, expected vs actual symlinks, and (with `--content`) unified diffs for conflicting files. |

Every command supports `--dry-run` and `--verbose`.

### Nerd Font icons

`dotfm` can use [Nerd Font](https://www.nerdfonts.com/) glyphs for status badges.
There is no reliable way for a CLI to detect whether its terminal is rendering
with a Nerd Font, so selection is opt-in:

```sh
export NERD_FONT=1        # enables Nerd glyphs when --icons=auto (default)
dotfm --icons nerd status # force glyphs regardless of env
dotfm --icons plain list  # force ASCII fallback
```

## Configuration

Two TOML files with clearly separated responsibilities:

### `dotfm.toml` — repository-wide tool registry

Lives at the root of your dotfiles repo. Declares every tool, its source paths, and where each should be symlinked on each OS. Shared across all machines.

```toml
# Single path (file or directory) — most common
[tools.alacritty]
description = "Alacritty terminal emulator config"
[[tools.alacritty.links]]
src = "alacritty"
dst.windows = "$APPDATA/alacritty"
dst.linux   = "$XDG_CONFIG_HOME/alacritty"

# Multiple files under a source directory, each linked into a destination
# directory by the same filename. Use this when the destination also holds
# files you don't want to manage (e.g. VS Code's settings + keybindings,
# but not its generated `snippets/` or `globalStorage/`).
[tools.code]
[[tools.code.links]]
src = { dir = "code", include = ["settings.json", "keybindings.json"] }
dst.windows = "$APPDATA/Code/User"
dst.linux   = "$XDG_CONFIG_HOME/Code/User"

[tools.git]
description = "Git global config"
[[tools.git.links]]
src = "git/.gitconfig"
dst.windows = "$USERPROFILE/.gitconfig"
dst.linux   = "$HOME/.gitconfig"
[[tools.git.post_apply]]
run = ["git", "config", "--global", "init.defaultBranch", "main"]
```

### `~/.config/dotfm/config.toml` — per-machine state

Lives outside the dotfiles repo. Records which dotfiles repo this machine uses and which tools are enabled here. Not shared.

```toml
dotfiles_root = "~/dotfiles"

enabled = [
  "alacritty",
  "mise",
  "nvim",
  "git",
]
```

Different machines check out the same dotfiles repo but maintain independent enabled lists.

## Roadmap

- **0.0.1** — `init`, `add`, `remove`, `apply`, `status`, `list`, `--dry-run`, `--force`.
- **0.0.2** — Nerd Font icons via `NERD_FONT` env var and `--icons` flag.
- **0.0.3** — `doctor` command (generic env checks + per-tool doctor script delegation).
- **0.0.4** — Polymorphic `src`: string for single path, `{ dir, include = [...] }` for multi-file expansion.
- **0.0.5** — `doctor` default = generic checks only; tool scripts opt-in via arg or `--all`.
- **0.0.6** — `diff` command (registry / link / content drift, `similar` crate).
- **0.0.7** — Post-apply hooks (e.g. `git config`), delegation to legacy `setup.sh` / `setup.ps1` scripts for edge cases.
- **0.1.0** — Stable TOML schema, prebuilt binaries, documented error codes.
- **future** — `diff`, `doctor`, maybe a `watch` mode.

Out of scope (on purpose): templating, encryption, secret management, cloud sync.

## Documentation

- [Changelog](CHANGELOG.md)
- [日本語 README](docs/README.jp.md)
- [PRD (English)](docs/PRD.md)
- [PRD (日本語)](docs/PRD.jp.md)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
