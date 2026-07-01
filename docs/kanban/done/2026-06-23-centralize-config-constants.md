---
id: 56
status: done
priority: Normal
tags: [refactor, config, settings]
created: 2026-06-23
updated: 2026-07-01
---
# Centralize hardcoded constants into settings.rs

Move scattered production config into `settings.rs`. Filter: values that could
**reasonably become user-configurable tomorrow**. Pure relocation — zero
behavior change. Internal byte caps and domain-identity constants stay local.

Spec: `docs/superpowers/specs/2026-07-01-centralize-config-settings-design.md`

## Tier A — Settings struct fields (App-held, live knobs now)
- `poll_interval`, `alert_threshold_pct`, `backoff_factor`, `backoff_cap` — existing
- **promote** `UPDATE_CHECK_INTERVAL_HOURS` (loose const) → `update_check_interval_hours` field

## Tier B — plain settings.rs consts (consumed by OnceLock globals / pure fn)
| Const | Source | Value |
|---|---|---|
| `HTTP_TIMEOUT` | `http.rs:21` | 15s |
| `DIAG_LOG_MAX_MESSAGES` | `diag.rs:4` `CAPACITY` | 100 (renamed) |
| `BAR_WARN_PCT` | `styled.rs:126` | 60.0 |
| `BAR_ALERT_PCT` | `styled.rs:128` | 80.0 |

Promoting Tier B to live fields needs global/fn plumbing — out of scope.

## Out of scope (stay local)
- Byte caps: `diag MAX_MSG_BYTES=2048`, `http.rs:58 512`, `claude.rs:249 2048`
  (no dedupe).
- Domain identity: USAGE_URL/PROFILE_URL, UA string, setup URLs, errSec codes,
  launchctl codes, START_YEAR.
- Settings persistence + tray submenu UI (REQUIREMENTS §8).

## Acceptance
- Listed values referenced from `settings.rs`; no leftover literals at sources.
- New test: `Settings::default().update_check_interval_hours == 24`.
- `cargo clippy -- -D warnings && cargo test` green.
- Idle CPU / runtime behavior unchanged.

## Narrative
- 2026-06-23: Captured after full source sweep for magic numbers. Original scope
  was the 5 diag/http tidiness constants (incl. byte-cap dedupe).
- 2026-07-01: Re-brainstormed under a stricter filter — "reasonably
  user-configurable tomorrow." User dropped the byte caps (not user-facing, no
  dedupe) and added: promote update-check interval to a Settings field, pull
  styled.rs bar-color zones (60/80) into settings.rs as `BAR_*` consts, rename
  diag `CAPACITY` → `DIAG_LOG_MAX_MESSAGES`. Split check: single atomic card, no
  split.
- 2026-07-01: Done. Two commits — 4c85ce9 (promote update-check interval to a
  Settings field) + a77e77d (four Tier B consts: HTTP_TIMEOUT,
  DIAG_LOG_MAX_MESSAGES, BAR_WARN_PCT, BAR_ALERT_PCT + rewire consumers). Both
  task reviews Approved, `cargo clippy -- -D warnings && cargo test` green (212).
  Minor deferred: `=` column-alignment of DEFAULT_UPDATE_CHECK_INTERVAL_HOURS
  const (longer name breaks sibling alignment).
