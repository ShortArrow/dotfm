# dotfm — プロダクト要件定義書（PRD）

**ステータス:** ドラフト（実装前）
**対象リリース:** 0.0.1 MVP

---

## 1. 概要

`dotfm` は、dotfiles 共有リポジトリのうち「このマシンで有効にしたいツール」を宣言し、シンボリックリンクとして配置する CLI ツール。dotfiles リポ内に散らばる手書きの `setup.sh` / `setup.ps1` を、1 つの宣言的な TOML レジストリと小さな状態付き CLI に置き換える。

## 2. 課題定義

典型的なパワーユーザーの dotfiles リポジトリは、ツールごとにディレクトリ（`alacritty/`、`nvim/`、`git/` ...）を持ち、それぞれに `setup.sh` か `setup.ps1` が置かれている。動機となったリポジトリではこの数が **40 個**（PowerShell 19 個、Bash 21 個）に達しており、重複が多い：

- 各スクリプトが「対象が既存なら削除してから symlink を作る」を再実装している。
- パス/コマンドの違いで Windows 用と Linux 用が別立てになっている。
- 「このラップトップでは alacritty と git だけ、あのデスクトップでは全部」のような機構が無い。
- 新ツール追加はスクリプトをもう 2 本書く作業。

既存マネージャは以上の条件を同時には満たさない：

- **chezmoi** はデフォルトでコピー、`dot_` / `private_` / `executable_` / `run_` のファイル名 prefix 規約あり、メンタルモデルはファイル単位、端末差は Go テンプレートで分岐する。宣言的ツールリストは無い。
- **GNU Stow** は symlink 第一だが Unix 限定。端末別ツールリストは無い。
- **dotbot** は YAML 駆動でクロスプラットフォームだが、端末別 enable/disable の CLI は無く、ユーザが YAML を分割する必要がある。

`dotfm` はこれらが同時には埋めないニッチを狙う。

## 3. ゴール

- **レジストリを 1 つに。** 散在する `setup.sh` / `setup.ps1` を 1 つの `dotfm.toml` に置き換える。
- **マシン別ツールリスト。** 同一リポを別端末がチェックアウトしても enabled リストは独立する。
- **デフォルトでシンボリックリンク。** dotfiles リポ内のファイルを編集すれば `apply` なしで即反映される。
- **ツール単位のメンタルモデル。** `add`、`remove`、`apply` は個々のファイルではなくツールを対象にする。
- **クロスプラットフォーム。** Windows と Linux（WSL は Linux 扱い）。Rust 単一バイナリ。
- **冪等な `apply`。** 2 回実行しても同じ結果になる。`--dry-run` で副作用なしにプレビューできる。
- **漸進的劣化。** exotic な初期化（curl インストーラ、root 所有ファイル等）を持つツールは、レジストリから既存シェルスクリプトに委譲できる。

## 4. 非ゴール

- **テンプレートなし。** Go templates、handlebars 等は扱わない。条件分岐が欲しければファイルを分けて、端末ごとに別ツールを有効化するアプローチを取る。
- **暗号化・シークレット管理なし。** パスワードマネージャ連携も扱わない。
- **クラウド同期なし。** ホスティングされた state も扱わない。
- **0.0.x で macOS は第一級サポートしない。** 「linux」として偶然動くかもしれないが検証はしない。
- **登録されていないファイルは管理しない。** `dotfm.toml` に無ければ `dotfm` は一切触らない。
- **パッケージマネージャは代替しない。** `dotfm` は設定を配置するだけで、ソフトウェアをインストールしない。

## 5. ターゲットユーザー

- 複数マシン（デスクトップ、ラップトップ、業務用、WSL、自宅サーバなど）で 1 つの dotfiles リポを使いまわす開発者。
- GUI ではなく、宣言的な設定ファイル 1 つと CLI に慣れているユーザー。
- 端末ごとに **違うツールセット** を使っており、現在はブランチ、コメントアウト、手作業でしのいでいるユーザー。
- dotfiles リポを唯一の情報源にしておきたく、編集に apply ステップを挟みたくない **symlink 派** のユーザー。

## 6. ユーザーストーリー

- 新しいマシンで `git clone <dotfiles> && dotfm init && dotfm add <tools> && dotfm apply` と打てば設定が揃う。
- 最小構成のマシンで、リポの共有ファイルを編集せず、ブランチも切らずに一部のツールだけ有効化できる。
- 新ツールを dotfiles リポに追加するときは `dotfm.toml` に 1 エントリ書くだけで済む。新しいシェルスクリプトは書かない。
- `dotfm apply` を何度走らせても、重複や壊れたリンクや驚きの上書きが発生しない。
- このマシンでツールの使用を止めたいとき、`dotfm remove <tool>` が symlink 撤去と enabled 更新を同時に行う。
- 変態的な初期化（tmux のテーマダウンロード等）は既存の `setup.sh` を残して `dotfm apply` から呼び出したい。
- Windows では symlink 権限エラー（Developer Mode か管理者権限が必要）が明示的に表示され、黙って失敗しない。

## 7. 機能要件

### 7.1 `dotfm init`

- `~/.config/dotfm/config.toml` が無ければ作成。既存なら丁重に失敗する（`--force` を案内）。
- `--dotfiles <path>` フラグから、または cwd が dotfiles リポっぽい（`dotfm.toml` が存在）なら自動検出で `dotfiles_root` を記録。
- 空の `enabled = []` を持つ初期ファイルを書き出す。

### 7.2 `dotfm add <tool>...`

- 各ツールが `dotfm.toml` に存在するか検証。不明なら非ゼロで終了し、利用可能なツール一覧を表示する。
- `config.toml` の `enabled` に追加。`toml_edit` を使って既存フォーマットとコメントを保つ。
- 自動 apply は **しない**。`run 'dotfm apply' to create symlinks` というヒントを出す。
- `--apply` フラグで直後に apply を走らせる選択肢を提供する。

### 7.3 `dotfm remove <tool>...`

- 各ツールについて、`dotfm.toml` の link 定義を参照し、symlink が `dotfiles_root` の内側を指していることを確認してから削除する（安全策：ユーザが差し替えたリンクは消さない）。
- `config.toml` の `enabled` から外す。
- `script = ...` のみで `unscript` が無いツールは、手動 cleanup が必要かもしれないと警告を出した上で enabled から外す。

### 7.4 `dotfm apply [tool...]`

- デフォルト：`enabled` の全ツールを apply。
- 引数形式：指定ツールのみ apply（各ツールは `enabled` に含まれていること）。
- 各ツール、各 link について：
  1. 宛先の現在状態（missing / correct link / wrong link / existing file / existing directory）を調べる。
  2. `correct link` なら何もしない。
  3. `wrong link` や `missing` なら作成する。
  4. `existing file` や `existing directory` ならそのリンクは中止し、明確なエラーを出す。`--force` 指定時は `.bak` 拡張子でバックアップしてから置換。
- その後、ツールの `post_apply` フック（コマンドトークン配列）を実行。
- ツールが失敗しても次のツールへ進み、結果を集約し、1 つでも失敗していれば非ゼロで終了。
- `--dry-run` と `--verbose` をサポート。

### 7.5 `dotfm status`

- 各 enabled ツールを、link ごとのステータスバッジ（`ok` / `missing` / `wrong` / `conflict`）と共に表示。
- 全 link が `ok` なときだけ終了コード 0。

### 7.6 `dotfm list`

- `dotfm.toml` 内の全ツールを、enabled 状態のマーカー付きで表示。

### 7.7 `dotfm.toml` スキーマ（案）

```toml
[tools.<name>]
description = "..."         # 任意。`list` で表示される
platforms = ["windows","linux"]   # 任意。省略時は両方対象

[[tools.<name>.links]]
src = "<dotfiles_root からの相対パス>"
dst.windows = "$APPDATA/..."
dst.linux   = "$XDG_CONFIG_HOME/..."

[[tools.<name>.post_apply]]
run = ["cmd", "arg1", "arg2"]
os  = ["linux"]             # 任意フィルタ

[tools.<name>.script]       # 委譲フォールバック
windows = "<.ps1 へのパス>"  # pwsh で実行
linux   = "<.sh へのパス>"   # bash で実行
```

`dst` で使える環境変数：`$HOME`、`$USERPROFILE`、`$APPDATA`、`$LOCALAPPDATA`、`$XDG_CONFIG_HOME`（未設定時は `$HOME/.config` にフォールバック）、`~`。

### 7.8 `config.toml` スキーマ

```toml
dotfiles_root = "~/dotfiles"
enabled = ["tool1", "tool2"]
```

## 8. 非機能要件

- **冪等性。** `dotfm apply` を N 回実行しても、1 回実行したのと同じ結果になる。
- **Dry-run。** `--dry-run` はファイルシステムに一切触らない。出力は計画のみ。
- **安全性。** `remove` は `dotfiles_root` を指す symlink のみ削除対象。ユーザ所有ファイルを `rm -rf` しない。
- **単一バイナリ。** ターゲットごとに Rust バイナリ 1 本で配布する。OS 以外の runtime 依存は無し。
- **起動時間。** ツール 50 個規模のレジストリで、warm cache の `dotfm status` と `dotfm list` が 100 ms 以内。
- **エラーメッセージ。** 失敗は必ずツール名・link (src/dst)・原因を表示する。CLI 境界で生の `io::Error` を出さない。
- **Windows symlink 権限。** Developer Mode 無効 + 非 elevated で symlink 作成が失敗した場合、エラーメッセージが両方の対処を明示する。
- **ロギング。** `--verbose` は `tracing` を使う。`RUST_LOG` を尊重する。

## 9. スコープ外（再掲）

- テンプレートエンジン
- 暗号化 / シークレットマネージャ
- クラウド / オンライン state
- 0.0.x での macOS 検証
- バイナリのインストール / パッケージ管理
- `--force` による `.bak` 以上のバックアップ機構

## 10. 成功指標

- 動機となった dotfiles リポで `setup.sh` + `setup.ps1` の数が **40 → 10 未満** に減る（委譲で残る真のエッジケースのみ）。
- 新ツール追加は **dotfm.toml の 1 コミットで済む**（一般的なケースで新シェルスクリプトは不要）。
- 新マシン初期設定：`git clone` → `dotfm init` → `dotfm add ...` → `dotfm apply` が代表マシンで 30 秒未満で完了する。
- 新規 apply 直後に再度 `dotfm apply` で終了コード 0 が続く（実運用での冪等性の証拠）。

## 11. 未解決事項

- **レジストリファイル名。** リポジトリルートの `dotfm.toml` vs `.dotfm/registry.toml`。現状はルート `dotfm.toml` が有力。
- **`.bak` の扱い。** 以前の `--force` apply が作った `.bak` を `remove` が掃除するか否か。現状は「残す」方針。文書化する。
- **フック発火タイミング。** `apply` が `post_apply` フックを毎回走らせるか、変更時のみ走らせるか。現状は「毎回」が有力。chezmoi の `run_onchange_` に相当する variant が必要になれば後で足す。
- **`dotfm add --all`。** レジストリ全ツール有効化するショートカットを提供するか。便利か、罠か。
- ~~**ライセンス。** MIT / Apache-2.0 / dual。~~ 決定：MIT OR Apache-2.0 のデュアル（Rust エコシステム慣例）。

## 12. 付録：比較表

| 要件 | chezmoi | Stow | dotbot | dotfm |
|---|---|---|---|---|
| デフォルトで symlink | No | Yes | Yes | **Yes** |
| ファイル名 prefix 規約なし | No（`dot_`、`private_`） | Yes | Yes | **Yes** |
| 端末別ツールリスト宣言 | テンプレ | なし | YAML 分割 | **Yes（TOML リスト）** |
| ツール単位の `add`/`remove` | No | No | No | **Yes** |
| クロスプラットフォーム（Win + Linux） | Yes | No | Yes | **Yes** |
| 単一バイナリ | Yes | No | Python | **Yes** |
