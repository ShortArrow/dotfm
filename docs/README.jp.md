# dotup

> 「ファイル」ではなく「ツール」で考える人のための、宣言的でシンボリックリンクベースの dotfiles マネージャ。

**バージョン:** 0.0.2（pre-alpha）。

---

## なぜ dotup か

既存の dotfiles マネージャはそれぞれ独自のメンタルモデルを強制してきます：

| ツール | メンタルモデル | デフォルトでシンボリックリンク | 端末別ツール選択 | Windows |
|---|---|---|---|---|
| [chezmoi](https://www.chezmoi.io/) | ファイル単位 + テンプレート | いいえ（コピー） | Go テンプレートで分岐 | はい |
| [GNU Stow](https://www.gnu.org/software/stow/) | パッケージ単位、symlink | はい | なし | いいえ |
| [dotbot](https://github.com/anishathalye/dotbot) | YAML タスクリスト | はい | YAML を分ければ可 | はい |

`dotup` は上記ツールが妥協しているポイントを 3 点押し切ります：

1. **ツール単位で考える。ファイル単位ではない。** `dotup add alacritty` と書く。`dotup add ~/.config/alacritty/alacritty.toml` ではない。ツールに属する全ファイルが一括で動く。
2. **シンボリックリンクがデフォルト。** dotfiles リポジトリのファイルを直接編集すれば即反映される。編集を伝搬させるためだけの `apply` ステップは不要。
3. **端末別ツールリストを第一級で扱う。** 各マシンが「どのツールを使うか」を TOML のリスト 1 つで宣言する。テンプレも条件分岐もなく、`enabled = ["alacritty", "mise", "nvim"]` と書くだけ。

## インストール

**未公開。** リリース後は：

```sh
# ソースから
cargo install --git https://github.com/<owner>/dotup

# crates.io から（予定）
cargo install dotup

# ビルド済みバイナリ（予定）
# GitHub Releases を参照
```

## クイックスタート

```sh
# 1. dotfiles リポ（ルートに dotup.toml がある）を clone
git clone https://github.com/<you>/dotfiles ~/dotfiles

# 2. このマシンで dotup を初期化
dotup init --dotfiles ~/dotfiles

# 3. このマシンで使いたいツールを追加
dotup add alacritty mise nvim git

# 4. シンボリックリンクを全部作成
dotup apply

# あとで：ドリフト確認、ツール削除など
dotup status
dotup remove nvim
dotup apply
```

## コマンド

| コマンド | 説明 |
|---|---|
| `dotup init` | `~/.config/dotup/config.toml` を作成し、dotfiles リポのパスを記録する。 |
| `dotup add <tool>...` | このマシンの enabled リストにツールを追加する。 |
| `dotup remove <tool>...` | ツールを外す：symlink を撤去し、enabled リストから削除する。 |
| `dotup apply [tool...]` | 有効ツールの symlink を作成/更新する。冪等。 |
| `dotup status` | 有効ツールの一覧と、各 symlink が同期しているかを表示する。 |
| `dotup list` | dotfiles リポの `dotup.toml` に定義された全ツールを表示する。 |

すべてのコマンドで `--dry-run` と `--verbose` をサポート。

### Nerd Font アイコン

[Nerd Font](https://www.nerdfonts.com/) のグリフをステータスバッジに使えます。
CLI 側からターミナルが Nerd Font を使っているか確実に検出する手段は無いため、
opt-in 方式：

```sh
export NERD_FONT=1        # --icons=auto（デフォルト）のときに Nerd グリフを有効化
dotup --icons nerd status # 環境変数に関わらず強制的にグリフ
dotup --icons plain list  # 強制的に ASCII
```

## 設定ファイル

役割を明確に分離した 2 つの TOML ファイル：

### `dotup.toml` — リポジトリ全体のツールレジストリ

dotfiles リポのルートに置く。すべてのツール、ソースパス、各 OS でのリンク先を宣言する。全マシンで共有。

```toml
[tools.alacritty]
description = "Alacritty terminal emulator config"
[[tools.alacritty.links]]
src = "alacritty"
dst.windows = "$APPDATA/alacritty"
dst.linux   = "$XDG_CONFIG_HOME/alacritty"

[tools.git]
description = "Git global config"
[[tools.git.links]]
src = "git/.gitconfig"
dst.windows = "$USERPROFILE/.gitconfig"
dst.linux   = "$HOME/.gitconfig"
[[tools.git.post_apply]]
run = ["git", "config", "--global", "init.defaultBranch", "main"]
```

### `~/.config/dotup/config.toml` — マシン単位の状態

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
- **0.0.3** — post-apply フック（例：`git config`）、エッジケース用に既存 `setup.sh` / `setup.ps1` への委譲。
- **0.1.0** — TOML スキーマ確定、ビルド済みバイナリ、エラーコード文書化。
- **将来** — `diff`、`doctor`、`watch` モードなど。

意図的にスコープ外：テンプレート、暗号化、シークレット管理、クラウド同期。

## 日本語話者向け補足

- 英語版は [README.md](../README.md)。本ドキュメントはその日本語対訳＋補足。
- ツール名の由来：`dot`（dotfiles）+ `up`（up, update, upkeep）。
- 命名の元候補だった `sharrow` は破棄。

## ドキュメント

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
