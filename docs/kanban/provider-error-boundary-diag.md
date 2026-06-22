---
id: 50
status: done
priority: High
tags: [robustness, logging, diag, pre-1.0]
spec: superpowers/specs/2026-06-22-provider-error-boundary-diag-design.md
plan: superpowers/plans/2026-06-22-provider-error-boundary-diag.md
created: 2026-06-22
updated: 2026-06-22
---
# Provider error boundary diagnostics

Guarantee that every provider ending a fetch in a non-happy state (`Error` or `Stale`)
leaves a trace in the diagnostic log. Log at the boundary in `refresh_all` via a pure
`state_diag_message(name, &UsageState) -> Option<String>` helper, instead of sweeping
each leaf path. Closes the silent `is_expired` Stale path (expired Claude token
short-circuits before the HTTP call, so no HTTP-layer `diag!` fires → empty log →
hidden "Other ▶ Diagnostics" submenu).

## Narrative
- 2026-06-22: Captured from a debugging session. User hit "Claude in error but no
  Diagnostics menu". Root cause: `do_fetch` returns `Stale` at the `is_expired` check
  (claude.rs:215) before any HTTP call, and that path emits no `diag!`; the Diagnostics
  submenu only renders when the in-memory log is non-empty (base.rs:17). Confirmed the
  user's token expired 6 days prior. A full audit found 11 silent non-happy paths across
  the codebase. Card #46 had deliberately deferred a full sweep to post-1.0.
  Decision: boundary logging (option B) over per-leaf sweep (option A) — B catches every
  provider non-happy state by construction, including future providers; A is fragile
  (must remember each new path, which is how the gap arose). Both `Error` and `Stale`
  log at `Level::Err`.
  Rejected: any dedup / log-on-transition / throttle system. User chose keep-simple and
  accepts log flooding for a persistently failing provider (steady 401 / network down at
  180s poll) — a full log signals the upstream problem must be fixed first. 429/5xx
  self-throttle via backoff; expired-token makes no HTTP call; so practical flood cases
  are limited.
  Split: non-provider silent paths carved into card #51. The menu-disappearance bug
  (refresh_all rebuilds from the fetched subset only) and in-memory-only log persistence
  are out of scope, noted in the spec.
- 2026-06-22: Moved to doing. Plan written; proceeding to implementation via TDD.
- 2026-06-22: Done. Implemented subagent-driven in 2 commits (0a677dc helper +
  tests, 66c3dbb refresh_all wiring). Task review clean (spec ✅ quality ✅, 0
  findings). Gate green: clippy -D warnings clean, 189 tests pass. Merged to master.
