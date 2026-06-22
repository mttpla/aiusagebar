# Provider Error Boundary Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Guarantee that every provider ending a fetch in a non-happy state (`Error` or `Stale`) leaves a trace in the diagnostic log, by logging once at the `refresh_all` boundary.

**Architecture:** A pure `state_diag_message(name, &UsageState) -> Option<String>` helper in `src/provider/mod.rs` decides what to log; `App::refresh_all` in `src/main.rs` calls it per fetched provider and pushes the message through the existing `diag!` macro. No new state, no dedup.

**Tech Stack:** Rust, existing `crate::diag` in-memory log, `cargo test` / `cargo clippy`.

## Global Constraints

- All `.rs` string literals must be English (runtime i18n is separate).
- No bare `pub` in this binary crate — default `pub(crate)` / private.
- Never add a `Co-Authored-By` trailer to commit messages.
- Run `cargo clippy -- -D warnings && cargo test` before every commit.
- No `#[allow(dead_code)]` — delete unused symbols instead.
- Tokens read-only; this feature touches no auth/network code.

---

### Task 1: `state_diag_message` decision helper

**Files:**
- Modify: `src/provider/mod.rs` (add free function after the `ProviderKind` impl block, ~line 35; add tests to the existing `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `UsageState` (existing enum, `src/provider/mod.rs:14-20`).
- Produces: `pub(crate) fn state_diag_message(name: &str, state: &UsageState) -> Option<String>`.

- [ ] **Step 1: Write the failing tests**

Add to the `mod tests` block in `src/provider/mod.rs`:

```rust
    #[test]
    fn diag_message_error_includes_name_and_msg() {
        let s = UsageState::Error("boom".to_string());
        assert_eq!(state_diag_message("Claude", &s), Some("Claude: boom".to_string()));
    }

    #[test]
    fn diag_message_stale_includes_name_and_msg() {
        let s = UsageState::Stale("Expired on 2026-06-17 — run: claude login".to_string());
        assert_eq!(
            state_diag_message("Claude", &s),
            Some("Claude: Expired on 2026-06-17 — run: claude login".to_string())
        );
    }

    #[test]
    fn diag_message_ok_is_none() {
        let s = UsageState::Ok(vec![], Some("max".to_string()));
        assert_eq!(state_diag_message("Claude", &s), None);
    }

    #[test]
    fn diag_message_not_configured_is_none() {
        assert_eq!(state_diag_message("Copilot", &UsageState::NotConfigured), None);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib provider::tests::diag_message`
Expected: FAIL — `cannot find function state_diag_message in this scope`.

- [ ] **Step 3: Write minimal implementation**

Insert into `src/provider/mod.rs` immediately after the closing `}` of `impl ProviderKind` (after line 35), before the `UsageProvider` trait:

```rust
/// Returns the diagnostic message to log for a provider that ended a fetch in a
/// non-happy state (`Error` or `Stale`), or `None` for happy/neutral states
/// (`Ok`, `NotConfigured`). Pure — used by the `refresh_all` boundary so every
/// provider error leaves a diagnostic trace without per-leaf instrumentation.
pub(crate) fn state_diag_message(name: &str, state: &UsageState) -> Option<String> {
    match state {
        UsageState::Error(msg) | UsageState::Stale(msg) => Some(format!("{}: {}", name, msg)),
        UsageState::Ok(..) | UsageState::NotConfigured => None,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib provider::tests::diag_message`
Expected: PASS (4 tests).

- [ ] **Step 5: Quality gate + commit**

```bash
cargo clippy -- -D warnings && cargo test
git add src/provider/mod.rs
git commit -m "feat(diag): add state_diag_message boundary helper"
```

---

### Task 2: Log provider non-happy states in `refresh_all`

**Files:**
- Modify: `src/main.rs` (inside `App::refresh_all`, the provider loop, ~lines 72-82)

**Interfaces:**
- Consumes: `crate::provider::state_diag_message` (Task 1); `ProviderKind::display_name` (`src/provider/mod.rs:29`); `crate::diag!` macro; `crate::diag::Level::Err`.
- Produces: no new public symbol — behavioral wiring only.

- [ ] **Step 1: Add the boundary log call**

In `src/main.rs`, inside the `for i in 0..count` loop of `refresh_all`, after the line
`let (state, http_err) = self.providers[i].fetch_with_http_error();` (currently line 72)
and before the `let b = self.backoff.get_mut(...)` line, insert:

```rust
            if let Some(msg) = crate::provider::state_diag_message(kind.display_name(), &state) {
                crate::diag!(crate::diag::Level::Err, "{}", msg);
            }
```

`kind` is already bound earlier in the loop (`let kind = self.providers[i].kind();`).

- [ ] **Step 2: Verify it compiles and existing tests still pass**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: PASS — no warnings, all existing tests green. (`refresh_all` has no unit test;
it drives the tray. Correctness here is the trivial `if let Some` wiring around the
Task 1 function, which is unit-tested.)

- [ ] **Step 3: Manual smoke check (optional, requires expired/erroring token)**

With an expired Claude token, run `make dev`, open the menu: the header shows Claude in
Stale, and "Other ▶ Diagnostics ▶ Copy diagnostic log" now appears. Copying yields a line
like `[HH:MM:SS ERR] [src/main.rs:NN] Claude: Expired on … — run: claude login`.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat(diag): log provider Error/Stale states at refresh_all boundary"
```

---

## Self-Review

**Spec coverage:**
- "Decision function mapping name + state to optional message" → Task 1. ✓
- "Single call site in refresh_all" → Task 2. ✓
- "Both Error and Stale at Level::Err" → Task 1 impl + Task 2 `Level::Err`. ✓
- "Ok / NotConfigured not logged" → Task 1 returns `None`, tested. ✓
- "Skipped (backoff) providers not logged" → call sits after the `if !force && !is_allowed { continue; }` guard, so skipped providers never reach it. ✓
- Out-of-scope items (non-provider paths, menu disappearance, persistence, dedup) → no tasks, correct. ✓

**Placeholder scan:** `NN` in the smoke-check line is an illustrative line number, not a code placeholder. No TBD/TODO in code steps. ✓

**Type consistency:** `state_diag_message(&str, &UsageState) -> Option<String>` used identically in both tasks; `ProviderKind::display_name(&self) -> &'static str` and `diag!(Level, fmt)` match existing signatures. ✓
