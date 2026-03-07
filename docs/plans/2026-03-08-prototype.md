# ccargus Prototype Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Claude Codeの複数セッションを管理するTUIプロトタイプを構築する（最小限コア）

**Architecture:** Component Architecture + Action-based event flow。ratatui + crossterm でTUI、portable-pty でPTY管理、vt100 で端末出力パース。tokio::select! で tick/render/event を多重化。

**Tech Stack:** Rust, ratatui, crossterm (event-stream), portable-pty, vt100, tokio, serde + toml, color-eyre

**Dev Tools:** clippy (pedantic), rustfmt, cargo-nextest, bacon, cargo-deny, taplo, mise

**Prototype Scope:**
- 開発環境セットアップ
- Component trait + Action enum の基盤
- セッション一覧（ハードコード → 動的ツリー）
- PTY埋め込み（フルインタラクション）
- ステータスライン
- 新規セッション起動・切替・削除
- TDDアプローチ

---

## Task 1: Rustプロジェクト初期化 + 開発環境セットアップ

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore`
- Create: `rustfmt.toml`
- Create: `deny.toml`
- Create: `.mise.toml`

**Step 1: cargo init**

Run: `cargo init --name ccargus`

**Step 2: 依存クレートを追加**

```bash
cargo add ratatui
cargo add crossterm --features event-stream
cargo add color-eyre
cargo add portable-pty
cargo add vt100
cargo add tokio --features full
cargo add serde --features derive
cargo add toml
cargo add futures
```

**Step 3: rustfmt.toml を作成**

```toml
edition = "2024"
max_width = 100
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
use_field_init_shorthand = true
```

**Step 4: Cargo.toml に clippy lint設定を追加**

```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
must_use_candidate = "allow"
```

**Step 5: deny.toml を作成**

```toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"

[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
    "Unicode-DFS-2016",
]

[bans]
multiple-versions = "warn"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

**Step 6: .mise.toml を作成**

```toml
[tools]
rust = "latest"

[tasks.check]
description = "Run all checks"
depends = ["format-check", "lint", "test", "deny"]

[tasks.lint]
description = "Run clippy"
run = "cargo clippy --all-targets --all-features"

[tasks.format]
description = "Format code"
run = ["cargo fmt", "taplo format"]

[tasks.format-check]
description = "Check formatting"
run = ["cargo fmt --check", "taplo check"]

[tasks.test]
description = "Run tests with nextest"
run = "cargo nextest run"

[tasks.deny]
description = "Check dependencies"
run = "cargo deny check"

[tasks.dev]
description = "Start bacon for background checking"
run = "bacon"
```

**Step 7: .gitignore**

```
/target
```

**Step 8: src/main.rs を最小構成に**

```rust
use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    println!("ccargus");
    Ok(())
}
```

**Step 9: ビルド + lint + format 確認**

```bash
cargo build
cargo clippy --all-targets --all-features
cargo fmt --check
```

Expected: すべて成功、警告なし

**Step 10: Commit**

```bash
git add -A
git commit -m "chore: initialize Rust project with dev tooling"
```

---

## Task 2: Action enum + Event/Tui 基盤（TDD）

**Files:**
- Create: `src/action.rs`
- Create: `src/event.rs`
- Create: `src/tui.rs`
- Modify: `src/main.rs`

**Step 1: src/action.rs のテスト + 実装**

Action enumの定義。全てのユーザーアクションを表現する。

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Tick,
    Render,
    Quit,
    FocusNext,
    SelectNextSession,
    SelectPrevSession,
    CreateSession,
    DeleteSession,
    SendBytes(Vec<u8>),
    Resize(u16, u16),
    None,
}
```

テスト: Action enumの基本的な等値比較

**Step 2: src/event.rs のテスト + 実装**

Event enum + tokioベースの EventHandler。crossterm の EventStream を tokio::select! で tick と多重化。

```rust
pub enum Event {
    Tick,
    Render,
    Key(KeyEvent),
    Resize(u16, u16),
    Error,
}
```

EventHandler:
- `new(tick_rate, frame_rate)` で初期化
- tokio::spawn で crossterm::event::EventStream を監視
- `next() -> Result<Event>` で受信

テスト:
- EventHandler 生成時にパニックしないこと
- tick_rate/frame_rate のバリデーション

**Step 3: src/tui.rs の実装**

Tui構造体: Terminal の初期化・終了・パニックフック設定。

- `new()` → raw mode有効化、alternate screen
- `exit()` → raw mode無効化、alternate screen終了
- パニック時に自動でターミナル復旧

**Step 4: main.rs を更新**

tokio::main で async main。Tui初期化 → EventHandler でイベントループ → Ctrl+C/q で終了。

**Step 5: テスト実行 + lint**

```bash
cargo nextest run
cargo clippy --all-targets --all-features
```

**Step 6: 動作確認**

```bash
cargo run
```

Expected: TUIが起動し、`q` で終了

**Step 7: Commit**

```bash
git add -A
git commit -m "feat: add Action enum, async EventHandler, and Tui setup"
```

---

## Task 3: Component trait + App構造体（TDD）

**Files:**
- Create: `src/components/mod.rs`
- Create: `src/app.rs`

**Step 1: src/components/mod.rs — Component trait定義**

```rust
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::action::Action;

pub trait Component {
    fn init(&mut self) -> Result<()> { Ok(()) }
    fn handle_key_event(&mut self, key: KeyEvent) -> Action { let _ = key; Action::None }
    fn update(&mut self, action: &Action) -> Result<()> { let _ = action; Ok(()) }
    fn render(&self, frame: &mut Frame, area: Rect);
}
```

**Step 2: src/app.rs — App構造体（TDD）**

App は全体の状態を保持し、Component間の調整を行う。

```rust
pub struct App {
    pub state: AppState,
    pub focus: Focus,
    pub selected_session: usize,
}
```

テスト:
- `App::new()` の初期状態
- `quit()` で状態遷移
- `toggle_focus()` の切替
- `select_next_session(max)` の境界値
- `select_prev_session()` の下限

**Step 3: テスト実行 + lint**

```bash
cargo nextest run
cargo clippy --all-targets --all-features
```

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: add Component trait and App state management"
```

---

## Task 4: UIコンポーネント — SessionTree, TerminalPane, StatusLine（TDD）

**Files:**
- Create: `src/components/session_tree.rs`
- Create: `src/components/terminal_pane.rs`
- Create: `src/components/status_line.rs`
- Modify: `src/main.rs`

**Step 1: session_tree.rs（TDD）**

ハードコードデータでリポジトリ > worktreeの階層ツリー表示。

Component trait実装:
- `render()`: List ウィジェットでツリー描画
- `handle_key_event()`: j/k で移動

テスト（ratatui::backend::TestBackend使用）:
- render()でBlock titleが "Sessions" であること
- 選択状態が反映されること

**Step 2: terminal_pane.rs（TDD）**

プレースホルダー（"No session selected"表示）。後でvt100レンダリングを追加。

テスト:
- セッションなし時のプレースホルダー表示

**Step 3: status_line.rs（TDD）**

ステータスライン描画。

テスト:
- render()でリポジトリ名・ブランチ名が含まれること

**Step 4: main.rs でレイアウト統合**

Layout::default() で vertical(main area + statusline 1行) → horizontal(25% sessions + 75% terminal) に分割し、各コンポーネントを配置。

**Step 5: テスト実行 + 動作確認**

```bash
cargo nextest run
cargo run
```

Expected: 3ペイン構成のTUIが表示される

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: add SessionTree, TerminalPane, StatusLine components"
```

---

## Task 5: ドメイン層 — PtySession（TDD）

**Files:**
- Create: `src/domain/mod.rs`
- Create: `src/domain/pty.rs`

**Step 1: src/domain/pty.rs（TDD）**

PtySession: portable-pty でPTY生成、vt100でパース。

```rust
pub struct PtySession {
    writer: Box<dyn Write + Send>,
    screen: Arc<Mutex<vt100::Parser>>,
    working_dir: String,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}
```

メソッド:
- `spawn(cmd, working_dir, rows, cols) -> Result<Self>`
- `write(data: &[u8]) -> Result<()>`
- `screen() -> Arc<Mutex<vt100::Parser>>`
- `is_alive() -> bool`
- `kill()`

テスト（実プロセスではなく `echo` 等の軽量コマンドで）:
- `spawn("echo", ...)` が成功すること
- `is_alive()` の初期値確認
- `write()` がエラーにならないこと
- `screen()` が空でないパーサーを返すこと

**Step 2: テスト実行**

```bash
cargo nextest run
```

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: add PtySession with portable-pty and vt100"
```

---

## Task 6: ドメイン層 — SessionManager（TDD）

**Files:**
- Create: `src/domain/session.rs`
- Modify: `src/app.rs`

**Step 1: src/domain/session.rs（TDD）**

SessionManager: セッションのCRUDとライフサイクル管理。

```rust
pub struct SessionInfo {
    pub id: usize,
    pub name: String,
    pub repo: String,
    pub branch: String,
    pub pty: PtySession,
}

pub struct SessionManager {
    sessions: Vec<SessionInfo>,
    next_id: usize,
}
```

テスト:
- 初期状態でlen() == 0
- create_session() でlen()が増加
- remove_session() でlen()が減少
- remove_session() の範囲外インデックスが安全
- get_mut() の正常・範囲外ケース
- IDが連番で採番される

※ PtySession部分はspawnに成功する軽量コマンドを使用

**Step 2: app.rs に SessionManager を統合**

```rust
pub struct App {
    pub state: AppState,
    pub focus: Focus,
    pub selected_session: usize,
    pub session_manager: SessionManager,
}
```

**Step 3: テスト実行**

```bash
cargo nextest run
```

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: add SessionManager for session lifecycle"
```

---

## Task 7: PTY出力レンダリング + キー入力転送

**Files:**
- Modify: `src/components/terminal_pane.rs`
- Modify: `src/event.rs` or `src/app.rs`

**Step 1: terminal_pane.rs を更新（TDD）**

vt100::Parser のスクリーンバッファを ratatui Buffer にセル単位で書き込む。

ヘルパー関数: `convert_color(vt100::Color) -> ratatui::style::Color`

テスト:
- `convert_color(vt100::Color::Default)` → `Color::Reset`
- `convert_color(vt100::Color::Idx(1))` → `Color::Indexed(1)`
- `convert_color(vt100::Color::Rgb(255, 0, 0))` → `Color::Rgb(255, 0, 0)`

**Step 2: キー入力のPTY転送**

`key_to_bytes(KeyEvent) -> Vec<u8>` 関数。

テスト:
- `Char('a')` → `[0x61]`
- `Ctrl+C` → `[0x03]`
- `Enter` → `[0x0d]`
- `Backspace` → `[0x7f]`
- `Esc` → `[0x1b]`
- `Up` → `[0x1b, 0x5b, 0x41]`

**Step 3: app.rs の update() にセッション操作を統合**

Action::CreateSession → session_manager.create_session()
Action::DeleteSession → session_manager.remove_session()
Action::SendBytes → pty.write()

**Step 4: テスト実行 + 動作確認**

```bash
cargo nextest run
cargo run
```

Expected:
- `n` でClaude Codeセッション起動、右ペインに出力表示
- Tab でフォーカス切替、キー入力がClaude Codeに転送される
- j/k でセッション切替
- d でセッション削除

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: render PTY output and forward key input to sessions"
```

---

## Task 8: セッション一覧の動的ツリー表示 + ステータスライン連動

**Files:**
- Modify: `src/components/session_tree.rs`
- Modify: `src/components/status_line.rs`

**Step 1: session_tree.rs を動的データに更新（TDD）**

SessionManagerのデータからリポジトリ名でグループ化し、ツリー形式で表示。

ヘルパー関数: `group_by_repo(sessions) -> Vec<(String, Vec<&SessionInfo>)>`

テスト:
- 空の場合 → 空リスト
- 同一repoの複数セッション → 1グループにまとまる
- 異なるrepo → 別グループ

**Step 2: status_line.rs を選択中セッションの実データに更新**

**Step 3: テスト実行 + 動作確認**

```bash
cargo nextest run
cargo run
```

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: dynamic session tree grouped by repository"
```

---

## Task 9: 最終チェック

**Step 1: 全チェック実行**

```bash
mise run check
```

Expected:
- `cargo fmt --check`: 差分なし
- `cargo clippy --all-targets --all-features`: 警告なし
- `cargo nextest run`: 全テスト PASS
- `cargo deny check`: 問題なし

**Step 2: 統合動作確認**

1. `cargo run` でTUI起動
2. `n` でClaude Codeセッション起動、右ペインに出力表示
3. `Tab` でフォーカス切替、Claude Codeにキー入力可能
4. 複数セッション作成、`j/k` で切替
5. `d` でセッション削除
6. `q` で終了

**Step 3: Commit**

```bash
git add -A
git commit -m "chore: final cleanup and verification"
```
