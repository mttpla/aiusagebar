---
id: 43
title: Fix resets_at nullable crash in Claude provider
status: done
priority: High
created: 2026-06-17
updated: 2026-06-17
closed: 2026-06-17
spec: specs/2026-06-17-fix-resets-at-null-design.md
plan: plans/2026-06-17-fix-resets-at-null.md
---
# Fix resets_at nullable crash in Claude provider

Orange "Parse error: invalid type: null, expected a string at line 1 column 48" appears
when Anthropic returns `"resets_at": null` for a window that was never throttled.

`WindowData.resets_at` is `String` — must be `Option<String>`. Two-line fix in
`src/provider/claude.rs`.

## Narrative

- 2026-06-17: Root cause found via serde error + column count. `resets_at: String` in
  `WindowData` rejects JSON `null`. API returns null when the rate-limit window has
  never been hit (no reset scheduled). Fix: make field `Option<String>` and drop the
  redundant `Some()` wrap in `parse_response` (LimitWindow.resets_at is already
  `Option<String>`). Spec at `docs/superpowers/specs/2026-06-17-fix-resets-at-null-design.md`.
  Marked urgent — visible user-facing error on every poll cycle for affected accounts.
- 2026-06-17: Moved to doing. Plan at `docs/superpowers/plans/2026-06-17-fix-resets-at-null.md`. Single task: add null-resets_at test, change field type, drop Some() wrap, commit.
- 2026-06-17: Implemented and reviewed. Commit 234766b. 148 tests pass, 0 clippy warnings. Final review: Ready to merge (Yes). Minor: happy-path test missing resets_at assertion; missing-field behavior undocumented.
- 2026-06-17: Done. Pushed to master.
