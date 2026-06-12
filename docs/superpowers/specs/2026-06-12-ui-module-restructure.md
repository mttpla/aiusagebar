# UI Module Restructure

**Date:** 2026-06-12
**Status:** Approved
**Parent spec:** `2026-06-11-ui-aesthetics-design.md` (§ File Structure)

## Goal

Extract all menu-building code from `src/main.rs` into a new `src/ui/` module. No visual change. No new dependencies. Pure refactor.

## Problem

`src/main.rs` owns `build_menu`, `append_label`, `MenuBuild`. Adding styling later requires touching `main.rs` for rendering logic — wrong separation. Provider-specific rendering has no home.

## File Structure

```
src/ui/
  mod.rs      — MenuBuild, FooterIds, pub fn build_menu(states, updated) -> MenuBuild
  base.rs     — append_footer(menu, updated) -> FooterIds  (Refresh + About + Quit items)
  claude.rs   — append_claude_section(menu, state)
  copilot.rs  — append_copilot_section(menu, state)
```

`main.rs` calls `ui::build_menu(states, updated)` and gets back `MenuBuild`. All `append_label`, `build_menu`, `MenuBuild` code removed from `main.rs`.

## Public API

```rust
// src/ui/mod.rs
pub struct MenuBuild {
    pub menu: Menu,
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
}

pub struct FooterIds {
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
}

pub fn build_menu(states: &[(&str, &UsageState)], last_updated: Option<&str>) -> MenuBuild;

// src/ui/base.rs
pub fn append_footer(menu: &Menu, updated: Option<&str>) -> FooterIds;

// src/ui/claude.rs
pub fn append_claude_section(menu: &Menu, state: &UsageState);

// src/ui/copilot.rs
pub fn append_copilot_section(menu: &Menu, state: &UsageState);
```

## Behavior

Plain text labels, identical to current output. `build_menu` routes by provider name:
- `"Claude"` → `claude::append_claude_section`
- `"Copilot"` → `copilot::append_copilot_section`
- fallback → plain disabled label

Footer order: separator → `↺ Refresh` → separator → `ℹ About AIUsageBar` → `✕ Quit`.

State rendering (plain text, identical to current `main.rs`):

| State | Header text | Row text |
|---|---|---|
| `Ok(_, Some(p))` | `● Claude — {p}` | `  {name} — {pct}  resets {reset}` |
| `Ok(_, None)` | `● Claude — account unavailable` | same |
| `Stale(msg)` | `● Claude ⚠ stale` | `  {msg}` |
| `Error(msg)` | `● Claude ✕ error` | `  {msg}` |
| `NotConfigured` | `● Claude` | `  not configured` |

## Testing

Unit tests for:
- `claude::header_label(name, state)` — all 5 `UsageState` variants
- `claude::pct_label(pct)` — `Some` and `None`
- `copilot::row_label(window)` — with/without counts, with/without reset
- `base::refresh_label(updated)` — `Some` and `None`

`cargo test` must stay green (73 passing). No new ObjC2 code.

## Affected Files

- `src/main.rs` — remove `MenuBuild`, `append_label`, `fn build_menu`; add `mod ui`; call `ui::build_menu`
- `src/ui/mod.rs` — new
- `src/ui/base.rs` — new
- `src/ui/claude.rs` — new
- `src/ui/copilot.rs` — new
