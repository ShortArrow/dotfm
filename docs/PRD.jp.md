# dotfm — プロダクト要件定義書（PRD）

**ステータス:** Living document。v0.0.6 時点の実装を反映。
**現行リリース:** 0.0.6（pre-alpha）。バージョンごとの履歴は
[CHANGELOG.md](../CHANGELOG.md) を参照。**(planned)** と記したセクションは
未実装。

---

## 1. 概要

`dotfm` は、dotfiles 共有リポジトリのうち「このマシンで有効にしたいツール」を
宣言し、シンボリックリンクとして配置する CLI ツール。dotfiles リポ内に散ら
ばる手書きの `setup.sh` / `setup.ps1` を、1 つの宣言的な TOML レジストリと
小さな状態付き CLI に置き換える。

## 2. 課題定義

典型的なパワーユーザーの dotfiles リポジトリは、ツールごとにディレクトリ
（`alacritty/`、`nvim/`、`git/` ...）を持ち、それぞれに `setup.sh` か
`setup.ps1` が置かれている。動機となったリポジトリではこの数が
**40 個**（PowerShell 19 個、Bash 21 個）に達しており、重複が多い：

- 各スクリプトが「対象が既存なら削除してから symlink を作る」を再実装している。
- パス／コマンドの違いで Windows 用と Linux 用が別立てになっている。
- 「このラップトップでは alacritty と git だけ、あのデスクトップでは全部」の
  ような機構が無い。
- 新ツール追加はスクリプトをもう 2 本書く作業。

既存マネージャは以上の条件を同時には満たさない：

- **chezmoi** はデフォルトでコピー、`dot_` / `private_` / `executable_` /
  `run_` のファイル名 prefix 規約あり、メンタルモデルはファイル単位、端末差は
  Go テンプレートで分岐する。宣言的ツールリストは無い。
- **GNU Stow** は symlink 第一だが Unix 限定。端末別ツールリストは無い。
- **dotbot** は YAML 駆動でクロスプラットフォームだが、端末別 enable/disable の
  CLI は無く、ユーザが YAML を分割する必要がある。

`dotfm` はこれらが同時には埋めないニッチを狙う。

## 3. ゴール

- **レジストリを 1 つに。** 散在する `setup.sh` / `setup.ps1` を 1 つの
  `dotfm.toml` に置き換える。
- **マシン別ツールリスト。** 同一リポを別端末がチェックアウトしても enabled
  リストは独立する。
- **デフォルトでシンボリックリンク。** dotfiles リポ内のファイルを編集すれば
  `apply` なしで即反映される。
- **ツール単位のメンタルモデル。** `add`、`remove`、`apply` は個々のファイル
  ではなくツールを対象にする。
- **クロスプラットフォーム。** Windows、Linux（WSL は Linux 扱い）、macOS を
  CI で確認。Rust 単一バイナリ。
- **冪等な `apply`。** 2 回実行しても同じ結果になる。`--dry-run` で副作用なし
  にプレビューできる。
- **漸進的劣化。** exotic な初期化（curl インストーラ、root 所有ファイル等）を
  持つツールは、レジストリから既存シェルスクリプトに委譲できる。

## 4. 非ゴール

- **テンプレートなし。** Go templates、handlebars 等は扱わない。条件分岐が
  欲しければファイルを分けて、端末ごとに別ツールを有効化するアプローチを取る。
- **暗号化・シークレット管理なし。** パスワードマネージャ連携も扱わない。
- **クラウド同期なし。** ホスティングされた state も扱わない。
- **登録されていないファイルは管理しない。** `dotfm.toml` に無ければ `dotfm`
  は一切触らない。
- **パッケージマネージャは代替しない。** `dotfm` は設定を配置するだけで、
  ソフトウェアをインストールしない。

## 5. ターゲットユーザー

- 複数マシン（デスクトップ、ラップトップ、業務用、WSL、自宅サーバ、Termux など）
  で 1 つの dotfiles リポを使いまわす開発者。
- GUI ではなく、宣言的な設定ファイル 1 つと CLI に慣れているユーザー。
- 端末ごとに **違うツールセット** を使っており、現在はブランチ、コメントアウト、
  手作業でしのいでいるユーザー。
- dotfiles リポを唯一の情報源にしておきたく、編集に apply ステップを挟みたく
  ない **symlink 派** のユーザー。

## 6. ユーザーストーリー

- 新しいマシンで `git clone <dotfiles> && dotfm init && dotfm add <tools> &&
  dotfm apply` と打てば設定が揃う。
- 最小構成のマシンで、リポの共有ファイルを編集せず、ブランチも切らずに一部の
  ツールだけ有効化できる。
- 新ツールを dotfiles リポに追加するときは `dotfm.toml` に 1 エントリ書くだけ
  で済む。新しいシェルスクリプトは書かない。
- `dotfm apply` を何度走らせても、重複や壊れたリンクや驚きの上書きが発生しない。
- このマシンでツールの使用を止めたいとき、`dotfm remove <tool>` が symlink
  撤去と enabled 更新を同時に行う。
- 変態的な初期化（tmux のテーマダウンロード等）は既存の `setup.sh` を残して
  `dotfm apply` から呼び出したい。
- `dotfm apply` で何が変わるか事前確認したい。実ファイルが残っている場合は
  unified diff で内容も比較したい（`dotfm diff --content`）。
- Windows では symlink 権限エラー（Developer Mode か管理者権限が必要）が
  明示的に表示され、黙って失敗しない。
- 「dotfiles 壊れた気がする」と同僚に言う前に確認したい：環境は
  `dotfm doctor`、ツール固有のチェックは `dotfm doctor <tool>`。

## 7. 機能要件

### 7.1 `dotfm init`

- `~/.config/dotfm/config.toml` が無ければ作成。既存なら丁重に失敗する
  （`--force` を案内）。
- `--dotfiles <path>` フラグから、または cwd が dotfiles リポっぽい
  （`dotfm.toml` が存在）なら自動検出で `dotfiles_root` を記録。
- 空の `enabled = []` を持つ初期ファイルを書き出す。

### 7.2 `dotfm add <tool>...`

- 各ツールが `dotfm.toml` に存在するか検証。不明なら非ゼロで終了し、利用可能な
  ツール一覧を表示する。
- `config.toml` の `enabled` に追加。`toml_edit` を使って既存フォーマットと
  コメントを保つ。
- 自動 apply は **しない**。`run 'dotfm apply' to create symlinks` という
  ヒントを出す。
- `--apply` フラグで直後に apply を走らせる選択肢を提供する。

### 7.3 `dotfm remove <tool>...`

- 各ツールについて、`dotfm.toml` の link 定義を参照し、symlink が
  `dotfiles_root` の内側を指していることを確認してから削除する（安全策：
  ユーザが差し替えたリンクは消さない）。
- `config.toml` の `enabled` から外す。
- `script = ...` のみで `unscript` が無いツールは、手動 cleanup が必要かも
  しれないと警告を出した上で enabled から外す。

### 7.4 `dotfm apply [tool...]`

- デフォルト：`enabled` の全ツールを apply。
- 引数形式：指定ツールのみ apply（各ツールは `enabled` に含まれていること）。
- 各ツール、各 link について：
  1. 宛先の現在状態（missing / correct link / wrong link / existing file /
     existing directory）を調べる。
  2. `correct link` なら何もしない。
  3. `wrong link` や `missing` なら作成する。
  4. `existing file` や `existing directory` ならそのリンクは中止し、明確な
     エラーを出す。`--force` 指定時は `.bak` 拡張子でバックアップしてから置換。
- その後、ツールの `post_apply` フック（コマンドトークン配列）を実行。
- `script` のみで定義されたツールは、対応プラットフォームのスクリプトを実行
  （Windows は `pwsh`、Linux は `bash`）。
- ツールが失敗しても次のツールへ進み、結果を集約し、1 つでも失敗していれば
  非ゼロで終了。
- `--dry-run` と `--verbose` をサポート。

### 7.5 `dotfm status`

- 各 enabled ツールを、link ごとのステータスバッジ（`ok` / `missing` /
  `wrong` / `conflict`）と共に表示。
- 全 link が `ok` なときだけ終了コード 0。

### 7.6 `dotfm list`

- `dotfm.toml` 内の全ツールを、enabled 状態のマーカー付きで表示。`platforms`
  に現在 OS が含まれないツールには `(not on this OS)` が付く。

### 7.7 `dotfm diff [tool...]`

3 レイヤーのドリフトを順に表示：

1. **レジストリドリフト**：enabled なのにレジストリに無いツール、または
   現在 OS で利用可能なのに enabled にされていないツール（参考情報）。
2. **リンクドリフト**：enabled 各ツールの各 link の状態。`WrongLink` の場合は
   `expected ↔ actual` を表示、ファイル／ディレクトリ衝突はマーク。
3. **コンテンツドリフト**（`--content` 付与時）：`ExistingFile` 衝突に対し、
   ソースと宛先の unified diff を表示。バイナリは NUL バイト検出でスキップ。

終了コード：レイヤー 2/3 にドリフトが無ければ 0、あれば 1。レイヤー 1 の
「available, not enabled」は **参考情報** であってドリフト扱いしない。

### 7.8 `dotfm doctor [tool...] [--all] [--no-generic]`

- **汎用チェック**（`--no-generic` 指定時を除き常時）：
  - `HOME` / `USERPROFILE` / `XDG_CONFIG_HOME` が存在パスに解決すること。
  - `dotfiles_root` がディレクトリであること。
  - enabled 各ツールの全 link が `CorrectLink` であること。
  - Windows: Developer Mode が有効であること
    （`AllowDevelopmentWithoutDevLicense` レジストリ値）。
- **ツール固有 doctor スクリプト**（opt-in）：
  - 引数なしのときは **何も走らない**（高速デフォルト）。
  - `<tool>...` 指定時はそのツールの `doctor` スクリプトを実行。
  - `--all` 指定時は enabled 全ツールの `doctor` を実行。
  - スクリプトは Windows で `pwsh -NoLogo -NoProfile -File`、Linux で `bash`
    から起動（`script` と同じ規約）。
- 終了コード：0 健全、1 汎用チェック失敗、2 doctor スクリプト失敗。

### 7.9 `dotfm.toml` スキーマ

```toml
[tools.<name>]
description = "..."                       # 任意。`list` で表示される
platforms = ["windows", "linux"]          # 任意。省略時は両方対象

# 文字列形式：<src> 1 つ（ファイルまたはディレクトリ）を <dst> にリンク
[[tools.<name>.links]]
src = "<dotfiles_root からの相対パス>"
dst.windows = "$APPDATA/..."              # apply 時に環境変数展開
dst.linux   = "$XDG_CONFIG_HOME/..."

# テーブル形式：<dst> ディレクトリ配下に複数ファイルを個別リンク
[[tools.<name>.links]]
src = { dir = "<subdir>", include = ["a.json", "b.json"] }
dst.windows = "$APPDATA/SomeApp/User"
dst.linux   = "$XDG_CONFIG_HOME/SomeApp/User"

[[tools.<name>.post_apply]]
run = ["cmd", "arg1", "arg2"]
os  = ["linux"]                           # 任意 OS フィルタ

[tools.<name>.script]                     # レガシースクリプト委譲
windows = "<.ps1 へのパス>"               # pwsh で実行
linux   = "<.sh へのパス>"                # bash で実行

[tools.<name>.unscript]                   # planned: 任意のクリーンアップスクリプト
windows = "<.ps1 へのパス>"
linux   = "<.sh へのパス>"

[tools.<name>.doctor]                     # ツール固有ヘルスチェック
windows = "<.ps1 へのパス>"
linux   = "<.sh へのパス>"
```

`dst` で使える環境変数：`$HOME`、`$USERPROFILE`、`$APPDATA`、`$LOCALAPPDATA`、
`$XDG_CONFIG_HOME`（未設定時は Linux で `$HOME/.config`、Windows で
`$USERPROFILE/.config` にフォールバック）、`~`。

### 7.10 `config.toml` スキーマ

```toml
dotfiles_root = "~/dotfiles"
enabled = ["tool1", "tool2"]
```

`$XDG_CONFIG_HOME/dotfm/config.toml` に置く（未設定時は
`$HOME/.config/dotfm/config.toml` にフォールバック）。`DOTFM_CONFIG` 環境変数
で上書き可能（主にテスト用）。

### 7.11 出力スタイル：Nerd Font アイコン

- `dotfm` はデフォルトで ASCII グリフ、Nerd Font は opt-in。
- グローバルフラグ `--icons auto|nerd|plain`（デフォルト `auto`）。
- `auto` は `NERD_FONT` 環境変数を尊重。空・偽値（`0`、`false` 等）でなければ
  Nerd Font 出力を選択。
- ターミナルが Nerd Font を使っているかを CLI 側から確実に判定する手段は
  無いため、auto-detection は意図的に持たない。選択は厳密に opt-in。

## 8. 非機能要件

- **冪等性。** `dotfm apply` を N 回実行しても、1 回実行したのと同じ結果になる。
- **Dry-run。** `--dry-run` はファイルシステムに一切触らない。出力は計画のみ。
- **安全性。** `remove` は `dotfiles_root` を指す symlink のみ削除対象。
  ユーザ所有ファイルを `rm -rf` しない。
- **単一バイナリ。** ターゲットごとに Rust バイナリ 1 本で配布する。OS 以外の
  runtime 依存は無し。
- **起動時間。** ツール 50 個規模のレジストリで、warm cache の `dotfm status`
  と `dotfm list` が 100 ms 以内。
- **エラーメッセージ。** 失敗は必ずツール名・link (src/dst)・原因を表示する。
  CLI 境界で生の `io::Error` を出さない。
- **Windows symlink 権限。** Developer Mode 無効 + 非 elevated で symlink 作成
  が失敗した場合、エラーメッセージが両方の対処を明示する。
- **ロギング。** `--verbose` は `tracing` を使う。`RUST_LOG` を尊重する。

## 9. 配布とプラットフォームサポート

- **ビルド済みバイナリ** は `release` workflow が `v*` タグ push 毎に生成：
  `x86_64-unknown-linux-gnu`、`x86_64-pc-windows-msvc`、`x86_64-apple-darwin`、
  `aarch64-apple-darwin`。アーカイブには README と両ライセンスファイル同梱、
  `SHA256SUMS` 付き。
- **Termux (aarch64-linux-android)** は **ビルド済み配布しない**。`cross`
  0.2.x の Android NDK イメージが `libunwind` リンクに失敗し、独自 NDK パイプ
  ライン保守はコスト過多。Termux ユーザーは
  `pkg install rust && cargo install dotfm` でソースからビルド。
- **macOS** は CI の第一級プラットフォーム（`cargo test` を `macos-latest` で
  実行）。Intel と Apple Silicon の両方でリリース。

## 10. スコープ外（再掲）

- テンプレートエンジン
- 暗号化 / シークレットマネージャ
- クラウド / オンライン state
- バイナリのインストール / パッケージ管理
- `--force` による `.bak` 以上のバックアップ機構
- `.bak` ファイルの自動削除

## 11. 成功指標

- 動機となった dotfiles リポで `setup.sh` + `setup.ps1` の数が
  **40 → 10 未満** に減る（委譲で残る真のエッジケースのみ）。
- 新ツール追加は **dotfm.toml の 1 コミットで済む**（一般的なケースで
  新シェルスクリプトは不要）。
- 新マシン初期設定：`git clone` → `dotfm init` → `dotfm add ...` →
  `dotfm apply` が代表マシンで 30 秒未満で完了する。
- 新規 apply 直後に再度 `dotfm apply` で終了コード 0 が続く（実運用での
  冪等性の証拠）。

## 12. 未解決事項

- **`dotfm add --all`。** レジストリ全ツール有効化するショートカットを
  提供するか。便利か、罠か。現状未実装。
- **`script` 委譲ツールの `unscript`。** スキーマ枠は予約済みだが実行コードは
  未実装。`script` を使うツールは `remove` 時に「手動 cleanup が必要かも」と
  警告を出すのみ。
- **`status <tool>`。** `dotfm status` は現在引数を取らない。`diff <tool>`
  と同様に対応すべきか。
- **`fm` 短縮バイナリエイリアス。** `[[bin]] name = "fm"` を検討したが、
  0.0.x 安定までは保留。
- **`include` の glob。** `Expand` 形式は現在ファイル名明示が必要。
  `*.json` のような glob は便利だが依存追加になるため保留。

### 解決済み

- ~~**レジストリファイル名。**~~ リポジトリルートの `dotfm.toml`。
- ~~**`.bak` の扱い。**~~ 残す方針。文書化する。
- ~~**フック発火タイミング。**~~ `apply` 毎に `post_apply` を毎回走らせる。
- ~~**ライセンス。**~~ MIT OR Apache-2.0（Rust エコシステム慣例）。
- ~~**クレート名。**~~ 当初 `dotup`（crates.io で取得済）、現在 `dotfm`。
  GitHub リポ名・バイナリ名も追従。

## 13. 付録：比較表

| 要件                                          | chezmoi               | Stow | dotbot      | dotfm              |
| --------------------------------------------- | --------------------- | ---- | ----------- | ------------------ |
| デフォルトで symlink                          | No                    | Yes  | Yes         | **Yes**            |
| ファイル名 prefix 規約なし                    | No（`dot_`、`private_`） | Yes  | Yes         | **Yes**            |
| 端末別ツールリスト宣言                        | テンプレ              | なし | YAML 分割   | **Yes（TOML リスト）** |
| ツール単位の `add` / `remove`                 | No                    | No   | No          | **Yes**            |
| クロスプラットフォーム（Win + Linux + macOS） | Yes                   | No   | Yes         | **Yes**            |
| 単一バイナリ                                  | Yes                   | No   | Python      | **Yes**            |
| `diff` で content の unified-diff 同梱        | `chezmoi diff`        | No   | No          | **Yes**            |
| ツール固有 `doctor` スクリプト委譲            | No                    | No   | No          | **Yes**            |
