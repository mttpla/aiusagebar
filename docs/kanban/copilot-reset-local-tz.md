---
id: 32
status: done
priority: High
tags: [copilot, ui, timezone, pre-1.0]
spec: superpowers/specs/2026-06-13-copilot-reset-local-tz-design.md
created: 2026-06-13
updated: 2026-06-15
---
# Copilot reset time in local OS timezone

Mirror card #31 for the Copilot provider: convert `resets_at` (raw ISO 8601 UTC) to OS local timezone before rendering at `src/ui/copilot.rs:19`. Same format rules — `HH:MM` same-day, `YYYY-MM-DD HH:MM` otherwise, raw passthrough on parse failure.

## Narrative
- 2026-06-13: Tagged `pre-1.0` — UTC reset time unreadable, blocker for 1.0 release.
- 2026-06-13: Split from card #31 to keep Claude PR focused. Reuse approach: a `format_reset_local(iso_utc, now)` helper, same signature and same test shape. Open question for brainstorming when picked up: extract the helper to a shared `src/ui/time.rs` and call from both `claude.rs` and `copilot.rs`, or duplicate per UI module — depends on whether card #31 lands first and what shape it leaves. If #31 already extracted the helper, this card just adds the call site + tests in `copilot.rs`.
- 2026-06-15: Implemented. Card #31 landed with helper inline in `claude.rs`. Chose branch 2: extracted `format_reset_local` to new `src/ui/time.rs` (pub crate), switched `claude.rs` to call it, added TZ conversion to `copilot.rs::row_label(window, now)`. 114 tests pass. `styled.rs::format_reset` left untouched — different purpose (window-aware progress bar labels).
