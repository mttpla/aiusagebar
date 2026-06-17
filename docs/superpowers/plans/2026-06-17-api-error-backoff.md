# Plan: Exponential backoff on 429 / 5xx (#19)

**Spec:** `docs/superpowers/specs/2026-06-17-api-error-backoff.md`

**Goal:** Per-provider exponential backoff on `RateLimited` and `ServerError(5xx)`.
Backoff variables (`factor`, `cap`) live in `Settings` for future configurability.

**New dependency:** none (stdlib only — `std::collections::HashMap`, `std::time`).

## Global constraints

- All string literals in `.rs` must be English.
- `cargo clippy -- -D warnings && cargo test` must pass before every commit.
- Never add `#[allow(dead_code)]`.
- Claude's 180s floor must never be violated (base = 300s, backoff only increases).
- Providers remain stateless.

---

## Task 1 — Add backoff fields to `Settings`

**File:** `src/settings.rs`

- [ ] Add constants and fields:

```rust
pub const DEFAULT_BACKOFF_FACTOR: u32   = 2;
pub const DEFAULT_BACKOFF_CAP: Duration = Duration::from_secs(3600);

pub struct Settings {
    pub poll_interval:       Duration,
    pub alert_threshold_pct: f32,
    pub backoff_factor:      u32,
    pub backoff_cap:         Duration,
}
```

- [ ] Update `Default` impl to include new fields.
- [ ] Add unit tests: `default_backoff_factor_is_two`, `default_backoff_cap_is_one_hour`.
- [ ] `cargo clippy -- -D warnings && cargo test`
- [ ] Commit: `feat(settings): add backoff_factor and backoff_cap fields`

---

## Task 2 — Add `ServerError(u16)` to `HttpError`

**File:** `src/http.rs`

- [ ] Add variant to enum:

```rust
pub enum HttpError {
    Unauthorized,
    RateLimited,
    ServerError(u16),
    Other(String),
}
```

- [ ] Update match in `get`: `500..=599 => Err(HttpError::ServerError(code))`
- [ ] Same in `get_public`.
- [ ] Check all exhaustive matches on `HttpError` in `claude.rs` / `copilot.rs` — update if needed.
- [ ] `cargo clippy -- -D warnings && cargo test`
- [ ] Commit: `feat(http): add ServerError(u16) variant for 5xx responses`

---

## Task 3 — Create `src/backoff.rs`

**File:** `src/backoff.rs` (new), `src/main.rs` (add `mod backoff;`)

- [ ] Create `src/backoff.rs`:

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_immediately_allowed() {
        assert!(BackoffState::new(Duration::from_secs(300)).is_allowed());
    }

    #[test]
    fn on_success_resets_interval() {
        let base = Duration::from_secs(300);
        let mut s = BackoffState::new(base);
        s.on_error(2, Duration::from_secs(3600));
        s.on_success(base);
        assert_eq!(s.current_interval, base);
    }

    #[test]
    fn on_error_doubles_interval() {
        let mut s = BackoffState::new(Duration::from_secs(300));
        s.on_error(2, Duration::from_secs(3600));
        assert_eq!(s.current_interval, Duration::from_secs(600));
        s.on_error(2, Duration::from_secs(3600));
        assert_eq!(s.current_interval, Duration::from_secs(1200));
    }

    #[test]
    fn on_error_caps_at_cap() {
        let cap = Duration::from_secs(3600);
        let mut s = BackoffState::new(Duration::from_secs(300));
        for _ in 0..20 {
            s.on_error(2, cap);
        }
        assert_eq!(s.current_interval, cap);
    }

    #[test]
    fn on_error_blocks_is_allowed() {
        let mut s = BackoffState::new(Duration::from_secs(300));
        s.on_error(2, Duration::from_secs(3600));
        assert!(!s.is_allowed());
    }
}
```

- [ ] Add `mod backoff;` to `src/main.rs`.
- [ ] `cargo clippy -- -D warnings && cargo test`
- [ ] Commit: `feat: add BackoffState with configurable factor and cap`

---

## Task 4 — Add `fetch_raw` to `UsageProvider` trait

**Files:** `src/provider/mod.rs`, `src/provider/claude.rs`, `src/provider/copilot.rs`

- [ ] Add `use crate::http::HttpError;` to `mod.rs`.
- [ ] Update trait with `fetch_raw` + default `fetch`:

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

- [ ] In `claude.rs`: rename `fetch` body to `fetch_raw`, remove manual error mapping (trait default handles it).
- [ ] In `copilot.rs`: same.
- [ ] `cargo clippy -- -D warnings && cargo test`
- [ ] Commit: `refactor(provider): add fetch_raw to trait with default fetch impl`

---

## Task 5 — Add backoff map to `App`; `refresh_provider` + `refresh_all`

**File:** `src/main.rs`

- [ ] Add imports: `use std::collections::HashMap; use backoff::BackoffState;`
- [ ] Add field to `App`: `backoff: HashMap<ProviderKind, BackoffState>`
- [ ] Remove `next_poll_at: Instant` from `App`.
- [ ] Initialise in `main()`:

```rust
let backoff: HashMap<ProviderKind, BackoffState> = providers
    .iter()
    .map(|p| (p.kind(), BackoffState::new(settings.poll_interval)))
    .collect();
```

- [ ] Replace `App::refresh` with `refresh_provider` + `refresh_all`:

```rust
fn refresh_provider(&mut self, idx: usize, force: bool) -> (ProviderKind, UsageState) {
    let kind = self.providers[idx].kind();
    let b = self.backoff.get_mut(&kind).expect("backoff entry missing");
    if !force && !b.is_allowed() {
        return (kind, UsageState::Error("backoff in effect".into()));
    }
    match self.providers[idx].fetch_raw() {
        Ok(windows) => {
            b.on_success(self.settings.poll_interval);
            (kind, UsageState::Ok(windows))
        }
        Err(e @ (HttpError::RateLimited | HttpError::ServerError(_))) => {
            b.on_error(self.settings.backoff_factor, self.settings.backoff_cap);
            let msg = match &e {
                HttpError::RateLimited       => "rate limited".into(),
                HttpError::ServerError(code) => format!("server error {code}"),
                _                            => unreachable!(),
            };
            (kind, UsageState::Error(msg))
        }
        Err(HttpError::Unauthorized)  => (kind, UsageState::Stale("token expired".into())),
        Err(HttpError::Other(msg))    => (kind, UsageState::Error(msg)),
    }
}

fn refresh_all(&mut self, force: bool) {
    let states: Vec<(ProviderKind, UsageState)> = (0..self.providers.len())
        .map(|i| self.refresh_provider(i, force))
        .collect();
    // ... menu build (same as current refresh()) ...
}
```

- [ ] Update callers: `resumed` → `refresh_all(false)`, update-check block → `refresh_all(false)`.
- [ ] `cargo clippy -- -D warnings && cargo test`
- [ ] Commit: `feat(main): per-provider backoff map and refresh_provider`

---

## Task 6 — Gate polling + manual refresh bypass in `about_to_wait`

**File:** `src/main.rs`

- [ ] Replace `if now >= self.next_poll_at` block:

```rust
let needs_refresh = (0..self.providers.len())
    .any(|i| self.backoff[&self.providers[i].kind()].is_allowed());
if needs_refresh {
    self.refresh_all(false);
    did_refresh = true;
}
```

- [ ] Manual refresh click: change to `self.refresh_all(true)`.
- [ ] Update `WaitUntil` deadline:

```rust
let next_provider = self.backoff.values()
    .map(|b| b.next_allowed_at)
    .min()
    .unwrap_or_else(|| Instant::now() + self.settings.poll_interval);
let next_update = /* existing update-check deadline as Instant */;
event_loop.set_control_flow(ControlFlow::WaitUntil(next_provider.min(next_update)));
```

- [ ] `cargo clippy -- -D warnings && cargo test`
- [ ] Manual smoke: idle CPU ~0% in Activity Monitor, Refresh click works immediately.
- [ ] Commit: `feat(main): gate per-provider poll on backoff; bypass on manual refresh`

---

## Self-review checklist

- [ ] `rg "next_poll_at" src/` → empty (field removed)
- [ ] `rg "catch_unwind" src/` → empty
- [ ] `cargo clippy -- -D warnings && cargo test` green
- [ ] Claude 180s floor: `settings.poll_interval` (300s) is the minimum base; `on_error` only increases it
- [ ] No new crate dependencies
