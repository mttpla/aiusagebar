---
id: 31
status: done
priority: High
tags: [claude, ui, timezone, pre-1.0]
spec: superpowers/specs/2026-06-13-claude-reset-local-tz-design.md
created: 2026-06-13
updated: 2026-06-15
---
# Claude reset time in local OS timezone

Convert Claude's `resets_at` (raw ISO 8601 UTC from `api.anthropic.com/api/oauth/usage`) to the OS local timezone before rendering. Compact format: `HH:MM` when reset is same calendar day as `Local::now()`, `YYYY-MM-DD HH:MM` otherwise. Today on `Europe/Rome` `14:00Z` should show as `16:00`, not `2026-06-06T14:00:00Z`.

## Narrative
- 2026-06-13: Tagged `pre-1.0` — UTC reset time unreadable, blocker for 1.0 release.
- 2026-06-13: Captured from user report — reset displayed in UTC, unreadable. Decisions: convert in UI layer (`src/ui/claude.rs`), not provider, to keep `LimitWindow.resets_at` machine-readable; helper `format_reset_local(iso_utc, now)` with injected `now` for deterministic tests; same-day vs different-day format split; malformed input falls back to raw passthrough. Rejected: relative time ("in 2h 30m") loses absolute reference; pre-format inside provider couples wire format to display; locale-aware formatter is YAGNI. Copilot UI has identical pattern at `src/ui/copilot.rs:19` — out of scope, tracked under card #32.
- 2026-06-15: Implemented. Commits 3acddee + 68fe5a4 on master. 4 TZ-agnostic tests (shape-check via NaiveTime/NaiveDateTime parse). `Local::now()` hoisted before loop per final review finding.
