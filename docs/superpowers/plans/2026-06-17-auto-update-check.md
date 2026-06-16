# Auto-update Check Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Poll GitHub Releases once per 24 wall-clock hours and show `↑ Update available X.Y.Z` at the top of the tray menu when a newer release is available.

**Architecture:** Three layers — `http::get_public` for unauthenticated HTTP, `update_check` module for version comparison and JSON parsing, and changes to `ui::build_menu` and `main.rs` for display and scheduling. The 24h timer uses `DateTime<Local>` (wall-clock) so it advances during sleep.

**Tech Stack:** Rust, `ureq` (already used for HTTP), `serde_json` (already a dep), `chrono::DateTime<Local>` (already used in `App`), `std::process::Command::new("open")` (already used in `about.rs`).

---

### Task 1: `http::get_public` — unauthenticated GET

**Files:**
- Modify: `src/http.rs`

The GitHub releases endpoint is public — no Authorization header. Add a thin variant that reuses the shared agent.

- [ ] **Step 1: Write the failing test**

Add inside the existing `#[cfg(test)]` block in `src/http.rs`:

```rust
#[test]
fn get_public_function_exists_and_compiles() {
    // structural: verifies the function signature is correct
    let _: fn(&str) -> Result<String, super::HttpError> = super::get_public;
}
```

- [ ] **Step 2: Run test to verify it fails**

```
cargo test -p aiusagebar http::tests::get_public_function_exists
```

Expected: FAIL — `get_public` not defined.

- [ ] **Step 3: Implement `get_public`**

Add after the existing `get()` function in `src/http.rs`:

```rust
pub fn get_public(url: &str) -> Result<String, HttpError> {
    let resp = agent()
        .get(url)
        .call()
        .map_err(|e| HttpError::Other(e.to_string()))?;
    match resp.status().as_u16() {
        200 => resp
            .into_body()
            .read_to_string()
            .map_err(|e| HttpError::Other(e.to_string())),
        401 => Err(HttpError::Unauthorized),
        429 => Err(HttpError::RateLimited),
        code => Err(HttpError::Other(format!("HTTP {}", code))),
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

```
cargo test -p aiusagebar http::tests::get_public_function_exists
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/http.rs
git commit -m "feat(http): add get_public for unauthenticated requests"
```

---

### Task 2: `update_check::is_newer` — pure version comparison

**Files:**
- Create: `src/update_check.rs`

`is_newer(current, remote)` is the only testable pure function. Expose as `pub(crate)` so tests outside the module can reach it.

- [ ] **Step 1: Create `src/update_check.rs` with failing tests**

```rust
pub fn check() -> Option<String> {
    todo!()
}

pub(crate) fn is_newer(current: &str, remote: &str) -> bool {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test] fn patch_bump_is_newer()       { assert!( is_newer("0.3.2", "0.3.3")); }
    #[test] fn minor_bump_is_newer()       { assert!( is_newer("0.3.2", "0.4.0")); }
    #[test] fn major_bump_is_newer()       { assert!( is_newer("0.3.2", "1.0.0")); }
    #[test] fn same_version_not_newer()    { assert!(!is_newer("0.4.0", "0.4.0")); }
    #[test] fn current_ahead_not_newer()   { assert!(!is_newer("0.5.0", "0.4.0")); }
    #[test] fn malformed_remote_false()    { assert!(!is_newer("0.3.2", "not-a-version")); }
    #[test] fn empty_remote_false()        { assert!(!is_newer("0.3.2", "")); }
    #[test] fn malformed_current_false()   { assert!(!is_newer("bad", "0.4.0")); }
}
```

- [ ] **Step 2: Declare the module in `src/main.rs`**

Add `mod update_check;` alongside the other `mod` declarations at the top of `src/main.rs`.

- [ ] **Step 3: Run tests to verify they fail**

```
cargo test -p aiusagebar update_check::tests
```

Expected: FAIL — `todo!()` panics.

- [ ] **Step 4: Implement `is_newer`**

Replace the `todo!()` stub:

```rust
pub(crate) fn is_newer(current: &str, remote: &str) -> bool {
    fn parse(v: &str) -> Option<(u32, u32, u32)> {
        let mut parts = v.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse::<u32>().ok()?;
        Some((major, minor, patch))
    }
    match (parse(current), parse(remote)) {
        (Some(c), Some(r)) => r > c,
        _ => false,
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

```
cargo test -p aiusagebar update_check::tests
```

Expected: all 8 tests PASS (the `check` stub will panic if called, but no test calls it yet).

- [ ] **Step 6: Commit**

```bash
git add src/update_check.rs src/main.rs
git commit -m "feat(update_check): add is_newer version comparison"
```

---

### Task 3: `update_check::parse_release` — JSON parsing

**Files:**
- Modify: `src/update_check.rs`

`parse_release(json: &str) -> Option<String>` extracts and validates the version from a GitHub API response body. Pure function — no HTTP, fully testable.

- [ ] **Step 1: Write failing tests**

Add to the `tests` module in `src/update_check.rs`:

```rust
use super::parse_release;

#[test]
fn valid_newer_with_assets_returns_version() {
    let json = r#"{"tag_name":"v0.4.0","assets":[{"name":"aiusagebar-macos-arm64-v0.4.0"}]}"#;
    assert_eq!(parse_release(json), Some("0.4.0".to_owned()));
}

#[test]
fn valid_same_version_with_assets_returns_version() {
    // parse_release only extracts; is_newer decides "new enough"
    let json = r#"{"tag_name":"v0.3.2","assets":[{"name":"aiusagebar-macos-arm64-v0.3.2"}]}"#;
    assert_eq!(parse_release(json), Some("0.3.2".to_owned()));
}

#[test]
fn empty_assets_returns_none() {
    let json = r#"{"tag_name":"v0.4.0","assets":[]}"#;
    assert_eq!(parse_release(json), None);
}

#[test]
fn github_404_body_returns_none() {
    let json = r#"{"message":"Not Found","documentation_url":"https://docs.github.com/rest"}"#;
    assert_eq!(parse_release(json), None);
}

#[test]
fn malformed_json_returns_none() {
    assert_eq!(parse_release("not json at all"), None);
}

#[test]
fn tag_without_v_prefix_returned_as_is() {
    let json = r#"{"tag_name":"0.4.0","assets":[{"name":"bin"}]}"#;
    assert_eq!(parse_release(json), Some("0.4.0".to_owned()));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test -p aiusagebar update_check::tests
```

Expected: FAIL — `parse_release` not defined.

- [ ] **Step 3: Implement `parse_release`**

Add above the `check()` stub in `src/update_check.rs`:

```rust
#[derive(serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<serde_json::Value>,
}

pub(crate) fn parse_release(json: &str) -> Option<String> {
    let release: GithubRelease = serde_json::from_str(json).ok()?;
    if release.assets.is_empty() {
        return None;
    }
    let tag = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
    if tag.is_empty() {
        return None;
    }
    Some(tag.to_owned())
}
```

Add at the top of `src/update_check.rs`:

```rust
use serde::Deserialize;
```

- [ ] **Step 4: Run tests to verify they pass**

```
cargo test -p aiusagebar update_check::tests
```

Expected: all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/update_check.rs
git commit -m "feat(update_check): add parse_release JSON extraction"
```

---

### Task 4: `update_check::check` — HTTP wrapper

**Files:**
- Modify: `src/update_check.rs`

`check()` is a thin composition of `http::get_public` + `parse_release` + `is_newer`. No new tests needed — the pieces are already tested; this just wires them together.

- [ ] **Step 1: Implement `check()`**

Replace the `todo!()` stub in `src/update_check.rs`:

```rust
pub fn check() -> Option<String> {
    let json = crate::http::get_public(
        "https://api.github.com/repos/mttpla/aiusagebar/releases/latest",
    )
    .ok()?;
    let remote = parse_release(&json)?;
    is_newer(env!("CARGO_PKG_VERSION"), &remote).then(|| remote)
}
```

- [ ] **Step 2: Verify it compiles**

```
cargo build
```

Expected: compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add src/update_check.rs
git commit -m "feat(update_check): implement check() via GitHub Releases API"
```

---

### Task 5: UI — update row in tray menu

**Files:**
- Modify: `src/ui/mod.rs`

Two changes:
1. `build_layout` gains `update: Option<&str>` — starts `idx` at 2 when `Some` so `styled::style_menu` indices are correct.
2. `build_menu` gains `update: Option<&str>` — prepends update `MenuItem` + separator when `Some`. `MenuBuild` gains `pub update: Option<MenuId>`.

- [ ] **Step 1: Write failing tests for the offset**

Add to the `tests` module in `src/ui/mod.rs`:

```rust
#[test]
fn build_layout_with_update_shifts_all_indices_by_2() {
    let state = UsageState::Ok(
        vec![LimitWindow { name: "d".into(), ..Default::default() }],
        Some("max".into()),
    );
    let layout = build_layout(&[(ProviderKind::Claude, &state)], None, Some("0.4.0"));
    // header was at 0 without update, now at 2
    assert_eq!(layout.header_indices[0].0, 2);
    // window item was at 1, now at 3
    assert_eq!(layout.window_items[0].0, 3);
    // refresh was at 2 (1 header + 1 window + footer), now at 4
    assert_eq!(layout.refresh_idx, 4);
    assert_eq!(layout.quit_idx, 7);
}

#[test]
fn build_layout_without_update_unchanged() {
    let state = UsageState::Ok(
        vec![LimitWindow { name: "d".into(), ..Default::default() }],
        Some("max".into()),
    );
    let layout = build_layout(&[(ProviderKind::Claude, &state)], None, None);
    assert_eq!(layout.header_indices[0].0, 0);
    assert_eq!(layout.refresh_idx, 2);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test -p aiusagebar ui::tests
```

Expected: FAIL — `build_layout` doesn't accept 3 args yet.

- [ ] **Step 3: Update `build_layout` signature and offset logic**

In `src/ui/mod.rs`, change the `build_layout` signature and its first line:

```rust
pub(crate) fn build_layout(
    states: &[(ProviderKind, &UsageState)],
    last_updated: Option<&str>,
    update: Option<&str>,
) -> MenuLayout {
    let mut idx: usize = if update.is_some() { 2 } else { 0 };
    // rest of function unchanged
```

- [ ] **Step 4: Update all existing `build_layout` callers**

`build_layout` is called once inside `build_menu` in `src/ui/mod.rs`:
```rust
let layout = build_layout(states, last_updated, update);
```

Also update all existing tests that call `build_layout` to pass `None` as the third argument. Each call becomes e.g.:
```rust
let layout = build_layout(&[], None, None);
let layout = build_layout(&[(ProviderKind::Claude, &state)], None, None);
```

- [ ] **Step 5: Expand `MenuBuild` and update `build_menu`**

In `src/ui/mod.rs`, update `MenuBuild`:

```rust
pub struct MenuBuild {
    pub menu: Menu,
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
    pub update: Option<MenuId>,
}
```

Update `build_menu` signature:

```rust
pub fn build_menu(
    states: &[(ProviderKind, &UsageState)],
    last_updated: Option<&str>,
    update: Option<&str>,
) -> MenuBuild {
```

At the start of `build_menu`, before the provider loop, add the update row when present:

```rust
use tray_icon::menu::PredefinedMenuItem;

let update_id: Option<MenuId> = if let Some(version) = update {
    let item = MenuItem::new(format!("↑ Update available {}", version), true, None);
    let id = item.id().clone();
    menu.append(&item).expect("menu append failed");
    menu.append(&PredefinedMenuItem::separator()).expect("menu append failed");
    Some(id)
} else {
    None
};
```

At the end of `build_menu`, add `update: update_id` to the `MenuBuild` return value:

```rust
MenuBuild {
    menu,
    about: footer.about,
    refresh: footer.refresh,
    quit: footer.quit,
    update: update_id,
}
```

- [ ] **Step 6: Run all tests**

```
cargo test -p aiusagebar
```

Expected: all tests PASS.

- [ ] **Step 7: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat(ui): prepend update row to tray menu when newer release available"
```

---

### Task 6: `main.rs` — timer, App fields, event handler

**Files:**
- Modify: `src/main.rs`

Wire up the 24h wall-clock check: new `App` fields, update timer logic in `about_to_wait`, and handle the update row click.

- [ ] **Step 1: Add new fields to `App`**

Update the `App` struct in `src/main.rs`:

```rust
struct App {
    tray: tray_icon::TrayIcon,
    icons: Icons,
    id_about: tray_icon::menu::MenuId,
    id_quit: tray_icon::menu::MenuId,
    id_refresh: tray_icon::menu::MenuId,
    id_update: Option<tray_icon::menu::MenuId>,
    providers: Vec<Box<dyn UsageProvider>>,
    last_refreshed_at: Option<DateTime<Local>>,
    settings: Settings,
    next_poll_at: Instant,
    next_update_check_after: DateTime<Local>,
    update_available: Option<String>,
}
```

- [ ] **Step 2: Update `refresh()` to pass update state**

In `App::refresh()`, update the `build_menu` call to pass the new parameters:

```rust
let build = ui::build_menu(&refs, Some(&updated), self.update_available.as_deref());
self.id_about = build.about;
self.id_refresh = build.refresh;
self.id_quit = build.quit;
self.id_update = build.update;
```

- [ ] **Step 3: Update initial `build_menu` call in `main()`**

In `fn main()`, update the initial call:

```rust
let build = ui::build_menu(&initial_refs, None, None);
```

And add the new fields to the `App` initializer:

```rust
let mut app = App {
    tray,
    icons,
    id_about: build.about,
    id_quit: build.quit,
    id_refresh: build.refresh,
    id_update: build.update,
    providers,
    last_refreshed_at: None,
    settings,
    next_poll_at,
    next_update_check_after: Local::now() + chrono::Duration::hours(24),
    update_available: None,
};
```

Add `use chrono::Duration;` at the top of `src/main.rs` if not already present (chrono is already imported for `DateTime<Local>`).

- [ ] **Step 4: Add update check timer in `about_to_wait`**

In `App::about_to_wait`, add the timer check before the menu event handling:

```rust
fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    let now = Instant::now();
    let mut did_refresh = false;
    if now >= self.next_poll_at {
        self.refresh();
        self.next_poll_at = now + self.settings.poll_interval;
        did_refresh = true;
    }

    if Local::now() >= self.next_update_check_after {
        self.update_available = update_check::check();
        self.next_update_check_after = Local::now() + chrono::Duration::hours(24);
        if !did_refresh {
            self.refresh();
            did_refresh = true;
        }
    }

    if let Ok(ev) = MenuEvent::receiver().try_recv() {
        if ev.id == self.id_quit {
            event_loop.exit();
        } else if ev.id == self.id_about {
            about::show();
        } else if ev.id == self.id_refresh && !did_refresh {
            self.refresh();
            self.next_poll_at = Instant::now() + self.settings.poll_interval;
        } else if self.id_update.as_ref().map_or(false, |id| ev.id == *id) {
            let _ = std::process::Command::new("open")
                .arg("https://github.com/mttpla/aiusagebar/releases/latest")
                .spawn();
        }
    }

    let _ = TrayIconEvent::receiver().try_recv();
    event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_poll_at));
}
```

- [ ] **Step 5: Build and verify**

```
cargo build
```

Expected: compiles with no errors or warnings.

- [ ] **Step 6: Run full test suite**

```
cargo test -p aiusagebar
```

Expected: all tests PASS.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire up 24h update check with wall-clock timer"
```
