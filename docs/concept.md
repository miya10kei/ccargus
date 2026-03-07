# ccargus - Concept Document

## Overview

Claude Codeの複数セッションを一元管理するTUIツール。
lazygit風のマルチペインレイアウトで、セッション一覧・Claude Codeとのフルインタラクション・エディタでのコード確認を統合する。

## Tech Stack

- **Language**: Rust
- **TUI Framework**: ratatui + crossterm
- **PTY Management**: portable-pty（子プロセスの疑似端末管理）
- **Config Format**: TOML (`~/.config/ccargus/config.toml`)

## Layout

```
┌ Sessions ──────────────┐┌ Claude Code ─────────────────────┐
│ ▼ miya10kei/ccargus    ││ > Analyzing code...              │
│   ▶ main               ││ > Found 3 issues                 │
│     feat/qa            ││ > Fixing issue #1...             │
│ ▼ miya10kei/api-server ││                                  │
│     main               ││ ┌──── vim (~/proj) ────────────┐ │
│ ▶ miya10kei/other-proj ││ │ src/main.rs                  │ │
│   (no sessions)        ││ │ fn main() {                  │ │
│                        ││ │   ...          [Esc: close]  │ │
│                        ││ └──────────────────────────────┘ │
│                        ││                                  │
└────────────────────────┘└──────────────────────────────────┘
 ccargus │ main │ ~/dev/.../ccargus │ PID: 12345 │ Running 5m
```

- **左**: セッション一覧（リポジトリ > worktree の階層ツリー表示）
- **右**: Claude Codeのフルインタラクティブターミナル（PTY埋め込み）
- **下部ステータスライン**: 選択中セッションの詳細情報（リポジトリ、ブランチ、ディレクトリ、PID、ステータス）
- **フローティングウィンドウ**: セッションの作業ディレクトリでエディタを起動

## Features

### Session Management

| 機能 | キー | 説明 |
|------|------|------|
| セッション選択 | `j/k` or `↑/↓` | セッション一覧を移動 |
| ツリー開閉 | `Enter` or `l/h` | リポジトリノードの展開・折りたたみ |
| 新規セッション | `n` | リポジトリ選択 → worktree選択 → セッション起動 |
| セッション削除 | `d` | セッションを停止・削除 |
| エディタ起動 | `e` | フローティングウィンドウでエディタ起動 |
| フォーカス切替 | `Tab` | 左ペイン ↔ 右ペイン（Claude Code操作） |
| 終了 | `q` | ccargus終了（セッションは継続可能） |

### New Session Flow

新規セッション作成は以下の3ステップ:

1. **リポジトリ選択**: `ghq list` + `fzf` でclone済みリポジトリを選択
2. **worktree選択**: 選択リポジトリのworktree一覧を表示し選択（新規worktree作成も可能）
3. **セッション起動**: 選択worktreeのディレクトリで`claude`を起動

### Session Lifecycle

- ccargusが起動した新規セッション + 既存の`claude`プロセスも検出・取り込み
- 各セッションはPTYを通じて管理、出力をキャプチャしてメインペインにレンダリング
- セッション情報（PID、作業ディレクトリ、ステータス）をリアルタイム更新

### Q&A Sub-Session

メインセッションの作業を中断せずに、コードについて質問したり設計相談ができるサブセッション機能。

**起動方法**: セッション選択中に `s` キーで起動モードを選択

| モード | 説明 |
|--------|------|
| Fork | メインセッションのコンテキストを `--fork-session` で引き継いでQ&Aセッションを起動。作業中のコードや背景を理解した状態で回答が得られる |
| New | 同じ作業ディレクトリで新規セッションを起動。まっさらな状態で質問したい場合に使用 |

**レイアウト**: 右ペインをさらに左右分割し、左がメインセッション、右がQ&Aサブセッション

```
┌ Sessions ──────────────┐┌ main ──────────────┐┌ Q&A ──────────────┐
│ ▼ miya10kei/ccargus    ││ > Fixing issue #1...││ Q: この関数の意図は？│
│   ▶ main               ││ > Done.            ││ A: これは...       │
│     feat/qa            ││                    ││                    │
│ ▼ miya10kei/api-server ││                    ││                    │
│     main               ││                    ││                    │
│                        ││                    ││                    │
└────────────────────────┘└────────────────────┘└────────────────────┘
 ccargus │ main │ ~/dev/.../ccargus │ PID: 12345 │ Running 5m │ Q&A: fork
```
```

- メイン・Q&A間のフォーカス切替は `Ctrl+w` で行う
- Q&Aセッションの終了は `Ctrl+d` またはQ&Aペイン内で `/exit`

### Floating Editor

- セッションの作業ディレクトリでエディタ（設定可能）をフローティングウィンドウとして起動
- Claude Codeの出力を確認しながらコードを編集するワークフローを実現

## Config

`~/.config/ccargus/config.toml`:

```toml
[editor]
command = "vim"

[keybindings]
new_session = "n"
delete_session = "d"
open_editor = "e"
```

## Architecture

### Design Pattern

- **Component Architecture**: ratatui公式推奨パターン。各UIコンポーネントがComponent traitを実装し、状態・イベント処理・描画を自己完結で持つ
- **Action-based Event Flow**: Event → Action enum → update() → render() の一方向データフロー
- **Async (tokio + channel)**: tokio::select! で tick/render/event を多重化。PTY I/Oは専用スレッドで処理

### Performance

- PTY読み取りは専用スレッドで行い、描画スレッドとはlock-freeまたは最小限のlockで共有
- フレームレート制御（60fps上限）で無駄な再描画を防止
- vt100パーサーのdirty flagで変更時のみ再描画

### Module Structure

```
src/
  main.rs               - エントリポイント（起動・パニックフック設定）
  app.rs                - App構造体、アプリ全体の状態管理・update()
  action.rs             - Action enum（全アクションの定義）
  config.rs             - TOML設定読み込み
  tui.rs                - Terminal初期化・終了・フレームレート制御
  event.rs              - Event enum + EventHandler（tokio::select!でmultiplex）

  components/           - UIコンポーネント（Component trait実装）
    mod.rs              - Component trait定義
    session_tree.rs     - 左ペイン: セッション一覧ツリー
    terminal_pane.rs    - 右ペイン: PTY出力レンダリング
    status_line.rs      - 下部ステータスライン

  domain/               - ドメインロジック（UI非依存）
    mod.rs
    session.rs          - SessionManager: セッションライフサイクル管理
    pty.rs              - PtySession: PTY生成・I/O・vt100パース
```

### Dev Tools

- **clippy** (pedantic): 厳格なlint
- **rustfmt**: コードフォーマッター
- **cargo-nextest**: モダンなテストランナー
- **bacon**: バックグラウンド自動チェック
- **cargo-deny**: 依存クレートのライセンス・脆弱性チェック
- **taplo**: TOML linter/formatter
- **mise**: ツールバージョン管理 + タスクランナー
