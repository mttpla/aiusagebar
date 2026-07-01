# Centralize Config into settings.rs — Design

Card #56. Pure relocation refactor. **Zero behavior change.**

## Goal

Move scattered production config values into `settings.rs` so configuration
lives in one place. Filter for what moves: **values that could reasonably
become user-configurable tomorrow**. Pure-internal magic numbers (byte caps)
and domain-identity constants (URLs, User-Agent, OS error codes) stay local.

## Two tiers

Moved values split by how they are consumed today.

### Tier A — `Settings` struct fields

Values already threaded through the App-held `Settings` instance (`main.rs:64`
`settings: Settings`). These are live knobs now — a future settings menu binds
directly to the struct field.

| Field | Source | Change |
|---|---|---|
| `poll_interval` | existing | — |
| `alert_threshold_pct` | existing | — |
| `backoff_factor` | existing | — |
| `backoff_cap` | existing | — |
| `update_check_interval_hours` | `settings.rs:7` loose const `UPDATE_CHECK_INTERVAL_HOURS` | **promote** to struct field |

`update_check_interval_hours`: `main.rs:161` and `main.rs:272` currently read
`settings::UPDATE_CHECK_INTERVAL_HOURS` via the const path. App already holds
`self.settings`, so both sites switch to `self.settings.update_check_interval_hours`
(and the construction site at `main.rs:272` reads the local `settings` binding).
Keep a `DEFAULT_UPDATE_CHECK_INTERVAL_HOURS` const feeding `Default`.

### Tier B — plain `settings.rs` consts

Values consumed by process-global `OnceLock` lazy singletons or pure display
functions that have no `Settings` handle. Centralized and doc-commented as
future user knobs, but promoting them to live struct fields requires new
plumbing (thread `Settings` into the global / fn) — **out of scope** for a
relocation refactor.

| Const | Source | Value | Consumer |
|---|---|---|---|
| `HTTP_TIMEOUT` | `http.rs:21` | `Duration::from_secs(15)` | `agent()` `OnceLock<ureq::Agent>` |
| `DIAG_LOG_MAX_MESSAGES` | `diag.rs:4` `CAPACITY` | `100` | `DIAG` `OnceLock` ring buffer |
| `BAR_WARN_PCT` | `styled.rs:126` | `60.0` | `bar_color` pure fn |
| `BAR_ALERT_PCT` | `styled.rs:128` | `80.0` | `bar_color` pure fn |

Naming notes:
- `DIAG_LOG_MAX_MESSAGES` — max messages retained in the in-memory diagnostic
  log ring buffer (message count, not bytes). Renamed from `CAPACITY` for clarity.
- `BAR_*` prefix distinguishes these bar-color zone boundaries from
  `alert_threshold_pct`, which drives icon state and notifications — a separate
  concern that happens to share the `80.0` value.

## Stay local (untouched)

- **Byte caps:** `diag.rs:5 MAX_MSG_BYTES = 2048`, `http.rs:58 truncate(..., 512)`,
  `provider/claude.rs:249 truncate(&body, 2048)`. Internal, not user-facing.
  No dedupe of the two `2048` literals — out of the user-configurable filter.
- **Domain identity:** `provider/claude.rs` `USAGE_URL`/`PROFILE_URL`, User-Agent
  fallback string, `main.rs` `CLAUDE_SETUP_URL`/`COPILOT_SETUP_URL`,
  `keychain.rs` `errSec*` codes, `launch_at_login.rs` launchctl exit codes,
  `about.rs:3 START_YEAR = 2026`.

## Consumers updated

- `http.rs:21` → `settings::HTTP_TIMEOUT`
- `diag.rs` `CAPACITY` refs (lines 32, 64, and test refs) → `settings::DIAG_LOG_MAX_MESSAGES`
- `ui/styled.rs:126,128` bar_color → `settings::BAR_WARN_PCT` / `settings::BAR_ALERT_PCT`
- `main.rs:161,272` → `self.settings.update_check_interval_hours` / local `settings` field

## Acceptance

- All Tier A/B values referenced from `settings.rs`; no scattered literals left
  at the listed sites.
- New unit test asserting `Settings::default().update_check_interval_hours == 24`.
- `cargo clippy -- -D warnings && cargo test` green.
- Idle CPU ~0% and runtime behavior unchanged (pure constant relocation).

## Out of scope

- Settings persistence (config file load/save).
- Tray settings submenu UI (REQUIREMENTS §8).
- Promoting Tier B consts to live `Settings` fields (needs global/fn plumbing).
- Byte-cap dedupe.
