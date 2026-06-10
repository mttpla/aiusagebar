# Polling Mechanism & Settings Struct

**Date:** 2026-06-10
**Status:** Approved

## Problem

The app currently has no automatic polling. Data refreshes only on manual "Refresh" click or app launch. The `ALERT_THRESHOLD` constant lives in `src/icon.rs` with no obvious home. Future features (per-provider intervals, enable/disable toggles, configurable threshold) need a central place for runtime settings.

## Scope

- Introduce `src/settings.rs` with a `Settings` struct
- Move `ALERT_THRESHOLD` from `src/icon.rs` to `settings.rs`
- Implement automatic polling via `ControlFlow::WaitUntil` in the winit event loop
- Track and display last-refresh timestamp in the menu

**Out of scope:** settings UI, JSON persistence, per-provider intervals, enable/disable toggles.

## Design

### `src/settings.rs`

New module. No I/O, no serde — just the struct and defaults.

```rust
use std::time::Duration;

pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(300); // 5 min
pub const DEFAULT_ALERT_THRESHOLD_PCT: f32 = 80.0;

pub struct Settings {
    pub poll_interval: Duration,
    pub alert_threshold_pct: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            poll_interval: DEFAULT_POLL_INTERVAL,
            alert_threshold_pct: DEFAULT_ALERT_THRESHOLD_PCT,
        }
    }
}
```

The `pub const` values remain importable independently so `icon.rs` can reference them directly until `for_state` is refactored to accept a threshold parameter.

### Migration: `src/icon.rs`

Remove `const ALERT_THRESHOLD: f32 = 80.0;`. Replace with `use crate::settings::DEFAULT_ALERT_THRESHOLD_PCT` and update the single usage site.

### `App` struct changes (`src/main.rs`)

Two new fields:

```rust
struct App {
    // ...existing fields unchanged...
    settings: Settings,
    next_poll_at: Instant,
    last_refreshed_at: Option<chrono::DateTime<chrono::Local>>,
}
```

Initialization in `main()`:

```rust
let settings = Settings::default();
let next_poll_at = Instant::now() + settings.poll_interval;
// ...
let mut app = App {
    // ...existing fields...
    settings,
    next_poll_at,
    last_refreshed_at: None,
};
```

`next_poll_at` starts one full interval after launch because `resumed()` already fires an immediate `refresh()` on startup.

### `refresh()` change

At the end of `refresh()`, record the timestamp:

```rust
self.last_refreshed_at = Some(chrono::Local::now());
```

### `build_menu` change

Add `last_updated: Option<&str>` parameter. If `Some`, append a disabled menu item before Refresh/Quit:

```
  Updated: 14:32        ← grayed-out, non-interactive (disabled NSMenuItem)
  ──────────────
  Refresh
  Quit
```

Format: `"Updated: HH:MM"` — minutes sufficient at 5-min polling cadence. String is hardcoded in English pending i18n pass (marked TODO in code).

Disabled items (`MenuItem::new(text, false, None)`) render gray in NSMenu natively — no extra styling needed.

### `about_to_wait` rewrite (`src/main.rs`)

```rust
fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    let now = Instant::now();
    if now >= self.next_poll_at {
        self.refresh();
        self.next_poll_at = now + self.settings.poll_interval;
    }

    if let Ok(ev) = MenuEvent::receiver().try_recv() {
        if ev.id == self.id_quit {
            event_loop.exit();
        } else if ev.id == self.id_refresh {
            self.refresh();
            self.next_poll_at = Instant::now() + self.settings.poll_interval;
        }
    }

    let _ = TrayIconEvent::receiver().try_recv();
    event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_poll_at));
}
```

Key invariants:
- `set_control_flow` called last — `next_poll_at` is already final at that point
- Manual "Refresh" resets the countdown — user just fetched, no reason to poll again immediately
- CPU idle remains ~0%: the loop sleeps in the OS scheduler between wakeups

## Future evolution

When settings become runtime-configurable (JSON on disk):
1. Add `#[derive(Serialize, Deserialize)]` to `Settings`
2. Add `Settings::load() -> Self` that reads `~/.config/aiusagebar/settings.json`, falls back to `Default::default()`
3. `for_state` / `for_providers` in `icon.rs` accept `threshold: f32` parameter instead of the const

No structural changes to `App` or `about_to_wait` required.
