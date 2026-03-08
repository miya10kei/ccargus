# ccargus - 用語辞書

ccargusコードベースで使用される主要な用語とその定義。

## ドメイン用語

### Worktree

Gitの `git worktree add` で作成される作業ディレクトリ。1つのリポジトリに対して複数のブランチを同時に展開できる仕組み。ccargusではこのworktreeを単位としてClaude Codeセッションを管理する。

対応構造体: `Worktree`（`src/domain/worktree.rs`）

### WorktreeState

worktreeのライフサイクル状態。

| 状態 | 説明 |
|------|------|
| `Running` | PTYセッションが稼働中 |
| `Stopped` | PTYなし。追跡のみ |

対応enum: `WorktreeState`（`src/domain/worktree.rs`）

### WorktreeEntry

ファイルシステム上で発見されたworktreeの情報を保持するデータ構造。ブランチ名・リポジトリ名・元リポジトリパス・worktreeパスを含む。起動時のスキャンやworktree状態の再構築に使用する。

対応構造体: `WorktreeEntry`（`src/domain/worktree.rs`）

### WorktreeManager

worktreeのファイルシステム操作を担当するマネージャ。`git worktree add/remove` コマンドの実行、ディレクトリ構造（`base_dir/host/owner/repo/branch/`）の管理、既存worktreeの検出（スキャン）を行う。

対応構造体: `WorktreeManager`（`src/domain/worktree.rs`）

### WorktreePool

アプリケーション実行中に追跡されるすべてのworktreeを管理するコンテナ。追加・削除・取得・同期の操作を提供する。

対応構造体: `WorktreePool`（`src/domain/worktree.rs`）

### Repository

ghq（Git Host Query）で管理されるGitリポジトリ。ファイルシステムパスと名前（例: `github.com/owner/repo`）で構成される。`ghq list -p` で一覧を取得する。

対応構造体: `Repository`（`src/domain/repo.rs`）

### PTY (Pseudo Terminal)

疑似端末。`portable-pty` クレートを用いて子プロセス（`claude` コマンド）をターミナルI/O付きで起動する。vt100パーサーでANSIエスケープシーケンスを解釈し、TUI上にレンダリングする。

対応構造体: `PtySession`（`src/domain/pty.rs`）

### Q&A Sub-Session

メインのClaude Codeセッションを中断せずに、コードへの質問や設計相談ができるサブセッション。右ペインを左右分割して表示する。

### QaMode

Q&Aサブセッションの起動モード。

| モード | 説明 |
|--------|------|
| `Fork` | メインセッションのコンテキストを `--continue` で引き継ぐ |
| `New` | 同じ作業ディレクトリで新規セッションを起動する |

対応enum: `QaMode`（`src/components/qa_selector.rs`）

### Scan

起動時にファイルシステムを走査し、既存のworktreeを検出するプロセス。物理的なworktreeとメモリ上の`Worktree`オブジェクトを対応づける。

### Sync

メモリ上の`WorktreePool`とファイルシステムの実際の状態を照合するプロセス。稼働中のPTYセッションを保持しつつ、worktree一覧を最新の状態に更新する。

## UI用語

### Component

UIコンポーネントのインターフェースを定義するtrait。`handle_key_event()` と `render()` を持ち、各コンポーネントが状態・イベント処理・描画を自己完結で管理する。

対応trait: `Component`（`src/components/mod.rs`）

### Focus

キーボード入力を受け付けるペインを示す状態。

| 値 | 説明 |
|----|------|
| `Worktrees` | 左ペイン（worktree一覧） |
| `Terminal` | 右ペイン（Claude Code出力） |
| `QaTerminal` | Q&Aペイン |

対応enum: `Focus`（`src/app.rs`）

### WorktreeTree

左ペインに表示されるworktree一覧のツリーコンポーネント。リポジトリごとにworktreeをグループ化し、Running（▶）/ Stopped（○）の状態を表示する。

対応構造体: `WorktreeTree`（`src/components/worktree_tree.rs`）

### TerminalPane

右ペインのPTY出力レンダリングコンポーネント。vt100パーサーの状態をratauiのウィジェットに変換して描画する。メイン単独表示とQ&A分割表示に対応する。

対応構造体: `TerminalPane`（`src/components/terminal_pane.rs`）

### StatusLine

下部ステータスラインコンポーネント。選択中worktreeのブランチ・ディレクトリ・リポジトリ・稼働状態・Q&Aモードを表示する。

対応構造体: `StatusLine`（`src/components/status_line.rs`）

### RepoSelector

ghqリポジトリ一覧からリポジトリを選択するフローティングダイアログ。検索クエリによるフィルタリングに対応する。

対応構造体: `RepoSelector`（`src/components/repo_selector.rs`）

### QaSelector

Q&Aサブセッションの起動モード（Fork / New）を選択するフローティングダイアログ。

対応構造体: `QaSelector`（`src/components/qa_selector.rs`）

### ConfirmDialog

worktree削除時のy/n確認ダイアログ。

対応構造体: `ConfirmDialog`（`src/components/confirm_dialog.rs`）

### EditorFloat

worktreeの作業ディレクトリで外部エディタ（vim, nvimなど）をフローティングウィンドウとして起動するコンポーネント。

対応構造体: `EditorFloat`（`src/components/editor_float.rs`）

## アーキテクチャ用語

### Event

システムからの入力イベント。Key, Mouse, Render, Resize, Tick, Errorの種別がある。`tokio::select!` で多重化される。

対応enum: `Event`（`src/event.rs`）

### Action

イベント処理から生成される内部アクション。`CreateWorktree`, `DeleteWorktree`, `SendBytes`, `FocusNext`, `Quit` など。Event → Action → update() → render() の一方向データフローで処理される。

対応enum: `Action`（`src/action.rs`）

### AppState

アプリケーション全体の状態。`Running` または `Quit`。

対応enum: `AppState`（`src/app.rs`）

### Tui

ratauiターミナルの初期化・終了・フレームレート制御を担当する構造体。

対応構造体: `Tui`（`src/tui.rs`）

### Config

TOML形式の設定ファイル（`~/.config/ccargus/config.toml`）。エディタ設定（`EditorConfig`）、キーバインド設定（`KeybindingsConfig`）、worktreeベースディレクトリ設定（`WorktreeConfig`）の3セクションで構成される。

対応構造体: `Config`（`src/config.rs`）
