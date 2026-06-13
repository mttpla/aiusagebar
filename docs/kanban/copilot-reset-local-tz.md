---
id: 32
status: backlog
priority: High
tags: [copilot, ui, timezone]
spec: superpowers/specs/2026-06-13-copilot-reset-local-tz-design.md
created: 2026-06-13
updated: 2026-06-13
---
# Copilot reset time in local OS timezone

Mirror card #31 for the Copilot provider: convert `resets_at` (raw ISO 8601 UTC) to OS local timezone before rendering at `src/ui/copilot.rs:19`. Same format rules — `HH:MM` same-day, `YYYY-MM-DD HH:MM` otherwise, raw passthrough on parse failure.

## Narrative
- 2026-06-13: Split from card #31 to keep Claude PR focused. Reuse approach: a `format_reset_local(iso_utc, now)` helper, same signature and same test shape. Open question for brainstorming when picked up: extract the helper to a shared `src/ui/time.rs` and call from both `claude.rs` and `copilot.rs`, or duplicate per UI module — depends on whether card #31 lands first and what shape it leaves. If #31 already extracted the helper, this card just adds the call site + tests in `copilot.rs`.
