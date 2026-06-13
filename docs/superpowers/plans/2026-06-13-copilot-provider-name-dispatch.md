# Copilot Provider Name + ProviderKind Dispatch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace stringly-typed provider dispatch with a `ProviderKind` enum so the Copilot section stops rendering "GitHub: unknown provider" and so future rename drift becomes a compile-time error.

**Architecture:** Hoist the existing `ProviderKind` enum from `src/ui/mod.rs` into `src/provider/mod.rs`. Trait method changes from `name() -> &'static str` to `kind() -> ProviderKind`. Display label lives on the enum as `display_name()`. UI dispatch (`build_menu`, `build_layout`) matches the enum exhaustively — no fallback arm.

**Tech Stack:** Rust 2021, `tray-icon`, `winit`. Tests via `cargo test`.

**Kanban card:** `docs/kanban/copilot-provider-name-dispatch.md` (id 33).

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `src/provider/mod.rs` | Modify | Owns `ProviderKind` enum + `display_name()`. Trait surface: `kind()`. |
| `src/ui/mod.rs` | Modify | Re-import `ProviderKind` from `provider::`. `build_menu`/`build_layout` take `&[(ProviderKind, &UsageState)]`. Exhaustive match. |
| `src/provider/claude.rs` | Modify | `UsageProvider::kind()` returns `ProviderKind::Claude`. |
| `src/provider/copilot.rs` | Modify | `UsageProvider::kind()` returns `ProviderKind::Copilot` (was `name() == "GitHub"`). |
| `src/main.rs` | Modify | Refresh + initial_refs pass `p.kind()` instead of `p.name()`. |
| `src/ui/styled.rs` | Modify | Update `use super::{MenuLayout, ProviderKind}` → `use crate::provider::ProviderKind`. |
| `README.md` | Modify | Line 15 table row "GitHub" → "Copilot". |

---

## Task 1: Add failing tests for ProviderKind + trait `kind()`

**Files:**
- Modify: `src/provider/mod.rs`

- [ ] **Step 1: Append failing tests to `src/provider/mod.rs`**

Replace the existing `#[cfg(test)] mod tests` block (currently at the bottom of the file) with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::claude::ClaudeProvider;
    use crate::provider::copilot::CopilotProvider;

    #[test]
    fn limit_window_default() {
        let w = LimitWindow::default();
        assert_eq!(w.name, "");
        assert!(w.percent_used.is_none());
    }

    #[test]
    fn provider_kind_display_name_claude() {
        assert_eq!(ProviderKind::Claude.display_name(), "Claude");
    }

    #[test]
    fn provider_kind_display_name_copilot() {
        assert_eq!(ProviderKind::Copilot.display_name(), "Copilot");
    }

    #[test]
    fn claude_provider_kind_is_claude() {
        let p = ClaudeProvider::new();
        assert_eq!(p.kind(), ProviderKind::Claude);
    }

    #[test]
    fn copilot_provider_kind_is_copilot() {
        let p = CopilotProvider::new();
        assert_eq!(p.kind(), ProviderKind::Copilot);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail to compile**

Run: `cargo test --lib provider::tests 2>&1 | tail -20`
Expected: compile errors. `ProviderKind` not found in `crate::provider`; `kind` method not on trait. This is intentional — failing tests prove they exercise the new contract.

---

## Task 2: Hoist `ProviderKind` into `provider::mod` and add `display_name()`

**Files:**
- Modify: `src/provider/mod.rs`

- [ ] **Step 1: Add enum + display_name() and change trait signature**

In `src/provider/mod.rs`, after the `UsageState` enum and before the `UsageProvider` trait, insert:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Claude,
    Copilot,
}

impl ProviderKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderKind::Claude => "Claude",
            ProviderKind::Copilot => "Copilot",
        }
    }
}
```

Then change the trait:

```rust
pub trait UsageProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn fetch(&self) -> UsageState;
}
```

- [ ] **Step 2: Run provider tests — still failing**

Run: `cargo test --lib provider::tests 2>&1 | tail -20`
Expected: provider/mod tests now compile; `ClaudeProvider`/`CopilotProvider` still expose `name()` so their tests fail with "no method `kind`". Crate as a whole won't build yet because impls still implement old trait. Acceptable at this checkpoint.

---

## Task 3: Update ClaudeProvider and CopilotProvider impls

**Files:**
- Modify: `src/provider/claude.rs:224-226`
- Modify: `src/provider/copilot.rs:125-128`

- [ ] **Step 1: Update ClaudeProvider impl**

In `src/provider/claude.rs`, replace:

```rust
impl UsageProvider for ClaudeProvider {
    fn name(&self) -> &'static str { "Claude" }
```

with:

```rust
impl UsageProvider for ClaudeProvider {
    fn kind(&self) -> crate::provider::ProviderKind { crate::provider::ProviderKind::Claude }
```

(Leave the `fetch` body unchanged.)

- [ ] **Step 2: Update CopilotProvider impl**

In `src/provider/copilot.rs`, replace:

```rust
impl crate::provider::UsageProvider for CopilotProvider {
    fn name(&self) -> &'static str {
        "GitHub"
    }
```

with:

```rust
impl crate::provider::UsageProvider for CopilotProvider {
    fn kind(&self) -> crate::provider::ProviderKind {
        crate::provider::ProviderKind::Copilot
    }
```

(Leave the `fetch` body unchanged.)

- [ ] **Step 3: Run provider tests — now passing**

Run: `cargo test --lib provider::tests 2>&1 | tail -20`
Expected: 5 provider/mod tests pass. The full crate still does not build — UI + main.rs are next.

---

## Task 4: Migrate UI dispatch to exhaustive `ProviderKind` match

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Remove local ProviderKind, import from provider**

In `src/ui/mod.rs`, at the top, change imports:

```rust
use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
use crate::provider::{LimitWindow, ProviderKind, UsageState};
```

Then delete the entire local enum block:

```rust
#[derive(Debug)]
pub(crate) enum ProviderKind {
    Claude,
    Copilot,
}
```

- [ ] **Step 2: Change `build_layout` signature + body to use ProviderKind exhaustively**

In `src/ui/mod.rs`, replace the existing `build_layout` function (lines 32-73) with:

```rust
/// Pure index-tracking function — does NOT build the actual Menu.
/// Uses section_item_count from claude/copilot modules to count items per section.
pub(crate) fn build_layout(
    states: &[(ProviderKind, &UsageState)],
    last_updated: Option<&str>,
) -> MenuLayout {
    let mut idx: usize = 2; // About(0) + separator(1)
    let mut header_indices: Vec<(usize, ProviderKind)> = Vec::new();
    let mut window_items: Vec<(usize, LimitWindow)> = Vec::new();

    for (kind, state) in states {
        header_indices.push((idx, *kind));
        if let UsageState::Ok(windows, _) = state {
            for (i, w) in windows.iter().enumerate() {
                window_items.push((idx + 1 + i, w.clone()));
            }
        }
        idx += match kind {
            ProviderKind::Claude => claude::section_item_count(state),
            ProviderKind::Copilot => copilot::section_item_count(state),
        };
    }

    MenuLayout {
        header_indices,
        window_items,
        refresh_idx: idx,
        quit_idx: idx + 1,
        last_updated: last_updated.map(str::to_owned),
    }
}
```

- [ ] **Step 3: Change `build_menu` signature + body to use ProviderKind exhaustively**

In `src/ui/mod.rs`, replace the `build_menu` function (lines 80-108) with:

```rust
pub fn build_menu(states: &[(ProviderKind, &UsageState)], last_updated: Option<&str>) -> MenuBuild {
    let menu = Menu::new();
    let item_about = MenuItem::new("About AIUsageBar", true, None);
    menu.append(&item_about).expect("menu append failed");
    menu.append(&PredefinedMenuItem::separator())
        .expect("menu append failed");
    for (kind, state) in states {
        match kind {
            ProviderKind::Claude => { let _ = claude::append_claude_section(&menu, state); }
            ProviderKind::Copilot => { let _ = copilot::append_copilot_section(&menu, state); }
        }
    }
    let footer = base::append_footer(&menu);
    let layout = build_layout(states, last_updated);

    #[cfg(target_os = "macos")]
    styled::style_menu(&menu, &layout);

    #[cfg(not(target_os = "macos"))]
    let _ = layout;

    MenuBuild {
        menu,
        about: item_about.id().clone(),
        refresh: footer.refresh,
        quit: footer.quit,
    }
}
```

- [ ] **Step 4: Migrate existing `build_layout` tests in `src/ui/mod.rs`**

In the `#[cfg(test)] mod tests` block at the bottom of `src/ui/mod.rs`, replace string keys with enum variants. Specifically:

```rust
#[test]
fn menu_layout_indices_claude_two_windows() {
    let state = UsageState::Ok(
        vec![
            LimitWindow { name: "d".into(), ..Default::default() },
            LimitWindow { name: "m".into(), ..Default::default() },
        ],
        Some("max".into()),
    );
    let layout = build_layout(&[(ProviderKind::Claude, &state)], None);
    assert_eq!(layout.header_indices[0].0, 2);
    assert_eq!(layout.refresh_idx, 5);
    assert_eq!(layout.quit_idx, 6);
}

#[test]
fn build_layout_claude_window_items_indices() {
    let state = UsageState::Ok(
        vec![
            LimitWindow { name: "5h session".into(), percent_used: Some(39.0), ..Default::default() },
            LimitWindow { name: "7d weekly".into(), percent_used: Some(15.0), ..Default::default() },
        ],
        Some("max".into()),
    );
    let layout = build_layout(&[(ProviderKind::Claude, &state)], None);
    assert_eq!(layout.window_items.len(), 2);
    assert_eq!(layout.window_items[0].0, 3);
    assert_eq!(layout.window_items[1].0, 4);
    assert_eq!(layout.window_items[0].1.name, "5h session");
    assert_eq!(layout.window_items[1].1.name, "7d weekly");
}

#[test]
fn build_layout_copilot_window_items_indices() {
    use crate::provider::LimitWindow;
    let claude_state = UsageState::Ok(
        vec![
            LimitWindow { name: "5h session".into(), ..Default::default() },
            LimitWindow { name: "7d weekly".into(), ..Default::default() },
        ],
        Some("max".into()),
    );
    let copilot_state = UsageState::Ok(
        vec![LimitWindow { name: "monthly".into(), ..Default::default() }],
        None,
    );
    let layout = build_layout(
        &[(ProviderKind::Claude, &claude_state), (ProviderKind::Copilot, &copilot_state)],
        None,
    );
    assert_eq!(layout.window_items.len(), 3);
    assert_eq!(layout.window_items[0].0, 3);
    assert_eq!(layout.window_items[1].0, 4);
    assert_eq!(layout.window_items[2].0, 6);
    assert_eq!(layout.window_items[2].1.name, "monthly");
    assert_eq!(layout.refresh_idx, 7);
}

#[test]
fn build_layout_non_ok_state_no_window_items() {
    let layout = build_layout(&[(ProviderKind::Claude, &UsageState::NotConfigured)], None);
    assert!(layout.window_items.is_empty());
}
```

(`menu_layout_indices_no_providers` does not need changes — it passes `&[]`.)

- [ ] **Step 5: Run ui tests — should pass standalone, but full crate still doesn't build**

Run: `cargo test --lib ui::tests 2>&1 | tail -30`
Expected: compile errors from `src/main.rs` (still calls `p.name()`). UI module itself is consistent.

---

## Task 5: Migrate `main.rs` callers + verify `styled.rs` import

**Files:**
- Modify: `src/main.rs:45`
- Modify: `src/main.rs:117-120`
- Modify: `src/ui/styled.rs:14`

- [ ] **Step 1: Update `App::refresh` body**

In `src/main.rs`, replace lines 43-52:

```rust
        let states: Vec<(&str, UsageState)> = self.providers
            .iter()
            .map(|p| (p.name(), p.fetch()))
            .collect();

        let state_refs: Vec<&UsageState> = states.iter().map(|(_, s)| s).collect();
        let icon_kind = IconKind::for_providers(&state_refs, self.settings.alert_threshold_pct);

        let refs: Vec<(&str, &UsageState)> =
            states.iter().map(|(n, s)| (*n, s)).collect();
```

with:

```rust
        let states: Vec<(ProviderKind, UsageState)> = self.providers
            .iter()
            .map(|p| (p.kind(), p.fetch()))
            .collect();

        let state_refs: Vec<&UsageState> = states.iter().map(|(_, s)| s).collect();
        let icon_kind = IconKind::for_providers(&state_refs, self.settings.alert_threshold_pct);

        let refs: Vec<(ProviderKind, &UsageState)> =
            states.iter().map(|(k, s)| (*k, s)).collect();
```

(`ProviderKind` is `Copy`, so `*k` works.)

Then ensure `ProviderKind` is in scope at the top of `main.rs`. Find line 17:

```rust
use provider::{UsageProvider, UsageState};
```

Replace with:

```rust
use provider::{ProviderKind, UsageProvider, UsageState};
```

- [ ] **Step 2: Update `initial_refs` builder**

In `src/main.rs`, replace lines 117-120:

```rust
    let initial_refs: Vec<(&str, &UsageState)> = providers
        .iter()
        .map(|p| (p.name(), &initial_state))
        .collect();
```

with:

```rust
    let initial_refs: Vec<(ProviderKind, &UsageState)> = providers
        .iter()
        .map(|p| (p.kind(), &initial_state))
        .collect();
```

- [ ] **Step 3: Fix `styled.rs` import path**

In `src/ui/styled.rs`, replace line 14:

```rust
use super::{MenuLayout, ProviderKind};
```

with:

```rust
use super::MenuLayout;
use crate::provider::ProviderKind;
```

(No code body changes — `MenuLayout::header_indices` already holds `ProviderKind` and styled.rs already matches it exhaustively at lines 280-281.)

- [ ] **Step 4: Build the whole crate**

Run: `cargo build 2>&1 | tail -20`
Expected: build succeeds. No `name()` / "unknown provider" references remain.

- [ ] **Step 5: Run full test suite**

Run: `cargo test 2>&1 | tail -30`
Expected: all tests pass (the 5 new provider tests + all existing tests including the 4 migrated build_layout tests).

---

## Task 6: README provider name rename

**Files:**
- Modify: `README.md:15`

- [ ] **Step 1: Update provider table row**

In `README.md`, locate the providers table (around lines 9-15). Change:

```markdown
| GitHub     | Monthly premium quota (per account)    |
```

to:

```markdown
| Copilot    | Monthly premium quota (per account)    |
```

Leave line 3 ("**OpenAI**, **Anthropic**, and **GitHub** Copilot") unchanged — "GitHub Copilot" is the official product name in prose. Leave line 37 ("Copilot Keychain prompt"), line 40 (`COPILOT_GITHUB_TOKEN`), and line 49 ("Copilot PAT") unchanged.

- [ ] **Step 2: Verify no other stale "GitHub" provider-name references**

Run: `grep -n "^| GitHub\|GitHub.*Monthly\|GitHub provider\|name == \"GitHub\"" README.md src/`
Expected: no matches.

---

## Task 7: Final verification + single commit

**Files:**
- All previously modified.

- [ ] **Step 1: Verification**

Run: `cargo check && cargo test && cargo clippy -- -D warnings`
Expected: no errors, no warnings, all tests pass.

- [ ] **Step 2: Confirm dispatch fallback is gone**

Run: `grep -rn "unknown provider" src/`
Expected: no matches.

- [ ] **Step 3: Update kanban card status to `done` and append closing narrative**

In `docs/kanban/copilot-provider-name-dispatch.md`:
- Change frontmatter `status: doing` → `status: done`.
- Update `updated:` to today (`2026-06-13`).
- Append to Narrative:

```markdown
- 2026-06-13: DONE. Trait now exposes `kind() -> ProviderKind`. UI dispatch
  exhaustive — fallback arm removed. Copilot section renders "Copilot" header.
  README provider table normalised. All tests pass, no clippy warnings.
```

- [ ] **Step 4: Stage and commit**

```bash
git add src/provider/mod.rs src/provider/claude.rs src/provider/copilot.rs \
        src/ui/mod.rs src/ui/styled.rs src/main.rs \
        README.md \
        docs/kanban/copilot-provider-name-dispatch.md \
        docs/superpowers/plans/2026-06-13-copilot-provider-name-dispatch.md
git commit -m "fix(provider): dispatch on ProviderKind enum, restore Copilot section

CopilotProvider::name() returned 'GitHub' while UI dispatch matched
'Copilot' — tray menu rendered 'GitHub: unknown provider'. Replaced
stringly-typed dispatch with exhaustive ProviderKind enum match;
renamed README provider row to match."
```

(No `Co-Authored-By` trailer per project memory.)
