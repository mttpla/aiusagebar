# Spec: Onboarding / NotConfigured empty state

Card: #13 — `docs/kanban/onboarding-empty-state.md`

## Goal

Replace the silent, non-clickable "not configured" label with an actionable row per
provider. On a fresh install the user should be able to click the provider header and
land on a dedicated setup page for that provider.

## Current behaviour

`header_label` for `NotConfigured` returns `"Claude: not configured"`.
`append_claude_section` / `append_copilot_section` call `append_label` which creates
a **disabled** `MenuItem` (`enabled = false`). No `MenuId` is exposed.

## Target behaviour

| State | Label | Clickable |
|---|---|---|
| `NotConfigured` | `"Claude — not signed in · Setup…"` | yes → opens provider setup page |
| everything else | unchanged | unchanged |

Same pattern for Copilot (`"GitHub Copilot — not signed in · Setup…"`).

## Changes

### `src/ui/claude.rs`

- `header_label`, `NotConfigured` arm:
  ```rust
  UsageState::NotConfigured => format!("{} — not signed in · Setup\u{2026}", name),
  ```
- `append_claude_section`: for `NotConfigured`, create an **enabled** `MenuItem`
  (`MenuItem::new(label, true, None)`) instead of calling `append_label`. Capture and
  return its `MenuId`.
- Return type changes from `usize` to `Option<tray_icon::menu::MenuId>`:
  - `NotConfigured` → `Some(id)`
  - all other states → `None`

### `src/ui/copilot.rs`

Same changes as `claude.rs`:
- `header_label` `NotConfigured` arm updated.
- `append_copilot_section` returns `Option<MenuId>`.

### `src/ui/mod.rs`

`MenuBuild` gains two fields:
```rust
pub setup_claude:  Option<MenuId>,
pub setup_copilot: Option<MenuId>,
```

`build_menu` propagates the `Option<MenuId>` returned by each `append_*_section` call
into the new fields.

`build_layout` and `section_item_count` are **unchanged** — `NotConfigured` still
occupies exactly 1 menu slot (the header row itself becomes the action).

### `src/main.rs`

Two URL constants at the top of the file:
```rust
const CLAUDE_SETUP_URL:  &str = "https://github.com/mttpla/aiusagebar/blob/master/claude-setup.md";
const COPILOT_SETUP_URL: &str = "https://github.com/mttpla/aiusagebar/blob/master/copilot-setup.md";
```

`App` struct gains:
```rust
id_setup_claude:  Option<MenuId>,
id_setup_copilot: Option<MenuId>,
```

`App::refresh` updates both fields from `build.setup_claude` / `build.setup_copilot`.

Event handler in `about_to_wait` adds two branches after the existing `id_update` check:
```rust
} else if self.id_setup_claude.as_ref().is_some_and(|id| ev.id == *id) {
    let _ = std::process::Command::new("open").arg(CLAUDE_SETUP_URL).spawn();
} else if self.id_setup_copilot.as_ref().is_some_and(|id| ev.id == *id) {
    let _ = std::process::Command::new("open").arg(COPILOT_SETUP_URL).spawn();
}
```

## What does NOT change

- `section_item_count` — stays 1 for `NotConfigured`.
- `build_layout` — index tracking is unaffected.
- `src/ui/styled.rs` — `header_attr_str` applies brand colour via `setAttributedTitle`,
  which works identically on enabled items. No ObjC2 changes needed.
- `src/ui/base.rs`, `src/ui/time.rs` — untouched.
- Icon logic, keychain, HTTP layer — untouched.

## Setup pages

Two dedicated Markdown files live at the repo root:

- `claude-setup.md` — Claude setup (token location, Keychain prompt, FAQ)
- `copilot-setup.md` — Copilot setup (PAT scopes, env var priority, FAQ)

These are separate from `README.md` by design: the main README stays concise; the
setup pages can be as detailed as needed without cluttering it.

GitHub renders both pages natively when the user clicks through from the menu. The
content of these files is out of scope for this card's implementation task — they can
be written in a follow-up or alongside the code changes.

## Tests

- `header_label` `NotConfigured` test in both `claude.rs` and `copilot.rs`: update
  expected string.
- `append_claude_section_count_not_configured` (and Copilot equivalent): rename to
  `section_item_count_not_configured`, assert still 1 — unchanged.
- New test in `claude.rs`: `setup_id_returned_when_not_configured` — call
  `append_claude_section` with a mock/real `Menu` and assert `Some(_)` is returned.
- New test in `copilot.rs`: same pattern.
- `MenuBuild` fields initialised in `build_menu` tests (existing tests can just
  ignore the new fields; add one test asserting `setup_claude == None` when state is
  `Ok`).
