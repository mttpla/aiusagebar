---
id: 56
status: todo
priority: Normal
tags: [refactor, config, settings]
created: 2026-06-23
updated: 2026-06-23
---
# Centralize hardcoded constants into settings.rs

Move scattered production magic numbers into `settings.rs` so config lives in one
place and select knobs can become user-configurable later. UI layout/color values
in `styled.rs` are explicitly out of scope — they stay local.

## Scope (5 values)

| Source | Current value | New const in settings.rs | Rationale |
|---|---|---|---|
| `diag.rs:4` | `CAPACITY = 100` | `DIAG_BUFFER_CAPACITY` | central + future user knob (log depth) |
| `diag.rs:5` | `MAX_MSG_BYTES = 2048` | `DIAG_MAX_MSG_BYTES` | central |
| `provider/claude.rs:249` | `truncate(&body, 2048)` | reuse `DIAG_MAX_MSG_BYTES` | dedupe magic number |
| `http.rs:21` | `Duration::from_secs(15)` | `HTTP_TIMEOUT` | central + future user knob (network) |
| `http.rs:58` | `truncate(..., 512)` | `DIAG_HTTP_BODY_CAP` | central (internal diag) |

The two genuine future-configurable knobs: `DIAG_BUFFER_CAPACITY` and `HTTP_TIMEOUT`.
The byte caps are central-for-tidiness only.

## Out of scope (stay local)
- All `styled.rs` UI layout numbers, font sizes, color tuples (incl. `60.0`/`80.0`
  bar-color thresholds — separate concern from alert threshold).
- `claude.rs:105` User-Agent fallback string (network identity, already
  env-overridable, tied to Claude hard-constraint #3 — keep in provider).
- Domain-local consts: setup URLs, errSec codes, launchctl codes, START_YEAR,
  USAGE_URL/PROFILE_URL.

## Acceptance
- 5 values referenced from `settings.rs` consts; no duplicate `2048` literal.
- `cargo clippy -- -D warnings && cargo test` green.
- Idle CPU / runtime behavior unchanged (pure constant relocation).

## Narrative
- 2026-06-23: Captured after full source sweep for magic numbers. Verified against
  live source (tokensave index was 13h stale). Found `settings.rs` already
  centralizes poll/backoff/threshold/update defaults, and `backoff.rs` magic
  numbers are test-only. Threshold already threaded as a param into `icon.rs`.
  Decision: move the 5 diag/http production constants; keep UI numbers and the
  network-identity UA string local. User chose the full-centralize option (all 5,
  incl. dedupe of `claude.rs:249`) over the minimal 2-knob set.
