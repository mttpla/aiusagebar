# Spec: Exponential backoff on 429 / 5xx

Card: #19 — `docs/kanban/api-error-backoff.md`

## Problem

The polling loop in `about_to_wait` calls `self.refresh()` for all providers at a
fixed interval (`settings.poll_interval`, default 300s). If the server returns HTTP 429
(rate-limited) or a 5xx error, the next poll fires at the same interval, producing a
burst that risks triggering Claude's documented "persistent ban" (CLAUDE.md §3: 180s
minimum poll interval, persistent HTTP 429 on violation). No protection exists against
back-to-back 5xx during a server outage.

## Non-goals

- No jitter (randomised delay). The ban risk is deterministic, not probabilistic.
- No user-visible backoff indicator in the menu.
- No persisting backoff state across app restarts.

## Design

### Settings additions

Two new fields in `src/settings.rs`, with matching `DEFAULT_*` constants:

```rust
pub const DEFAULT_BACKOFF_FACTOR: u32     = 2;
pub const DEFAULT_BACKOFF_CAP: Duration   = Duration::from_secs(3600);

pub struct Settings {
    pub poll_interval:       Duration,
    pub alert_threshold_pct: f32,
    pub backoff_factor:      u32,      // multiplier per error step (e.g. 2 → doubles)
    pub backoff_cap:         Duration, // maximum backoff interval
}
```

These are the only knobs needed to tune retry behaviour in the future. The base
interval is always `settings.poll_interval`.

### New `HttpError` variant

`src/http.rs` currently maps all non-200/401/429 responses to `Other(format!("HTTP
{}", code))`, conflating 5xx server errors and network errors. Network errors should
NOT trigger backoff (the remote server is not at fault).

Add:

```rust
pub enum HttpError {
    Unauthorized,
    RateLimited,
    ServerError(u16),  // new: HTTP 5xx
    Other(String),     // network errors, unexpected 4xx, body-read failures
}
```

`http::get` and `http::get_public` update their match arms:

```rust
500..=599 => Err(HttpError::ServerError(code)),
code      => Err(HttpError::Other(format!("HTTP {}", code))),
```

### `BackoffState` struct

New file `src/backoff.rs` — pure logic, no I/O:

```rust
use std::time::{Duration, Instant};

pub struct BackoffState {
    pub next_allowed_at:  Instant,
    pub current_interval: Duration,
}

impl BackoffState {
    pub fn new(base: Duration) -> Self {
        Self { next_allowed_at: Instant::now(), current_interval: base }
    }

    pub fn on_success(&mut self, base: Duration) {
        self.current_interval = base;
    }

    pub fn on_error(&mut self, factor: u32, cap: Duration) {
        self.current_interval = (self.current_interval * factor).min(cap);
        self.next_allowed_at  = Instant::now() + self.current_interval;
    }

    pub fn is_allowed(&self) -> bool {
        Instant::now() >= self.next_allowed_at
    }
}
```

`on_error` receives `factor` and `cap` from `Settings` — no hardcoded constants inside
the struct.

### Backoff sequence (defaults: base = 300s, factor = 2, cap = 3600s)

| Step | Interval |
|------|----------|
| 0 (base) | 300s |
| 1st error | 600s |
| 2nd consecutive | 1200s |
| 3rd consecutive | 2400s |
| 4th+ consecutive | 3600s (cap) |

Claude's 180s floor is never violated — base 300s > 180s, backoff only increases.

### Where state lives

`App` gains a `HashMap<ProviderKind, BackoffState>` alongside `providers`. Providers
remain stateless; the `UsageProvider` trait is not responsible for backoff.

### Exposing the raw error

To decide which `HttpError` variant was returned before it is erased into a
`UsageState` string, add `fetch_raw` to the trait:

```rust
pub trait UsageProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn fetch_raw(&self) -> Result<Vec<LimitWindow>, HttpError>;
    fn fetch(&self) -> UsageState {
        match self.fetch_raw() {
            Ok(windows)                    => UsageState::Ok(windows),
            Err(HttpError::Unauthorized)   => UsageState::Stale("token expired".into()),
            Err(HttpError::RateLimited)    => UsageState::Error("rate limited".into()),
            Err(HttpError::ServerError(c)) => UsageState::Error(format!("server error {c}")),
            Err(HttpError::Other(msg))     => UsageState::Error(msg),
        }
    }
}
```

`claude.rs` and `copilot.rs` rename their current `fetch` body to `fetch_raw`; the
trait default handles the conversion.

### What triggers backoff

| Outcome | Action |
|---|---|
| `UsageState::Ok(_)` | `on_success` — reset `current_interval` to `settings.poll_interval` |
| `HttpError::RateLimited` | `on_error(factor, cap)` |
| `HttpError::ServerError(_)` | `on_error(factor, cap)` |
| `HttpError::Unauthorized` | no backoff (→ `Stale`) |
| `HttpError::Other(_)` | no backoff (network / parse error) |
| `UsageState::NotConfigured` | no backoff |
| `UsageState::Stale(_)` | no backoff |

### Polling gating in `about_to_wait`

The global `next_poll_at: Instant` on `App` is replaced by per-provider
`BackoffState.next_allowed_at`. `about_to_wait` fires `refresh_provider(idx, force:
false)` only when `backoff[kind].is_allowed()`. `WaitUntil` deadline = minimum
`next_allowed_at` across all providers (or the update-check deadline, whichever is
sooner).

### Manual refresh

The menu-click handler calls `refresh_all(force: true)`, which skips the
`is_allowed()` gate. Backoff state is still updated from the result so a
user-triggered failure correctly extends the interval.

### UI

No change. Backoff is invisible to the user.

## Files touched

| File | Change |
|---|---|
| `src/settings.rs` | Add `backoff_factor: u32`, `backoff_cap: Duration`, constants |
| `src/http.rs` | Add `ServerError(u16)`; update match in `get` and `get_public` |
| `src/backoff.rs` | New: `BackoffState` |
| `src/provider/mod.rs` | Add `fetch_raw` to trait; add default `fetch` impl |
| `src/provider/claude.rs` | Rename fetch body to `fetch_raw` |
| `src/provider/copilot.rs` | Same |
| `src/main.rs` | Add `backoff` map; `refresh_provider` + `refresh_all`; gate per-provider in `about_to_wait`; remove global `next_poll_at` |

## Tests

- `src/backoff.rs`: `on_success` resets interval; doubling sequence; cap enforcement; `is_allowed` timing.
- `src/settings.rs`: new defaults are correct.
- `src/http.rs`: `ServerError` variant constructed for 5xx status codes.
- `src/provider/`: `fetch_raw` maps to `UsageState` via trait default.
- Manual smoke: idle CPU ~0%, click Refresh works immediately after a forced 429.
