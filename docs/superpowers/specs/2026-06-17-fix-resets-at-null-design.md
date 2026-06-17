# Fix: WindowData.resets_at nullable crash

**Date:** 2026-06-17

## Problem

`parse_response` crashes with serde error `invalid type: null, expected a string at line 1 column 48`
when Anthropic returns `"resets_at": null` for a window that has never been throttled.

Root cause: `WindowData.resets_at` is typed `String` — serde rejects JSON `null`.
The error surfaces in the menu bar as an orange "Parse error: …" row.

## Fix

1. Change `resets_at: String` → `resets_at: Option<String>` in `WindowData`.
2. In `parse_response`, drop the redundant `Some(…)` wrapping — `LimitWindow.resets_at`
   is already `Option<String>`, so assign directly.

## Scope

- `src/provider/claude.rs` only: two lines changed.

## Non-goals

- No change to `LimitWindow`, `UsageState`, or any other type.
- No observability / debug logging (tracked separately).
