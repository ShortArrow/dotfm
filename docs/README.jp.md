# dotfm

[![crates.io](https://img.shields.io/crates/v/dotfm.svg)](https://crates.io/crates/dotfm)
[![CI](https://github.com/ShortArrow/dotfm/actions/workflows/ci.yml/badge.svg)](https://github.com/ShortArrow/dotfm/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/dotfm.svg)](#ライセンス)

> 「ファイル」ではなく「ツール」で考える人のための、宣言的でシンボリックリンクベースの dotfiles マネージャ。

> **⚠ Pre-alpha (0.0.x)。** TOML スキーマは予告なく変更される可能性があります。
> エラーコードや出力フォーマットは未確定です。個人的な実験用途のみで、
> 本番のプロビジョニングスクリプトに組み込むのはまだ避けてください。

---

## なぜ dotfm か

既存の dotfiles マネージャはそれぞれ独自のメンタルモデルを強制してきます：

| ツール | メンタルモデル | デフォルトでシンボリックリンク | 端末別ツール選択 | Windows |
|---|---|---|---|---|
| [chezmoi](https://www.chezmoi.io/) | ファイル単位 + テンプレート | いいえ（コピー） | Go テンプレートで分岐 | はい |
| [GNU Stow](https://www.gnu.org/software/stow/) | パッケージ単位、symlink | はい | なし | いいえ |
| [dotbot](https://github.com/anishathalye/dotbot) | YAML タスクリスト | はい | YAML を分ければ可 | はい |

`dotfm` は上記ツールが妥協しているポイントを 3 点押し切ります：

1. **ツール単位で考える。ファイル単位ではない。** `dotfm add alacritty` と書く。`dotfm add ~/.config/alacritty/alacritty.toml` ではない。ツールに属する全ファイルが一括で動く。
2. **シンボリックリンクがデフォルト。** dotfiles リポジトリのファイルを直接編集すれば即反映される。編集を伝搬させるためだけの `apply` ステップは不要。
3. **端末別ツールリストを第一級で扱う。** 各マシンが「どのツールを使うか」を TOML のリスト 1 つで宣言する。テンプレも条件分岐もなく、`enabled = ["alacritty", "mise", "nvim"]` と書くだけ。

## インストール

```sh
# crates.io から
cargo install dotfm

# ソースから (最新 main)
cargo install --git https://github.com/ShortArrow/dotfm

# ビルド済みバイナリ
# https://github.com/ShortArrow/dotfm/releases
```

## クイックスタート

```sh
# 1. dotfiles リポ（ルートに dotfm.toml がある）を clone
git clone https://github.com/<you>/dotfiles ~/dotfiles

# 2. このマシンで dotfm を初期化
dotfm init --dotfiles ~/dotfiles

# 3. このマシンで使いたいツールを追加
dotfm add alacritty mise nvim git

# 4. シンボリックリンクを全部作成
dotfm apply

# あとで：ドリフト確認、ツール削除など
dotfm status
dotfm remove nvim
dotfm apply
```

## コマンド

| コマンド | 説明 |
|---|---|
| `dotfm init` | `~/.config/dotfm/config.toml` を作成し、dotfiles リポのパスを記録する。 |
| `dotfm add <tool>...` | このマシンの enabled リストにツールを追加する。 |
| `dotfm remove <tool>...` | ツールを外す：symlink を撤去し、enabled リストから削除する。 |
| `dotfm apply [tool...]` | 有効ツールの symlink を作成/更新する。冪等。 |
| `dotfm status` | 有効ツールの一覧と、各 symlink が同期しているかを表示する。 |
| `dotfm list` | dotfiles リポの `dotfm.toml` に定義された全ツールを表示する。 |
| `dotfm doctor [tool...]` | 環境チェックを実行（常時）。ツール固有の doctor スクリプトは、ツール名を指定するか `--all` を付けたときのみ走る。 |
| `dotfm diff [tool...]` | 3 レイヤーで差分を表示：レジストリ vs enabled、期待する symlink vs 実際の状態、（`--content` 付与時）衝突ファイルの unified diff。 |

すべてのコマンドで `--dry-run` と `--verbose` をサポート。

### Nerd Font アイコン

[Nerd Font](https://www.nerdfonts.com/) のグリフをステータスバッジに使えます。
CLI 側からターミナルが Nerd Font を使っているか確実に検出する手段は無いため、
opt-in 方式：

```sh
export NERD_FONT=1        # --icons=auto（デフォルト）のときに Nerd グリフを有効化
dotfm --icons nerd status # 環境変数に関わらず強制的にグリフ
dotfm --icons plain list  # 強制的に ASCII
```

## 設定ファイル

役割を明確に分離した 2 つの TOML ファイル：

### `dotfm.toml` — リポジトリ全体のツールレジストリ

dotfiles リポのルートに置く。すべてのツール、ソースパス、各 OS でのリンク先を宣言する。全マシンで共有。

```toml
# 単一パス（ファイルまたはディレクトリ）。最も一般的なケース
[tools.alacritty]
description = "Alacritty terminal emulator config"
[[tools.alacritty.links]]
src = "alacritty"
dst.windows = "$APPDATA/alacritty"
dst.linux   = "$XDG_CONFIG_HOME/alacritty"

# ソースディレクトリ配下の複数ファイルを、宛先ディレクトリに同名で個別symlink。
# 宛先に「管理したくないファイル」が混ざる場合に使う（例：VS Code の
# settings.json + keybindings.json は管理するが、生成物の snippets/ や
# globalStorage/ は触らない）。
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

### `~/.config/dotfm/config.toml` — マシン単位の状態

dotfiles リポの外に置く。このマシンがどの dotfiles リポを使うか、どのツールを有効化しているかを記録。共有しない。

```toml
dotfiles_root = "~/dotfiles"

enabled = [
  "alacritty",
  "mise",
  "nvim",
  "git",
]
```

別のマシンが同じ dotfiles リポを checkout しても、enabled リストはマシンごとに独立する。

## ロードマップ

- **0.0.1** — `init`、`add`、`remove`、`apply`、`status`、`list`、`--dry-run`、`--force`。
- **0.0.2** — `NERD_FONT` 環境変数 / `--icons` フラグによる Nerd Font アイコン対応。
- **0.0.3** — `doctor` コマンド（汎用環境チェック + ツール固有 doctor スクリプトの委譲）。
- **0.0.4** — ポリモーフィック `src`：単一パスは文字列、複数ファイル展開は `{ dir, include = [...] }`。
- **0.0.5** — `doctor` のデフォルトを generic チェックのみに変更。ツールスクリプトは引数指定または `--all` でopt-in。
- **0.0.6** — `diff` コマンド（レジストリ / link / 内容の3レイヤー差分、`similar` crate 使用）。
- **0.0.7** — post-apply フック（例：`git config`）、エッジケース用に既存 `setup.sh` / `setup.ps1` への委譲。
- **0.1.0** — TOML スキーマ確定、ビルド済みバイナリ、エラーコード文書化。
- **将来** — `diff`、`doctor`、`watch` モードなど。

意図的にスコープ外：テンプレート、暗号化、シークレット管理、クラウド同期。

## 日本語話者向け補足

- 英語版は [README.md](../README.md)。本ドキュメントはその日本語対訳＋補足。
- ツール名の由来：`dot`（dotfiles）+ `up`（up, update, upkeep）。
- 命名の元候補だった `sharrow` は破棄。

## ドキュメント

- [Changelog](../CHANGELOG.md)
- [英語 README](../README.md)
- [PRD（英語）](PRD.md)
- [PRD（日本語）](PRD.jp.md)

## ライセンス

以下のいずれかを選択可能なデュアルライセンス：

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) または <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) または <http://opensource.org/licenses/MIT>)

利用者が選択できます。

### コントリビュート

明示的に別段の指定をしない限り、あなたが本プロジェクトに意図的に提出したコントリビュートは、Apache-2.0 の定義に従い、追加条項なしで上記のデュアルライセンスの下で提供されたものとみなされます。
