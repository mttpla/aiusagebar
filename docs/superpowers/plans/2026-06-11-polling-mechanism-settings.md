# Polling Mechanism & Settings Struct Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add automatic 5-minute background polling via `ControlFlow::WaitUntil`, a central `Settings` struct for runtime constants, and a last-refresh timestamp displayed in the tray menu.

**Architecture:** New `src/settings.rs` holds `Settings` (poll interval, alert threshold) with `Default` impl. `App` gains three fields: `settings`, `next_poll_at`, `last_refreshed_at`. `about_to_wait` checks `Instant::now() >= next_poll_at` and sleeps via `WaitUntil`. `build_menu` gains `last_updated: Option<&str>` — when `Some`, renders a disabled "Updated: HH:MM" item + separator above Refresh/Quit.

**Tech Stack:** Rust std (`std::time::Instant`), `chrono 0.4` (already in Cargo.toml), `tray-icon 0.19` (`PredefinedMenuItem::separator()`)

---

### Task 1: Create `src/settings.rs`

**Files:**
- Create: `src/settings.rs`
- Modify: `src/main.rs` (add `mod settings;`)

- [ ] **Step 1: Create `src/settings.rs` with struct and tests**

```rust
use std::time::Duration;

pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(300);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_poll_interval_is_five_minutes() {
        let s = Settings::default();
        assert_eq!(s.poll_interval, Duration::from_secs(300));
    }

    #[test]
    fn default_alert_threshold_is_eighty_percent() {
        assert_eq!(Settings::default().alert_threshold_pct, 80.0_f32);
    }
}
```

- [ ] **Step 2: Add module declaration to `src/main.rs`**

After the existing `mod launch_at_login;` line, add:
```rust
mod settings;
```

- [ ] **Step 3: Run tests**

```bash
cargo test settings
```

Expected: `test settings::tests::default_poll_interval_is_five_minutes ... ok` and `test settings::tests::default_alert_threshold_is_eighty_percent ... ok`. 2 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/settings.rs src/main.rs
git commit -m "feat: add Settings struct with poll interval and alert threshold defaults"
```

---

### Task 2: Migrate `ALERT_THRESHOLD` from `src/icon.rs`

**Files:**
- Modify: `src/icon.rs`

- [ ] **Step 1: Run existing icon tests as baseline**

```bash
cargo test icon
```

Expected: 12 tests pass.

- [ ] **Step 2: Replace the constant**

In `src/icon.rs`, remove:
```rust
const ALERT_THRESHOLD: f32 = 80.0;
```

Add after `use crate::provider::UsageState;`:
```rust
use crate::settings::DEFAULT_ALERT_THRESHOLD_PCT;
```

On the `for_state` comparison line, change:
```rust
                if windows.iter().any(|w| w.percent_used.unwrap_or(0.0) >= ALERT_THRESHOLD) {
```
to:
```rust
                if windows.iter().any(|w| w.percent_used.unwrap_or(0.0) >= DEFAULT_ALERT_THRESHOLD_PCT) {
```

- [ ] **Step 3: Run tests**

```bash
cargo test icon
```

Expected: same 12 tests pass, no regressions.

- [ ] **Step 4: Commit**

```bash
git add src/icon.rs
git commit -m "refactor: move ALERT_THRESHOLD to settings::DEFAULT_ALERT_THRESHOLD_PCT"
```

---

### Task 3: Update `build_menu` with last-refresh display

**Files:**
- Modify: `src/main.rs`

This task adds `last_updated: Option<&str>` to `build_menu`. Both call sites pass `None` for now — the live value is wired up in Task 4.

- [ ] **Step 1: Add `PredefinedMenuItem` to imports**

In `src/main.rs`, update the tray-icon menu import:
```rust
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIconBuilder, TrayIconEvent,
};
```

- [ ] **Step 2: Update `build_menu` signature**

Change:
```rust
fn build_menu(states: &[(&str, &UsageState)]) -> MenuBuild {
```
to:
```rust
fn build_menu(states: &[(&str, &UsageState)], last_updated: Option<&str>) -> MenuBuild {
```

- [ ] **Step 3: Add timestamp item and separator before Refresh/Quit**

Before the `let item_refresh = MenuItem::new("Refresh", true, None);` line, insert:
```rust
        if let Some(ts) = last_updated {
            // TODO: i18n
            append_label(&menu, format!("Updated: {}", ts));
            menu.append(&PredefinedMenuItem::separator())
                .expect("menu append failed");
        }
```

- [ ] **Step 4: Fix call site in `main()`**

Change:
```rust
let build = App::build_menu(&initial_refs);
```
to:
```rust
let build = App::build_menu(&initial_refs, None);
```

- [ ] **Step 5: Fix call site in `refresh()`**

Change:
```rust
let build = Self::build_menu(&refs);
```
to:
```rust
let build = Self::build_menu(&refs, None);
```

- [ ] **Step 6: Verify compilation**

```bash
cargo check
```

Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs
git commit -m "feat: add last-refresh timestamp slot to build_menu"
```

---

### Task 4: Record last-refresh timestamp in `App`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add chrono import**

Add near the top of `src/main.rs` after the existing `use` lines:
```rust
use chrono::{DateTime, Local};
```

- [ ] **Step 2: Add `last_refreshed_at` to `App` struct**

Change the `App` struct to:
```rust
struct App {
    tray: tray_icon::TrayIcon,
    icons: Icons,
    id_quit: tray_icon::menu::MenuId,
    id_refresh: tray_icon::menu::MenuId,
    providers: Vec<Box<dyn UsageProvider>>,
    last_refreshed_at: Option<DateTime<Local>>,
}
```

- [ ] **Step 3: Initialize field in `main()`**

Update the `App { ... }` block:
```rust
let mut app = App {
    tray,
    icons,
    id_quit: build.quit,
    id_refresh: build.refresh,
    providers,
    last_refreshed_at: None,
};
```

- [ ] **Step 4: Record timestamp at end of `refresh()`**

After `self.tray.set_icon(Some(self.icons.get(icon_kind))).ok();`, add:
```rust
        self.last_refreshed_at = Some(Local::now());
```

- [ ] **Step 5: Pass live timestamp to `build_menu` in `refresh()`**

Replace the `None` placeholder. Change:
```rust
let build = Self::build_menu(&refs, None);
```
to:
```rust
let updated = self.last_refreshed_at.as_ref().map(|t| t.format("%H:%M").to_string());
let build = Self::build_menu(&refs, updated.as_deref());
```

- [ ] **Step 6: Verify compilation**

```bash
cargo check
```

Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs
git commit -m "feat: record and display last-refresh timestamp in tray menu"
```

---

### Task 5: Implement automatic polling via `WaitUntil`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add imports**

Add to the top of `src/main.rs`:
```rust
use std::time::Instant;
use settings::Settings;
```

- [ ] **Step 2: Add `settings` and `next_poll_at` to `App` struct**

```rust
struct App {
    tray: tray_icon::TrayIcon,
    icons: Icons,
    id_quit: tray_icon::menu::MenuId,
    id_refresh: tray_icon::menu::MenuId,
    providers: Vec<Box<dyn UsageProvider>>,
    last_refreshed_at: Option<DateTime<Local>>,
    settings: Settings,
    next_poll_at: Instant,
}
```

- [ ] **Step 3: Initialize new fields in `main()`**

Before the `App { ... }` block, add:
```rust
let settings = Settings::default();
let next_poll_at = Instant::now() + settings.poll_interval;
```

Update the `App { ... }` block:
```rust
let mut app = App {
    tray,
    icons,
    id_quit: build.quit,
    id_refresh: build.refresh,
    providers,
    last_refreshed_at: None,
    settings,
    next_poll_at,
};
```

- [ ] **Step 4: Rewrite `about_to_wait`**

Replace the entire body:
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

- [ ] **Step 5: Verify compilation**

```bash
cargo check
```

Expected: no errors.

- [ ] **Step 6: Run full test suite**

```bash
cargo test
```

Expected: all 14 tests pass (12 icon + 2 settings).

- [ ] **Step 7: Smoke test**

```bash
make dev
```

Verify in Activity Monitor that idle CPU stays ~0%. Click Refresh — "Updated: HH:MM" appears above separator before Refresh/Quit. After 5 minutes, the menu auto-updates without manual interaction.

- [ ] **Step 8: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement automatic polling via WaitUntil with Settings-driven interval"
```
