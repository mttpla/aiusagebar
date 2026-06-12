# UI Module Restructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract all menu-building code from `src/main.rs` into a new `src/ui/` module — pure refactor, no new ObjC2 code.

**Architecture:** Four new files under `src/ui/`: `mod.rs` (public API + routing), `claude.rs` (Claude section renderer), `copilot.rs` (Copilot section renderer), `base.rs` (footer helpers). `main.rs` drops `MenuBuild`/`append_label`/`build_menu` and calls `ui::build_menu` instead.

**Tech Stack:** Rust, `tray-icon` crate (`Menu`, `MenuItem`, `MenuId`, `PredefinedMenuItem`), `crate::provider::{UsageState, LimitWindow}`.

---

## File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `src/ui/mod.rs` | `MenuBuild`, `append_label`, `build_menu`, module declarations |
| Create | `src/ui/claude.rs` | `append_claude_section`, `header_label`, `pct_label` + unit tests |
| Create | `src/ui/copilot.rs` | `append_copilot_section`, `row_label` + unit tests |
| Create | `src/ui/base.rs` | `FooterIds`, `append_footer`, `refresh_label` + unit tests |
| Modify | `src/main.rs` | Add `mod ui;`, remove extracted code, use `ui::build_menu` |

---

### Task 1: Scaffold `src/ui/` skeleton

**Files:**
- Create: `src/ui/mod.rs`
- Create: `src/ui/claude.rs` (stub)
- Create: `src/ui/copilot.rs` (stub)
- Create: `src/ui/base.rs` (stub)
- Modify: `src/main.rs` (add `mod ui;`)

- [ ] **Step 1: Create `src/ui/mod.rs`**

```rust
use tray_icon::menu::{Menu, MenuId, MenuItem};
use crate::provider::UsageState;

pub mod base;
pub mod claude;
pub mod copilot;

pub struct MenuBuild {
    pub menu: Menu,
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
}

pub(crate) fn append_label(menu: &Menu, text: impl Into<String>) {
    menu.append(&MenuItem::new(text.into(), false, None))
        .expect("menu append failed");
}

pub fn build_menu(_states: &[(&str, &UsageState)], _last_updated: Option<&str>) -> MenuBuild {
    todo!()
}
```

- [ ] **Step 2: Create `src/ui/claude.rs` (stub)**

```rust
use tray_icon::menu::Menu;
use crate::provider::UsageState;

pub(crate) fn header_label(_name: &str, _state: &UsageState) -> String {
    todo!()
}

pub(crate) fn pct_label(_pct: Option<f32>) -> String {
    todo!()
}

pub fn append_claude_section(_menu: &Menu, _state: &UsageState) {
    todo!()
}
```

- [ ] **Step 3: Create `src/ui/copilot.rs` (stub)**

```rust
use tray_icon::menu::Menu;
use crate::provider::{LimitWindow, UsageState};

pub(crate) fn row_label(_window: &LimitWindow) -> String {
    todo!()
}

pub fn append_copilot_section(_menu: &Menu, _state: &UsageState) {
    todo!()
}
```

- [ ] **Step 4: Create `src/ui/base.rs` (stub)**

```rust
use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};

pub struct FooterIds {
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
}

pub(crate) fn refresh_label(_updated: Option<&str>) -> Option<String> {
    todo!()
}

pub fn append_footer(_menu: &Menu, _updated: Option<&str>) -> FooterIds {
    todo!()
}
```

- [ ] **Step 5: Add `mod ui;` to `src/main.rs`**

In `src/main.rs`, add after the existing `mod provider;` declaration:

```rust
mod ui;
```

- [ ] **Step 6: Verify it compiles (with warnings)**

```bash
cargo check 2>&1 | grep -E "^error"
```

Expected: no output (zero errors). Warnings about `todo!()` or unused imports are fine.

- [ ] **Step 7: Commit scaffold**

```bash
git add src/ui/mod.rs src/ui/claude.rs src/ui/copilot.rs src/ui/base.rs src/main.rs
git commit -m "feat(ui): scaffold src/ui/ module structure"
```

---

### Task 2: `src/ui/claude.rs` — label helpers and Claude section

**Files:**
- Modify: `src/ui/claude.rs`

- [ ] **Step 1: Write failing tests**

Append to `src/ui/claude.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::UsageState;

    fn make_ok(profile: Option<&str>) -> UsageState {
        UsageState::Ok(vec![], profile.map(str::to_owned))
    }

    #[test]
    fn header_ok_with_profile() {
        assert_eq!(header_label("Claude", &make_ok(Some("max"))), "Claude — max");
    }

    #[test]
    fn header_ok_no_profile() {
        assert_eq!(header_label("Claude", &make_ok(None)), "Claude — account unavailable");
    }

    #[test]
    fn header_stale() {
        assert_eq!(
            header_label("Claude", &UsageState::Stale("token expired".into())),
            "Claude ⚠  token expired"
        );
    }

    #[test]
    fn header_error() {
        assert_eq!(
            header_label("Claude", &UsageState::Error("network failure".into())),
            "Claude ✕  network failure"
        );
    }

    #[test]
    fn header_not_configured() {
        assert_eq!(
            header_label("Claude", &UsageState::NotConfigured),
            "Claude: not configured"
        );
    }

    #[test]
    fn pct_some() {
        assert_eq!(pct_label(Some(42.5)), "42.5%");
    }

    #[test]
    fn pct_none() {
        assert_eq!(pct_label(None), "—");
    }
}
```

- [ ] **Step 2: Run tests — expect fail**

```bash
cargo test ui::claude::tests 2>&1 | tail -15
```

Expected: panics with `not yet implemented` (todos fire at runtime).

- [ ] **Step 3: Implement `header_label`, `pct_label`, `append_claude_section`**

Replace the entire `src/ui/claude.rs` with:

```rust
use tray_icon::menu::Menu;
use crate::provider::UsageState;

pub(crate) fn header_label(name: &str, state: &UsageState) -> String {
    match state {
        UsageState::Ok(_, Some(p)) => format!("{} — {}", name, p),
        UsageState::Ok(_, None) => format!("{} — account unavailable", name),
        UsageState::Stale(msg) => format!("{} ⚠  {}", name, msg),
        UsageState::Error(msg) => format!("{} ✕  {}", name, msg),
        UsageState::NotConfigured => format!("{}: not configured", name),
    }
}

pub(crate) fn pct_label(pct: Option<f32>) -> String {
    pct.map(|p| format!("{:.1}%", p))
        .unwrap_or_else(|| "—".to_string())
}

pub fn append_claude_section(menu: &Menu, state: &UsageState) {
    super::append_label(menu, header_label("Claude", state));
    if let UsageState::Ok(windows, _) = state {
        for w in windows {
            let reset = w.resets_at.as_deref().unwrap_or("?");
            super::append_label(
                menu,
                format!("  {} — {}  resets {}", w.name, pct_label(w.percent_used), reset),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::UsageState;

    fn make_ok(profile: Option<&str>) -> UsageState {
        UsageState::Ok(vec![], profile.map(str::to_owned))
    }

    #[test]
    fn header_ok_with_profile() {
        assert_eq!(header_label("Claude", &make_ok(Some("max"))), "Claude — max");
    }

    #[test]
    fn header_ok_no_profile() {
        assert_eq!(header_label("Claude", &make_ok(None)), "Claude — account unavailable");
    }

    #[test]
    fn header_stale() {
        assert_eq!(
            header_label("Claude", &UsageState::Stale("token expired".into())),
            "Claude ⚠  token expired"
        );
    }

    #[test]
    fn header_error() {
        assert_eq!(
            header_label("Claude", &UsageState::Error("network failure".into())),
            "Claude ✕  network failure"
        );
    }

    #[test]
    fn header_not_configured() {
        assert_eq!(
            header_label("Claude", &UsageState::NotConfigured),
            "Claude: not configured"
        );
    }

    #[test]
    fn pct_some() {
        assert_eq!(pct_label(Some(42.5)), "42.5%");
    }

    #[test]
    fn pct_none() {
        assert_eq!(pct_label(None), "—");
    }
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test ui::claude::tests 2>&1 | tail -15
```

Expected: `test result: ok. 7 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add src/ui/claude.rs
git commit -m "feat(ui/claude): implement header_label, pct_label, append_claude_section"
```

---

### Task 3: `src/ui/copilot.rs` — Copilot section renderer

**Files:**
- Modify: `src/ui/copilot.rs`

- [ ] **Step 1: Write failing tests**

Replace `src/ui/copilot.rs` with tests + stubs:

```rust
use tray_icon::menu::Menu;
use crate::provider::{LimitWindow, UsageState};

pub(crate) fn row_label(window: &LimitWindow) -> String {
    todo!()
}

pub fn append_copilot_section(_menu: &Menu, _state: &UsageState) {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::LimitWindow;

    fn make_window(name: &str, pct: Option<f32>, resets_at: Option<&str>) -> LimitWindow {
        LimitWindow {
            name: name.to_owned(),
            percent_used: pct,
            limit: None,
            remaining: None,
            resets_at: resets_at.map(str::to_owned),
            unlimited: false,
        }
    }

    #[test]
    fn row_with_pct_and_reset() {
        let w = make_window("Daily", Some(42.5), Some("2026-06-13 00:00"));
        assert_eq!(row_label(&w), "  Daily — 42.5%  resets 2026-06-13 00:00");
    }

    #[test]
    fn row_no_pct_no_reset() {
        let w = make_window("Daily", None, None);
        assert_eq!(row_label(&w), "  Daily — —  resets ?");
    }
}
```

- [ ] **Step 2: Run tests — expect fail**

```bash
cargo test ui::copilot::tests 2>&1 | tail -15
```

Expected: panics with `not yet implemented`.

- [ ] **Step 3: Implement `row_label` and `append_copilot_section`**

Replace the entire `src/ui/copilot.rs` with:

```rust
use tray_icon::menu::Menu;
use crate::provider::{LimitWindow, UsageState};

pub(crate) fn row_label(window: &LimitWindow) -> String {
    let pct = window
        .percent_used
        .map(|p| format!("{:.1}%", p))
        .unwrap_or_else(|| "—".to_string());
    let reset = window.resets_at.as_deref().unwrap_or("?");
    format!("  {} — {}  resets {}", window.name, pct, reset)
}

pub fn append_copilot_section(menu: &Menu, state: &UsageState) {
    match state {
        UsageState::NotConfigured => {
            super::append_label(menu, "Copilot: not configured");
        }
        UsageState::Stale(msg) => {
            super::append_label(menu, format!("Copilot ⚠  {}", msg));
        }
        UsageState::Error(msg) => {
            super::append_label(menu, format!("Copilot ✕  {}", msg));
        }
        UsageState::Ok(windows, profile) => {
            let header = match profile {
                Some(p) => format!("Copilot — {}", p),
                None => "Copilot — account unavailable".to_string(),
            };
            super::append_label(menu, header);
            for w in windows {
                super::append_label(menu, row_label(w));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::LimitWindow;

    fn make_window(name: &str, pct: Option<f32>, resets_at: Option<&str>) -> LimitWindow {
        LimitWindow {
            name: name.to_owned(),
            percent_used: pct,
            limit: None,
            remaining: None,
            resets_at: resets_at.map(str::to_owned),
            unlimited: false,
        }
    }

    #[test]
    fn row_with_pct_and_reset() {
        let w = make_window("Daily", Some(42.5), Some("2026-06-13 00:00"));
        assert_eq!(row_label(&w), "  Daily — 42.5%  resets 2026-06-13 00:00");
    }

    #[test]
    fn row_no_pct_no_reset() {
        let w = make_window("Daily", None, None);
        assert_eq!(row_label(&w), "  Daily — —  resets ?");
    }
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test ui::copilot::tests 2>&1 | tail -15
```

Expected: `test result: ok. 2 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add src/ui/copilot.rs
git commit -m "feat(ui/copilot): implement row_label, append_copilot_section"
```

---

### Task 4: `src/ui/base.rs` — footer helpers

**Files:**
- Modify: `src/ui/base.rs`

Footer layout (per spec): optional "Updated: {ts}" label → separator → Refresh → separator → About AIUsageBar → Quit.

- [ ] **Step 1: Write failing tests**

Replace `src/ui/base.rs` with tests + stubs:

```rust
use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};

pub struct FooterIds {
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
}

pub(crate) fn refresh_label(updated: Option<&str>) -> Option<String> {
    todo!()
}

pub fn append_footer(_menu: &Menu, _updated: Option<&str>) -> FooterIds {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_label_some() {
        assert_eq!(refresh_label(Some("12:34")), Some("Updated: 12:34".to_string()));
    }

    #[test]
    fn refresh_label_none() {
        assert_eq!(refresh_label(None), None);
    }
}
```

- [ ] **Step 2: Run tests — expect fail**

```bash
cargo test ui::base::tests 2>&1 | tail -15
```

Expected: panics with `not yet implemented`.

- [ ] **Step 3: Implement `refresh_label` and `append_footer`**

Replace the entire `src/ui/base.rs` with:

```rust
use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};

pub struct FooterIds {
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
}

pub(crate) fn refresh_label(updated: Option<&str>) -> Option<String> {
    updated.map(|ts| format!("Updated: {}", ts))
}

pub fn append_footer(menu: &Menu, updated: Option<&str>) -> FooterIds {
    if let Some(label) = refresh_label(updated) {
        super::append_label(menu, label);
    }
    menu.append(&PredefinedMenuItem::separator())
        .expect("menu append failed");
    let item_refresh = MenuItem::new("Refresh", true, None);
    menu.append(&item_refresh).expect("menu append failed");
    menu.append(&PredefinedMenuItem::separator())
        .expect("menu append failed");
    let item_about = MenuItem::new("About AIUsageBar", true, None);
    let item_quit = MenuItem::new("Quit", true, None);
    menu.append(&item_about).expect("menu append failed");
    menu.append(&item_quit).expect("menu append failed");
    FooterIds {
        about: item_about.id().clone(),
        refresh: item_refresh.id().clone(),
        quit: item_quit.id().clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_label_some() {
        assert_eq!(refresh_label(Some("12:34")), Some("Updated: 12:34".to_string()));
    }

    #[test]
    fn refresh_label_none() {
        assert_eq!(refresh_label(None), None);
    }
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test ui::base::tests 2>&1 | tail -15
```

Expected: `test result: ok. 2 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add src/ui/base.rs
git commit -m "feat(ui/base): implement refresh_label, append_footer"
```

---

### Task 5: Complete `src/ui/mod.rs` — `build_menu` routing

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Implement `build_menu`**

Replace `src/ui/mod.rs` with the final version:

```rust
use tray_icon::menu::{Menu, MenuId, MenuItem};
use crate::provider::UsageState;

pub mod base;
pub mod claude;
pub mod copilot;

pub struct MenuBuild {
    pub menu: Menu,
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
}

pub(crate) fn append_label(menu: &Menu, text: impl Into<String>) {
    menu.append(&MenuItem::new(text.into(), false, None))
        .expect("menu append failed");
}

pub fn build_menu(states: &[(&str, &UsageState)], last_updated: Option<&str>) -> MenuBuild {
    let menu = Menu::new();
    for (name, state) in states {
        match *name {
            "Claude" => claude::append_claude_section(&menu, state),
            "Copilot" => copilot::append_copilot_section(&menu, state),
            _ => append_label(&menu, format!("{}: unknown provider", name)),
        }
    }
    let footer = base::append_footer(&menu, last_updated);
    MenuBuild {
        menu,
        about: footer.about,
        refresh: footer.refresh,
        quit: footer.quit,
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check 2>&1 | grep -E "^error"
```

Expected: no output (zero errors).

- [ ] **Step 3: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat(ui): implement build_menu with provider routing"
```

---

### Task 6: Update `src/main.rs` — wire up `ui::build_menu`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Remove `MenuBuild`, `append_label`, `App::build_menu` from `main.rs`**

Delete these three blocks from `src/main.rs`:

```rust
// DELETE: lines 28–33
struct MenuBuild {
    menu: Menu,
    about: tray_icon::menu::MenuId,
    refresh: tray_icon::menu::MenuId,
    quit: tray_icon::menu::MenuId,
}

// DELETE: lines 35–38
fn append_label(menu: &Menu, text: impl Into<String>) {
    menu.append(&MenuItem::new(text.into(), false, None))
        .expect("menu append failed");
}

// DELETE: lines 52–108 (entire impl App { fn build_menu ... } block)
impl App {
    fn build_menu(states: &[(&str, &UsageState)], last_updated: Option<&str>) -> MenuBuild {
        ...
    }
    ...
}
```

After deletion, `impl App` only contains `fn refresh(...)`.

- [ ] **Step 2: Update the `tray_icon` import**

Replace:
```rust
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIconBuilder, TrayIconEvent,
};
```

With:
```rust
use tray_icon::{
    menu::MenuEvent,
    TrayIconBuilder, TrayIconEvent,
};
```

(`Menu`, `MenuItem`, `PredefinedMenuItem` are now only used inside `src/ui/`.)

- [ ] **Step 3: Replace `App::build_menu` call in `App::refresh`**

In `App::refresh`, change:
```rust
let build = Self::build_menu(&refs, Some(&updated));
```
to:
```rust
let build = ui::build_menu(&refs, Some(&updated));
```

- [ ] **Step 4: Replace `App::build_menu` call in `main()`**

In `fn main()`, change:
```rust
let build = App::build_menu(&initial_refs, None);
```
to:
```rust
let build = ui::build_menu(&initial_refs, None);
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check 2>&1
```

Expected: no errors. If clippy warns about unused imports, fix them.

- [ ] **Step 6: Run all tests**

```bash
cargo test 2>&1 | tail -20
```

Expected: all original tests pass; new ui tests add to the count (≥84 total: 73 original + 7 claude + 2 copilot + 2 base).

- [ ] **Step 7: Commit**

```bash
git add src/main.rs
git commit -m "refactor(main): extract menu building into src/ui/"
```

---

### Task 7: Final verification and card close

- [ ] **Step 1: Full test suite**

```bash
cargo test 2>&1
```

Expected: `test result: ok. N passed; 0 failed` where N ≥ 84.

- [ ] **Step 2: Release build**

```bash
cargo build --release 2>&1 | tail -5
```

Expected: `Finished release [optimized]`.

- [ ] **Step 3: Clippy**

```bash
cargo clippy 2>&1 | grep -E "^error"
```

Expected: no output.

- [ ] **Step 4: Close kanban card**

Update `docs/kanban/ui-module-restructure.md`:
- Set `status: done`
- Update `updated: 2026-06-12`
- Append to Narrative: `- 2026-06-12: Completed. src/ui/{mod,claude,copilot,base}.rs created. main.rs cleaned. All tests green.`

- [ ] **Step 5: Commit docs**

```bash
git add docs/kanban/ui-module-restructure.md docs/superpowers/plans/2026-06-12-ui-module-restructure.md
git commit -m "docs: close card #7 — ui module restructure done"
```
