# Rename Session to Worktree Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** コードベース全体で "session" の概念を "worktree" に統一する

**Architecture:** `session.rs` の内容を `worktree.rs` に統合し、`session_tree.rs` を `worktree_tree.rs` にリネーム。全ての型名・変数名・UI文字列を一括更新する。

**Tech Stack:** Rust, ratatui

---

### Task 1: domain層の統合 — session.rs を worktree.rs に統合

**Files:**
- Delete: `src/domain/session.rs`
- Modify: `src/domain/worktree.rs` (session.rs の内容を統合、型名リネーム)
- Modify: `src/domain/mod.rs` (`pub mod session` を削除)

**Step 1: `src/domain/worktree.rs` に session.rs の内容を統合**

session.rs の型名を以下のようにリネームして worktree.rs 末尾に追加:
- `SessionInfo` → `Worktree`
- `SessionState` → `WorktreeState`
- `SessionManager` → `WorktreePool`
- メソッド: `close_qa_session` → `close_qa`, `create_qa_session` → `create_qa`, `has_qa_session` → `has_qa`
- メソッド: `add_session` → `add`, `remove_session` → `remove`, `sessions()` → `all()`
- テストヘルパー: `add_stopped_session` → `add_stopped`, `add_test_session` → `add_test`
- テスト関数名も同様にリネーム

**Step 2: `src/domain/mod.rs` から `pub mod session` を削除**

**Step 3: ビルド確認**

Run: `cargo check 2>&1 | head -30`
Expected: session.rs 参照のコンパイルエラー（Task 2以降で修正）

**Step 4: Commit**
```
git add src/domain/
git commit -m "refactor: session.rs を worktree.rs に統合し型名をリネーム"
```

---

### Task 2: Action variants のリネーム

**Files:**
- Modify: `src/action.rs`

**Step 1: Action enum のリネーム**
- `DeleteSession` → `DeleteWorktree`
- `SelectNextSession` → `SelectNextWorktree`
- `SelectPrevSession` → `SelectPrevWorktree`
- `StartSession` → `StartWorktree`
- `StopSession` → `StopWorktree`

**Step 2: Commit**
```
git add src/action.rs
git commit -m "refactor: Action variants を session → worktree にリネーム"
```

---

### Task 3: App のリネーム

**Files:**
- Modify: `src/app.rs`

**Step 1: リネーム**
- import: `domain::session::SessionManager` → `domain::worktree::WorktreePool`
- `Focus::Sessions` → `Focus::Worktrees`
- `selected_session` → `selected_worktree`
- `session_manager` → `worktree_pool`
- `select_next_session` → `select_next_worktree`
- `select_prev_session` → `select_prev_worktree`
- テスト関数名も同様にリネーム

**Step 2: Commit**
```
git add src/app.rs
git commit -m "refactor: App の session 関連フィールド/メソッドを worktree にリネーム"
```

---

### Task 4: Config のリネーム

**Files:**
- Modify: `src/config.rs`

**Step 1: リネーム**
- `default_delete_session` → `default_delete_worktree`
- `default_new_session` → `default_new_worktree`
- `default_qa_session` → `default_qa_worktree`
- `KeybindingsConfig` のフィールド: `delete_session` → `delete_worktree`, `new_session` → `new_worktree`, `qa_session` → `qa_worktree`
- テスト文字列も同様に更新

**Step 2: Commit**
```
git add src/config.rs
git commit -m "refactor: Config の session 関連設定を worktree にリネーム"
```

---

### Task 5: session_tree.rs → worktree_tree.rs リネーム

**Files:**
- Delete: `src/components/session_tree.rs`
- Create: `src/components/worktree_tree.rs`
- Modify: `src/components/mod.rs`

**Step 1: ファイルリネームと型名変更**
- `SessionEntry` → `WorktreeItem`
- `SessionTree` → `WorktreeTree`
- `group_by_repo` の引数/変数: `sessions` → `worktrees`, `session` → `wt`
- UI文字列: `" Sessions "` → `" Worktrees "`, `"(no sessions)"` → `"(no worktrees)"`
- テスト関数名も同様にリネーム

**Step 2: `src/components/mod.rs` 更新**
- `pub mod session_tree` → `pub mod worktree_tree`

**Step 3: Commit**
```
git add src/components/
git commit -m "refactor: session_tree を worktree_tree にリネームし型名を更新"
```

---

### Task 6: main.rs の全参照更新

**Files:**
- Modify: `src/main.rs`

**Step 1: import更新**
- `domain::session::SessionInfo` → `domain::worktree::Worktree`
- `components::session_tree::{SessionEntry, SessionTree}` → `components::worktree_tree::{WorktreeItem, WorktreeTree}`

**Step 2: 変数名/関数名更新**
- `session_tree` → `worktree_tree`
- `session` → `wt`
- `session_manager` → `worktree_pool`
- `selected_session` → `selected_worktree`
- `handle_sessions_key` → `handle_worktrees_key`
- `SessionEntry { ... }` → `WorktreeItem { ... }`
- `SessionInfo::from_worktree_entry` → `Worktree::from_entry`
- `SessionInfo::has_qa_session` → `Worktree::has_qa`

**Step 3: UI文字列更新**
- `"Delete session '...'"` → `"Delete worktree '...'"`
- `"no session"` → `"no worktree"`

**Step 4: Commit**
```
git add src/main.rs
git commit -m "refactor: main.rs の全 session 参照を worktree に更新"
```

---

### Task 7: terminal_pane.rs のUI文字列更新

**Files:**
- Modify: `src/components/terminal_pane.rs`

**Step 1: リネーム**
- `HINT_TEXT`: `"Press 'n' to create a new session."` → `"Press 'n' to create a new worktree."`
- テスト: `renders_banner_when_no_session` → `renders_banner_when_no_worktree`
- テストのassert文字列も更新

**Step 2: Commit**
```
git add src/components/terminal_pane.rs
git commit -m "refactor: terminal_pane のUI文字列を worktree に更新"
```

---

### Task 8: confirm_dialog.rs のテスト文字列更新

**Files:**
- Modify: `src/components/confirm_dialog.rs`

**Step 1:** テスト内の `"Delete session?"` → `"Delete worktree?"` に更新

**Step 2: Commit**
```
git add src/components/confirm_dialog.rs
git commit -m "refactor: confirm_dialog のテスト文字列を worktree に更新"
```

---

### Task 9: qa_selector.rs のUI文字列更新

**Files:**
- Modify: `src/components/qa_selector.rs`

**Step 1:** `"Start fresh Q&A session"` → `"Start fresh Q&A"` に更新

**Step 2: Commit**
```
git add src/components/qa_selector.rs
git commit -m "refactor: qa_selector のUI文字列を更新"
```

---

### Task 10: ビルド・テスト・Lint検証

**Step 1: ビルド確認**
Run: `cargo check`
Expected: 成功

**Step 2: テスト実行**
Run: `cargo test`
Expected: 全テスト通過

**Step 3: Lint確認**
Run: `cargo clippy -- -D warnings`
Expected: 警告なし

**Step 4: Format確認**
Run: `cargo fmt -- --check`
Expected: 差分なし

**Step 5: grep で残存 "session" を確認**
Run: `grep -ri 'session' src/ --include='*.rs'`
Expected: pty.rs の `PtySession` のみ（これはPTYライブラリの概念なのでそのまま）

**Step 6: 最終コミット（必要な修正があった場合）**
