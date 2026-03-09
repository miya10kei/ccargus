# ccargus

Claude Codeの複数worktreeセッションを一元管理するTUIアプリケーション。

lazygit風のマルチペインレイアウトで、worktree一覧・Claude Codeとのフルインタラクション・エディタでのコード確認を統合します。

## レイアウト

```
┌ Worktrees ─────────────┐┌ Claude Code ─────────────────────┐
│ ▼ miya10kei/ccargus    ││ > Analyzing code...              │
│   ▶ main               ││ > Found 3 issues                 │
│     feat/qa            ││ > Fixing issue #1...             │
│ ▼ miya10kei/api-server ││                                  │
│     main               ││ ┌──── vim (~/proj) ────────────┐ │
│ ▶ miya10kei/other-proj ││ │ src/main.rs                  │ │
│   (no worktrees)       ││ │ fn main() {                  │ │
│                        ││ │   ...          [Esc: close]  │ │
│                        ││ └──────────────────────────────┘ │
└────────────────────────┘└──────────────────────────────────┘
 ccargus │ main │ ~/dev/.../ccargus │ PID: 12345 │ Running 5m
```

## 主な機能

- **マルチworktree管理** — git worktreeと連携し、複数プロジェクトのClaude Codeセッションを同時に管理
- **フルインタラクティブPTY** — 埋め込みターミナルでClaude Codeを直接操作（キーボード・マウス対応）
- **Q&Aサブセッション** — メインの作業を中断せずに、Fork（コンテキスト引き継ぎ）またはNew（新規）で質問・相談
- **フローティングエディタ** — Claude Codeの出力を確認しながらエディタでコードを編集
- **tmuxライクなコピーモード** — ターミナル出力のテキスト選択・クリップボードコピー
- **ghq連携** — `ghq list`でリポジトリを検索・選択し、worktreeを素早く作成
- **スクロールバック** — PTY出力の履歴をスクロールして確認

## 前提条件

- [Rust](https://www.rust-lang.org/) (latest stable)
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) (`claude` コマンドがPATHに必要)
- [ghq](https://github.com/x-motemen/ghq) (リポジトリ管理)
- [git](https://git-scm.com/) (worktree操作)

## インストール

### Homebrew (macOS / Linux)

```bash
brew tap miya10kei/tap https://github.com/miya10kei/ccargus
brew install ccargus
```

### ソースからビルド

```bash
git clone https://github.com/miya10kei/ccargus.git
cd ccargus
cargo build --release
```

ビルド後のバイナリは `target/release/ccargus` に生成されます。

## 使い方

```bash
ccargus
```

### キーバインド

#### Worktree操作（左ペイン）

| キー | 説明 |
|------|------|
| `j` / `k` / `↑` / `↓` | worktree一覧を移動 |
| `Enter` / `l` / `h` | ツリーの展開・折りたたみ |
| `n` | 新規worktree作成 |
| `d` | worktree削除 |
| `e` | フローティングエディタを起動 |
| `s` | Q&Aサブセッションを起動 |
| `Tab` | 左ペイン ↔ 右ペインのフォーカス切替 |
| `q` | 終了 |

#### ターミナルペイン（右ペイン）

| キー | 説明 |
|------|------|
| `Tab` | 左ペインへフォーカスを戻す |
| `Ctrl+w` | メイン ↔ Q&Aペインのフォーカス切替 |
| その他 | Claude Codeへの直接入力 |

### 新規worktree作成フロー

1. `n` キーでリポジトリ選択（`ghq list` + フィルタリング）
2. worktree選択（既存worktreeまたは新規作成）
3. ベースブランチを選択し、Claude Codeを自動起動

### Q&Aサブセッション

`s` キーで起動モードを選択:

| モード | 説明 |
|--------|------|
| **Fork** | メインのコンテキストを `--fork-session` で引き継いでQ&Aを起動 |
| **New** | 同じ作業ディレクトリで新規Q&Aを起動 |

```
┌ Worktrees ────────┐┌ main ──────────┐┌ Q&A ──────────────┐
│ ▼ miya10kei/ccargus││ > Fixing #1... ││ Q: この関数の意図は？│
│   ▶ main          ││ > Done.        ││ A: これは...       │
│                    ││                ││                    │
└────────────────────┘└────────────────┘└────────────────────┘
```

## 設定

`~/.config/ccargus/config.toml`:

```toml
[editor]
command = "vim"           # エディタコマンド (default: "vim")

[keybindings]
new_worktree = "n"        # 新規worktree (default: "n")
delete_worktree = "d"     # worktree削除 (default: "d")
open_editor = "e"         # エディタ起動 (default: "e")
qa_worktree = "s"         # Q&A起動 (default: "s")

[worktree]
base_dir = "~/.local/share/ccargus/worktrees"  # worktree格納先
```

## アーキテクチャ

```
src/
├── main.rs               エントリポイント、イベントループ
├── app.rs                アプリケーション状態管理
├── action.rs             Action enum定義
├── config.rs             TOML設定読み込み
├── tui.rs                Terminal初期化・終了
├── event.rs              イベントハンドリング
├── copy_mode.rs          コピーモード
├── keys.rs               キー入力変換
│
├── components/           UIコンポーネント (Component trait)
│   ├── worktree_tree.rs    左ペイン: worktreeツリー
│   ├── terminal_pane.rs    右ペイン: PTY出力
│   ├── status_line.rs      ステータスライン
│   ├── confirm_dialog.rs   確認ダイアログ
│   ├── qa_selector.rs      Q&Aモード選択
│   ├── repo_selector.rs    リポジトリ選択
│   └── editor_float.rs     フローティングエディタ
│
└── domain/               ドメインロジック (UI非依存)
    ├── worktree.rs         worktreeライフサイクル管理
    ├── pty.rs              PTYセッション管理
    └── repo.rs             リポジトリ検出 (ghq連携)
```

### 設計方針

- **Component Architecture** — 各UIコンポーネントがComponent traitを実装し、状態・イベント処理・描画を自己完結で保持
- **Action-based Event Flow** — Event → Action enum → update() → render() の一方向データフロー
- **Async (tokio)** — `tokio::select!` でtick/render/event/PTY I/Oを多重化
- **デマンド駆動描画** — vt100パーサーのdirty flagにより、変更時のみ再描画

## 開発

### 必要ツール

- [mise](https://mise.jdx.dev/) — ツールバージョン管理 + タスクランナー
- [cargo-nextest](https://nexte.st/) — テストランナー
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) — 依存チェック
- [bacon](https://github.com/Canop/bacon) — バックグラウンドチェック
- [taplo](https://taplo.tamasfe.dev/) — TOMLフォーマッター

### タスク

```bash
mise run check          # 全チェック実行 (format + lint + test + deny)
mise run lint           # clippy (pedantic)
mise run format         # コードフォーマット
mise run format-check   # フォーマットチェック
mise run test           # テスト実行
mise run deny           # 依存クレートチェック
mise run dev            # baconによるバックグラウンドチェック
```

## ライセンス

[MIT](LICENSE)
