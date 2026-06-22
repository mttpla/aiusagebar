# Move provider Details into the Other submenu — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Relocate each provider's `Details…` item out of its menu section and into the always-present `Other ▶` submenu, grouped per provider next to `Diagnostics`.

**Architecture:** A new pure function `other_entries` decides what appears inside `Other` (provider Details entries for providers that have raw JSON, then Diagnostics when the log is non-empty, then a placeholder if nothing else). `append_other` renders that decision and owns the Details `MenuId`s. The provider section builders shrink — they no longer append `Details…`. `main.rs` computes which providers have raw JSON and threads that list into `build_menu`.

**Tech Stack:** Rust, `tray-icon` menu API (`Menu`, `Submenu`, `MenuItem`, `MenuId`).

## Global Constraints

- Binary crate: default `pub(crate)`/private, never bare `pub`.
- All `.rs` string literals in English.
- Tokens are read-only; no network/auth code touched here.
- Run `cargo clippy -- -D warnings && cargo test` before every commit.
- No `#[allow(dead_code)]`.
- Spec: `docs/superpowers/specs/2026-06-22-details-in-other-submenu-design.md`.

---

### Task 1: Pure `other_entries` decision function

**Files:**
- Modify: `src/ui/base.rs` (add enum + function + tests)

**Interfaces:**
- Consumes: `crate::provider::ProviderKind` (must be `Copy + PartialEq + Debug` — it already derives these as a `HashMap` key; if `Debug` is missing, add it to its `#[derive(...)]`).
- Produces:
  - `pub(crate) enum OtherEntry { Provider(ProviderKind), Diagnostics, Placeholder }`
  - `pub(crate) fn other_entries(details_kinds: &[ProviderKind], diag_empty: bool) -> Vec<OtherEntry>`

- [ ] **Step 1: Write the failing tests**

Add inside the existing `#[cfg(test)] mod tests` in `src/ui/base.rs` (create the module if absent):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderKind;

    #[test]
    fn entries_both_providers_and_diag() {
        let got = other_entries(&[ProviderKind::Claude, ProviderKind::Copilot], false);
        assert_eq!(
            got,
            vec![
                OtherEntry::Provider(ProviderKind::Claude),
                OtherEntry::Provider(ProviderKind::Copilot),
                OtherEntry::Diagnostics,
            ]
        );
    }

    #[test]
    fn entries_provider_without_raw_json_omitted() {
        let got = other_entries(&[ProviderKind::Claude], false);
        assert_eq!(
            got,
            vec![OtherEntry::Provider(ProviderKind::Claude), OtherEntry::Diagnostics]
        );
    }

    #[test]
    fn entries_diag_empty_omits_diagnostics() {
        let got = other_entries(&[ProviderKind::Claude], true);
        assert_eq!(got, vec![OtherEntry::Provider(ProviderKind::Claude)]);
    }

    #[test]
    fn entries_nothing_present_falls_back_to_placeholder() {
        let got = other_entries(&[], true);
        assert_eq!(got, vec![OtherEntry::Placeholder]);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib ui::base::tests`
Expected: FAIL — `cannot find type OtherEntry` / `cannot find function other_entries`.

- [ ] **Step 3: Implement the enum and function**

Add near the top of `src/ui/base.rs` (after the `use` line), and add `ProviderKind` to the `use`:

```rust
use crate::provider::ProviderKind;

#[derive(Debug, PartialEq)]
pub(crate) enum OtherEntry {
    Provider(ProviderKind),
    Diagnostics,
    Placeholder,
}

/// Decides what appears inside the "Other ▶" submenu, in order:
/// one entry per provider that has raw JSON, then Diagnostics when the diag
/// log is non-empty, then a single Placeholder if nothing else would show.
pub(crate) fn other_entries(details_kinds: &[ProviderKind], diag_empty: bool) -> Vec<OtherEntry> {
    let mut entries: Vec<OtherEntry> =
        details_kinds.iter().map(|k| OtherEntry::Provider(*k)).collect();
    if !diag_empty {
        entries.push(OtherEntry::Diagnostics);
    }
    if entries.is_empty() {
        entries.push(OtherEntry::Placeholder);
    }
    entries
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib ui::base::tests`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src/ui/base.rs
git commit -m "feat: add other_entries decision for Other submenu contents"
```

---

### Task 2: Render Details inside Other and thread raw-JSON presence

**Files:**
- Modify: `src/ui/base.rs` (`append_other`)
- Modify: `src/ui/mod.rs` (`build_menu`)
- Modify: `src/main.rs` (both `build_menu` call sites)

**Interfaces:**
- Consumes: `other_entries`, `OtherEntry` (Task 1); `crate::diag::is_empty`; `ProviderKind::display_name() -> &'static str`.
- Produces:
  - `pub(crate) struct OtherIds { pub(crate) details_claude: Option<MenuId>, pub(crate) details_copilot: Option<MenuId>, pub(crate) copy_diag: Option<MenuId> }`
  - `pub(crate) fn append_other(menu: &Menu, details_kinds: &[ProviderKind]) -> OtherIds`
  - `build_menu(..., details_kinds: &[ProviderKind])` — new trailing parameter.

Note: in this task the provider sections STILL append their flat `Details…` (its returned id is ignored), so Details appears in both places temporarily. Task 3 removes the flat one. The crate compiles and ships after each task.

- [ ] **Step 1: Rewrite `append_other` in `src/ui/base.rs`**

Replace the entire existing `append_other` function (and its doc comment) with:

```rust
pub(crate) struct OtherIds {
    pub(crate) details_claude: Option<MenuId>,
    pub(crate) details_copilot: Option<MenuId>,
    pub(crate) copy_diag: Option<MenuId>,
}

/// Appends the always-present "Other ▶" submenu. Contains a "Provider ▶ Details…"
/// entry for each provider in `details_kinds`, then "Diagnostics ▶ Copy diagnostic
/// log" when the diag log is non-empty, and a disabled "No diagnostics" placeholder
/// only when nothing else would appear.
pub(crate) fn append_other(menu: &Menu, details_kinds: &[ProviderKind]) -> OtherIds {
    let other = Submenu::new("Other", true);
    let mut details_claude: Option<MenuId> = None;
    let mut details_copilot: Option<MenuId> = None;
    let mut copy_diag: Option<MenuId> = None;

    for entry in other_entries(details_kinds, crate::diag::is_empty()) {
        match entry {
            OtherEntry::Provider(kind) => {
                let sub = Submenu::new(kind.display_name(), true);
                let item = MenuItem::new("Details…", true, None);
                let id = item.id().clone();
                sub.append(&item).expect("menu append failed");
                other.append(&sub).expect("menu append failed");
                match kind {
                    ProviderKind::Claude => details_claude = Some(id),
                    ProviderKind::Copilot => details_copilot = Some(id),
                }
            }
            OtherEntry::Diagnostics => {
                let diagnostics = Submenu::new("Diagnostics", true);
                let copy = MenuItem::new("Copy diagnostic log", true, None);
                copy_diag = Some(copy.id().clone());
                diagnostics.append(&copy).expect("menu append failed");
                other.append(&diagnostics).expect("menu append failed");
            }
            OtherEntry::Placeholder => {
                let placeholder = MenuItem::new("No diagnostics", false, None);
                other.append(&placeholder).expect("menu append failed");
            }
        }
    }

    menu.append(&other).expect("menu append failed");
    OtherIds { details_claude, details_copilot, copy_diag }
}
```

- [ ] **Step 2: Update `build_menu` in `src/ui/mod.rs`**

Change the signature to add the trailing parameter:

```rust
pub(crate) fn build_menu(
    states: &[(ProviderKind, &UsageState)],
    last_updated: Option<&str>,
    update: Option<&str>,
    details_kinds: &[ProviderKind],
) -> MenuBuild {
```

Inside the provider loop, ignore the section's returned details id (Task 3 removes it). Change:

```rust
            ProviderKind::Claude => {
                let (sc, dc) = claude::append_claude_section(&menu, state);
                setup_claude = sc;
                details_claude = Some(dc);
            }
            ProviderKind::Copilot => {
                let (sc, dc) = copilot::append_copilot_section(&menu, state);
                setup_copilot = sc;
                details_copilot = Some(dc);
            }
```

to:

```rust
            ProviderKind::Claude => {
                let (sc, _dc) = claude::append_claude_section(&menu, state);
                setup_claude = sc;
            }
            ProviderKind::Copilot => {
                let (sc, _dc) = copilot::append_copilot_section(&menu, state);
                setup_copilot = sc;
            }
```

Replace the `let copy_diag = base::append_other(&menu);` line and the `MenuBuild { ... }` construction so the Details/copy ids come from `append_other`:

```rust
    let other = base::append_other(&menu, details_kinds);
    let footer = base::append_footer(&menu);
    let layout = build_layout(states, last_updated, update);

    #[cfg(target_os = "macos")]
    styled::style_menu(&menu, &layout);

    #[cfg(not(target_os = "macos"))]
    let _ = layout;

    MenuBuild {
        menu,
        about: footer.about,
        refresh: footer.refresh,
        quit: footer.quit,
        update: update_id,
        setup_claude,
        setup_copilot,
        details_claude: other.details_claude,
        details_copilot: other.details_copilot,
        copy_diag: other.copy_diag,
    }
```

The local `mut details_claude` / `mut details_copilot` declarations above the loop are now unused — delete those two `let mut details_claude: Option<MenuId> = None;` / `details_copilot` lines.

- [ ] **Step 3: Update both `build_menu` call sites in `src/main.rs`**

In `refresh_all` (around line 89-93), compute the provider kinds that currently have raw JSON, then pass them:

```rust
        let refs: Vec<(ProviderKind, &UsageState)> =
            states.iter().map(|(k, s)| (*k, s)).collect();
        let details_kinds: Vec<ProviderKind> = refs
            .iter()
            .map(|(k, _)| *k)
            .filter(|k| {
                self.providers
                    .iter()
                    .any(|p| p.kind() == *k && p.raw_json().is_some())
            })
            .collect();
        let now = Local::now();
        let updated = now.format("%H:%M").to_string();
        let build = ui::build_menu(
            &refs,
            Some(&updated),
            self.update_available.as_deref(),
            &details_kinds,
        );
```

In `main` (around line 201), the initial build has no fetched data yet, so pass an empty slice:

```rust
    let build = ui::build_menu(&initial_refs, None, None, &[]);
```

- [ ] **Step 4: Verify it compiles and all tests pass**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: PASS. (Details now appears both in each section and inside Other — expected interim state.)

- [ ] **Step 5: Commit**

```bash
git add src/ui/base.rs src/ui/mod.rs src/main.rs
git commit -m "feat: render provider Details inside the Other submenu"
```

---

### Task 3: Remove the flat Details from provider sections

**Files:**
- Modify: `src/ui/claude.rs` (`append_claude_section`, `section_item_count`, tests)
- Modify: `src/ui/copilot.rs` (`append_copilot_section`, `section_item_count`, tests)
- Modify: `src/ui/mod.rs` (provider loop destructure, `build_layout` index tests)

**Interfaces:**
- Produces: `append_claude_section(menu, state) -> Option<MenuId>` and `append_copilot_section(menu, state) -> Option<MenuId>` (setup id only). `section_item_count`: `Ok` → `1 + windows.len()`, else `1`.

- [ ] **Step 1: Update the section-count tests to the new expectations**

In `src/ui/claude.rs` tests:

```rust
    #[test]
    fn append_claude_section_count_ok_two_windows() {
        let state = UsageState::Ok(
            vec![
                LimitWindow { name: "daily".into(), percent_used: Some(50.0), ..Default::default() },
                LimitWindow { name: "monthly".into(), percent_used: Some(20.0), ..Default::default() },
            ],
            Some("max".into()),
        );
        assert_eq!(section_item_count(&state), 3); // 1 header + 2 windows
    }

    #[test]
    fn append_claude_section_count_not_configured() {
        assert_eq!(section_item_count(&UsageState::NotConfigured), 1); // header only
    }
```

In `src/ui/copilot.rs` tests:

```rust
    #[test]
    fn append_copilot_section_count_ok_one_window() {
        use crate::provider::UsageState;
        let state = UsageState::Ok(
            vec![make_window("monthly", Some(10.0), None)],
            None,
        );
        assert_eq!(section_item_count(&state), 2); // 1 header + 1 window
    }

    #[test]
    fn append_copilot_section_count_not_configured() {
        use crate::provider::UsageState;
        assert_eq!(section_item_count(&UsageState::NotConfigured), 1); // header only
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib ui::claude ui::copilot`
Expected: FAIL — `section_item_count` still returns the old `2 + windows.len()` / `2`.

- [ ] **Step 3: Shrink `section_item_count` and the section builders**

In `src/ui/claude.rs`, replace the doc comment + `section_item_count` and `append_claude_section`:

```rust
/// Returns the number of NSMenu items that `append_claude_section` will append:
/// 1 header + 1 per window when `UsageState::Ok`, else 1 header.
pub(crate) fn section_item_count(state: &UsageState) -> usize {
    match state {
        UsageState::Ok(windows, _) => 1 + windows.len(),
        _ => 1,
    }
}

pub(crate) fn append_claude_section(menu: &Menu, state: &UsageState) -> Option<MenuId> {
    if let UsageState::NotConfigured = state {
        let item = MenuItem::new(
            header_label(ProviderKind::Claude.display_name(), state),
            true,
            None,
        );
        let setup_id = item.id().clone();
        menu.append(&item).expect("menu append failed");
        return Some(setup_id);
    }
    super::append_label(menu, header_label(ProviderKind::Claude.display_name(), state));
    if let UsageState::Ok(windows, _) = state {
        let now = Local::now();
        for w in windows {
            let reset = w
                .resets_at
                .as_deref()
                .map(|s| super::time::format_reset_local(s, now))
                .unwrap_or_else(|| "?".to_string());
            super::append_label(
                menu,
                format!("  {} — {}  resets {}", w.name, pct_label(w.percent_used), reset),
            );
        }
    }
    None
}
```

In `src/ui/copilot.rs`, replace the doc comment + `section_item_count` and `append_copilot_section`:

```rust
/// Returns the number of NSMenu items that `append_copilot_section` will append:
/// 1 header + 1 per window when `UsageState::Ok`, else 1 header.
pub(crate) fn section_item_count(state: &UsageState) -> usize {
    match state {
        UsageState::Ok(windows, _) => 1 + windows.len(),
        _ => 1,
    }
}

pub(crate) fn append_copilot_section(menu: &Menu, state: &UsageState) -> Option<MenuId> {
    if let UsageState::NotConfigured = state {
        let item = MenuItem::new(
            header_label(ProviderKind::Copilot.display_name(), state),
            true,
            None,
        );
        let setup_id = item.id().clone();
        menu.append(&item).expect("menu append failed");
        return Some(setup_id);
    }
    super::append_label(menu, header_label(ProviderKind::Copilot.display_name(), state));
    if let UsageState::Ok(windows, _) = state {
        let now = Local::now();
        for w in windows {
            super::append_label(menu, row_label(w, now));
        }
    }
    None
}
```

- [ ] **Step 4: Update the provider loop in `src/ui/mod.rs`**

The section builders now return `Option<MenuId>` directly. Change the loop to:

```rust
            ProviderKind::Claude => {
                setup_claude = claude::append_claude_section(&menu, state);
            }
            ProviderKind::Copilot => {
                setup_copilot = copilot::append_copilot_section(&menu, state);
            }
```

- [ ] **Step 5: Update `build_layout` index tests in `src/ui/mod.rs`**

Each provider section is now one item shorter, so footer/window indices shift. Apply these exact changes:

`menu_layout_indices_claude_two_windows`:
```rust
        assert_eq!(layout.refresh_idx, 4);
        assert_eq!(layout.quit_idx, 7);
```

`build_layout_copilot_window_items_indices`:
```rust
        assert_eq!(layout.window_items[2].0, 4);
        assert_eq!(layout.window_items[2].1.name, "monthly");
        assert_eq!(layout.refresh_idx, 6);
        assert_eq!(layout.quit_idx, 9);
```
(`window_items[0].0` stays 1 and `window_items[1].0` stays 2 — headers/windows order is unchanged.)

`build_layout_with_update_shifts_all_indices_by_2`:
```rust
        // header was at 0 without update, now at 2
        assert_eq!(layout.header_indices[0].0, 2);
        // window item was at 1, now at 3
        assert_eq!(layout.window_items[0].0, 3);
        // refresh: header(2) + window(1) = idx 3, +1 Other = 4
        assert_eq!(layout.refresh_idx, 4);
        assert_eq!(layout.quit_idx, 7);
```

`build_layout_without_update_unchanged`:
```rust
        assert_eq!(layout.header_indices[0].0, 0);
        assert_eq!(layout.refresh_idx, 3);
```

(`menu_layout_indices_no_providers` is unchanged — no provider sections.)

- [ ] **Step 6: Run the full suite**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: PASS. Details now appears only inside `Other ▶`.

- [ ] **Step 7: Commit**

```bash
git add src/ui/claude.rs src/ui/copilot.rs src/ui/mod.rs
git commit -m "feat: remove flat Details from provider sections"
```

---

## Self-Review

**Spec coverage:**
- Details leaves sections → Task 3. ✓
- `section_item_count` −1 → Task 3. ✓
- Provider Details only when raw JSON present → Task 1 (`other_entries`) + Task 2 (`details_kinds` from `main.rs`). ✓
- Diagnostics hidden when log empty → Task 1 + Task 2. ✓
- Order providers-then-Diagnostics → Task 1. ✓
- Empty-Other placeholder fallback → Task 1 + Task 2. ✓
- `append_other` takes provider info + owns Details ids → Task 2. ✓
- `build_menu`/`build_layout` index recompute → Task 2 (signature), Task 3 (test values). ✓
- `main.rs` click handler unchanged → confirmed, not modified. ✓
- Out of scope (`details::show`, per-account submenus) → untouched. ✓

**Placeholder scan:** No TBD/TODO; every code step shows full code. ✓

**Type consistency:** `other_entries(&[ProviderKind], bool) -> Vec<OtherEntry>`, `append_other(&Menu, &[ProviderKind]) -> OtherIds`, `build_menu(..., &[ProviderKind])`, section builders `-> Option<MenuId>` — used consistently across tasks. ✓
