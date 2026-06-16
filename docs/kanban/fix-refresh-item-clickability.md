---
title: Fix Refresh menu item clickability + footer button tests
status: in-progress
---

## Problem

Since commit f47a25c, clicking "↺ Refresh" does nothing.
`style_menu` calls `item.setView(Some(&view))` on the Refresh NSMenuItem.
macOS intercepts all mouse events for custom-view items — the MenuEvent
carrying `id_refresh` never fires (main.rs:86).

## Root Cause

f47a25c replaced `refresh_attr_str` + `apply_to_item` with
`make_refresh_row_view` + `setView` to stop a 290pt tab stop from
expanding NSMenu width. NSView fixes layout but blocks click events.

## Fix

Option 3: restore attributed string path, remove tab stop.

1. Extract pure functions (no NSKit, no main thread required):
   - `refresh_display_text(updated: Option<&str>) -> (String, usize)`
     → full display string + byte length of "↺ Refresh" for color range
   - `about_display_text() -> &'static str`
   - `quit_display_text() -> &'static str`
2. Rewrite `refresh_attr_str` to use `refresh_display_text`, no tab stop,
   no paragraph style.
   Format: `"↺ Refresh"` when None; `"↺ Refresh  ·  Updated HH:MM"` when Some.
3. Rewrite `about_attr_str` and `quit_attr_str` to delegate to their pure fns.
4. Delete `make_refresh_row_view`, `refresh_para_style`, unused imports.
5. In `style_menu`: use `apply_to_item` for Refresh — no `setView`.

## Tests (TDD — write RED before touching production code)

### refresh_display_text
- `None` → starts with "↺ Refresh", no '\t', no "Updated"
- `Some("12:34")` → contains "↺ Refresh", contains "12:34", no '\t'
- offset returned == NSString byte length of "↺ Refresh"

### about_display_text
- contains "About"
- contains "AIUsageBar"
- no '\t'

### quit_display_text
- eq "Quit" (or starts with it)
- no '\t'

## Rejected options

- **NSButton in NSView**: forwards click but requires objc2 subclass or raw
  msg_send dance; more complex than removing setView entirely.
- **Keep setView + forward events manually**: fragile, couples UI to AppKit
  hit-testing internals.

## Narrative

The custom-NSView approach was a valid fix for tab-stop expansion but
silently broke core functionality. The right tradeoff is to drop
right-alignment (visual nicety) to keep items clickable (core behavior).
Inline "·" separator gives visual separation without layout machinery.
Tests for all three footer items prevent the same class of regression in
future style refactors.
