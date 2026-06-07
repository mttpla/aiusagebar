# Code Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply all findings from the 2026-06-07 code review: close token/body log leaks, add HTTP timeout, harden credential error reporting, eliminate per-refresh PNG decode, tighten ergonomics around `build_menu` / `ClaudeProvider`, and clean up `launch_at_login` magic numbers + spec drift.

**Architecture:** No new modules. Surgical edits across `src/main.rs`, `src/http.rs`, `src/provider/claude.rs`, `src/provider/mod.rs`, `src/launch_at_login.rs`, `build.rs`, `Cargo.toml`. Plus a new file `src/icon.rs` that encapsulates pre-parsed `tray_icon::Icon` cache (spec drift fix). TDD where logic exists; mechanical refactors covered by existing 12 tests + new tests for new behavior.

**Tech Stack:** Rust 2021, `tray-icon`, `winit`, `reqwest` (blocking, rustls-tls), `serde_json`, `chrono`, `security-framework`.

---

## File Map

| File | Change | Reason |
|---|---|---|
| `src/http.rs` | Modify | Drop body/401 `eprintln!`, add shared `Client` with timeout |
| `src/provider/claude.rs` | Modify | Drop token `eprintln!`, harden cred error variants, Local TZ date, hoist `use`, `Default`, `let-else`, `[_; 2]` |
| `src/provider/mod.rs` | Modify | Derive `PartialEq` on `UsageState` |
| `src/icon.rs` | Create | Pre-parsed icon cache + `IconKind::for_state` |
| `src/main.rs` | Modify | Use `icon::Icons`, struct return from `build_menu`, helper, `Eq` derive |
| `src/launch_at_login.rs` | Modify | Named exit codes, content-compare before rewrite, narrow `allow(dead_code)` |
| `build.rs` | Modify | Drop misleading Italian comment |
| `Cargo.toml` | Modify | `chrono` default-features off |

---

## Task 1 — Security: remove debug `eprintln!` lines

**Files:**
- Modify: `src/http.rs:22, 26-27`
- Modify: `src/provider/claude.rs:134-136`

- [ ] **Step 1: Delete body+401 log lines in `src/http.rs`**

Edit `src/http.rs` so the `match` block contains no `eprintln!`:

```rust
    match resp.status().as_u16() {
        200 => resp.text().map_err(|e| HttpError::Other(e.to_string())),
        401 => Err(HttpError::Unauthorized),
        429 => Err(HttpError::RateLimited),
        code => Err(HttpError::Other(format!("HTTP {}", code))),
    }
```

- [ ] **Step 2: Delete token+expiry+UA log lines in `src/provider/claude.rs`**

In `fn fetch`, remove the three `eprintln!("[debug] …")` lines (currently L134-136). The body becomes:

```rust
        let ua = get_user_agent();
        match crate::http::get(USAGE_URL, &creds.access_token, &[("User-Agent", ua)]) {
```

- [ ] **Step 3: Verify with grep that no `eprintln!` containing "token", "body", or "[debug]" remains in `src/`**

Run: `grep -RnE 'eprintln!.*(token|\[debug\]|body)' src/`
Expected: zero matches.

- [ ] **Step 4: Run full test suite**

Run: `cargo test --lib`
Expected: all 12 existing tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/http.rs src/provider/claude.rs
git commit -m "security: stop logging tokens and response bodies to stderr"
```

---

## Task 2 — HTTP: shared client + 15s timeout

**Files:**
- Modify: `src/http.rs` (full file)

- [ ] **Step 1: Write the failing test**

Append to `src/http.rs` tests module:

```rust
    #[test]
    fn shared_client_is_reused() {
        let a = super::client() as *const reqwest::blocking::Client;
        let b = super::client() as *const reqwest::blocking::Client;
        assert_eq!(a, b, "client() must return the same instance across calls");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib http::tests::shared_client_is_reused`
Expected: FAIL — `client` is undefined.

- [ ] **Step 3: Replace the body of `src/http.rs`**

Full file content:

```rust
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub enum HttpError {
    Unauthorized,
    RateLimited,
    Other(String),
}

pub fn client() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("failed to build HTTP client")
    })
}

pub fn get(url: &str, token: &str, extra_headers: &[(&str, &str)]) -> Result<String, HttpError> {
    let mut builder = client()
        .get(url)
        .header("Authorization", format!("Bearer {}", token));
    for (name, value) in extra_headers {
        builder = builder.header(*name, *value);
    }
    let resp = builder.send().map_err(|e| HttpError::Other(e.to_string()))?;
    match resp.status().as_u16() {
        200 => resp.text().map_err(|e| HttpError::Other(e.to_string())),
        401 => Err(HttpError::Unauthorized),
        429 => Err(HttpError::RateLimited),
        code => Err(HttpError::Other(format!("HTTP {}", code))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_error_variants_are_distinct() {
        assert_ne!(HttpError::Unauthorized, HttpError::RateLimited);
    }

    #[test]
    fn http_error_other_carries_message() {
        let e = HttpError::Other("boom".to_string());
        assert_eq!(e, HttpError::Other("boom".to_string()));
    }

    #[test]
    fn shared_client_is_reused() {
        let a = super::client() as *const reqwest::blocking::Client;
        let b = super::client() as *const reqwest::blocking::Client;
        assert_eq!(a, b, "client() must return the same instance across calls");
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib http::`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/http.rs
git commit -m "http: reuse client and apply 15s timeout"
```

---

## Task 3 — UA parser: defensive version extraction

**Files:**
- Modify: `src/provider/claude.rs:63-73`

Current behavior works (`claude --version` → `2.1.153 (Claude Code)` → first token = `2.1.153`). Goal: tolerate future format changes like `Claude Code 2.1.153` without 429-ing the user.

- [ ] **Step 1: Write the failing tests**

In `src/provider/claude.rs` `tests` module, append:

```rust
    #[test]
    fn parse_version_first_token_numeric() {
        assert_eq!(super::parse_version("2.1.153 (Claude Code)"), Some("2.1.153".to_string()));
    }

    #[test]
    fn parse_version_skips_leading_words() {
        assert_eq!(super::parse_version("Claude Code 2.1.153"), Some("2.1.153".to_string()));
    }

    #[test]
    fn parse_version_none_on_empty() {
        assert_eq!(super::parse_version(""), None);
    }
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --lib provider::claude::tests::parse_version`
Expected: FAIL — `parse_version` undefined.

- [ ] **Step 3: Implement `parse_version` and wire it into `get_user_agent`**

Replace `get_user_agent` and add the helper:

```rust
fn parse_version(s: &str) -> Option<String> {
    s.split_whitespace()
        .find(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .map(|t| t.trim_matches(|c: char| !c.is_ascii_digit() && c != '.').to_string())
        .filter(|t| !t.is_empty())
}

fn get_user_agent() -> &'static str {
    USER_AGENT.get_or_init(|| {
        std::process::Command::new("claude")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .as_deref()
            .and_then(parse_version)
            .map(|v| format!("claude-code/{}", v))
            .unwrap_or_else(|| "claude-code/2.1.153".to_string())
    })
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib provider::claude::`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/provider/claude.rs
git commit -m "claude: harden user-agent version parser"
```

---

## Task 4 — Credentials: distinguish corrupt JSON from missing file

**Files:**
- Modify: `src/provider/claude.rs:22-38, 124-128`

- [ ] **Step 1: Write the failing tests**

Append to `tests`:

```rust
    #[test]
    fn load_result_missing_is_not_configured() {
        // Using a never-existing path via a fresh helper
        assert!(matches!(super::parse_credentials_payload(None), super::CredLoad::NotConfigured));
    }

    #[test]
    fn load_result_corrupt_is_malformed() {
        let bad = Some("{not json".to_string());
        assert!(matches!(super::parse_credentials_payload(bad), super::CredLoad::Malformed(_)));
    }

    #[test]
    fn load_result_valid_is_ok() {
        let good = Some(r#"{"claudeAiOauth":{"accessToken":"t","expiresAt":1}}"#.to_string());
        assert!(matches!(super::parse_credentials_payload(good), super::CredLoad::Ok(_)));
    }
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --lib provider::claude::tests::load_result`
Expected: FAIL — `CredLoad` / `parse_credentials_payload` undefined.

- [ ] **Step 3: Add `CredLoad` enum + `parse_credentials_payload`, rewire `fetch`**

In `src/provider/claude.rs`, replace `pub fn load_credentials` and friends with:

```rust
pub enum CredLoad {
    NotConfigured,
    Malformed(String),
    Ok(ClaudeCredentials),
}

pub fn parse_credentials_payload(json: Option<String>) -> CredLoad {
    let Some(json) = json else { return CredLoad::NotConfigured; };
    match serde_json::from_str::<CredentialsFile>(&json) {
        Ok(file) => CredLoad::Ok(ClaudeCredentials {
            access_token: file.claude_ai_oauth.access_token,
            expires_at_ms: file.claude_ai_oauth.expires_at,
        }),
        Err(e) => CredLoad::Malformed(e.to_string()),
    }
}

pub fn load_credentials() -> CredLoad {
    parse_credentials_payload(load_credentials_json())
}
```

In `impl UsageProvider for ClaudeProvider::fetch`, replace the credential match:

```rust
        let creds = match load_credentials() {
            CredLoad::NotConfigured => return UsageState::NotConfigured,
            CredLoad::Malformed(e) => return UsageState::Error(format!("Malformed credentials: {}", e)),
            CredLoad::Ok(c) => c,
        };
```

- [ ] **Step 4: Run all claude tests**

Run: `cargo test --lib provider::claude::`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/provider/claude.rs
git commit -m "claude: surface malformed credentials as Error, not NotConfigured"
```

---

## Task 5 — `format_expiry_date` uses local timezone

**Files:**
- Modify: `src/provider/claude.rs:48-55`
- Modify: existing test `format_expiry_date_known_timestamp` (will be removed; replaced)

- [ ] **Step 1: Update existing test to assert on local conversion**

Replace `format_expiry_date_known_timestamp` test with a TZ-independent shape check:

```rust
    #[test]
    fn format_expiry_date_yyyy_mm_dd_shape() {
        let s = format_expiry_date(1749081600000);
        let bytes = s.as_bytes();
        assert_eq!(bytes.len(), 10, "got {s}");
        assert_eq!(bytes[4], b'-');
        assert_eq!(bytes[7], b'-');
    }
```

- [ ] **Step 2: Run test to confirm current (UTC) impl still passes**

Run: `cargo test --lib provider::claude::tests::format_expiry_date_yyyy_mm_dd_shape`
Expected: PASS (10-char `YYYY-MM-DD`).

- [ ] **Step 3: Switch implementation to `chrono::Local`**

Replace `format_expiry_date`:

```rust
pub fn format_expiry_date(expires_at_ms: u64) -> String {
    use chrono::{Local, TimeZone};
    let secs = (expires_at_ms / 1000) as i64;
    match Local.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d").to_string(),
        _ => "?".to_string(),
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib provider::claude::`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/provider/claude.rs
git commit -m "claude: format expiry date in local timezone"
```

---

## Task 6 — `UsageState`: derive `PartialEq`; windows array; misc claude polish

**Files:**
- Modify: `src/provider/mod.rs:13`
- Modify: `src/provider/claude.rs` (top imports, `parse_response` return type, `Default`)

- [ ] **Step 1: Derive `PartialEq` on `UsageState`**

In `src/provider/mod.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum UsageState {
    NotConfigured,
    Stale(String),
    Ok(Vec<LimitWindow>),
    Error(String),
}
```

- [ ] **Step 2: Hoist `use` lines to top of `src/provider/claude.rs`**

Move L57-59 to the top of the file (after the existing `use serde::Deserialize;`):

```rust
use serde::Deserialize;
use std::sync::{Mutex, OnceLock};
use crate::http::HttpError;
use crate::provider::{LimitWindow, UsageState, UsageProvider};
```

Delete the mid-file `use` block.

- [ ] **Step 3: Change `parse_response` to return `[LimitWindow; 2]`**

```rust
fn parse_response(body: &str) -> Result<[LimitWindow; 2], String> {
    let resp: UsageResponse = serde_json::from_str(body).map_err(|e| e.to_string())?;
    Ok([
        LimitWindow {
            name: "5h session".to_string(),
            percent_used: Some(resp.five_hour.utilization),
            limit: None,
            remaining: None,
            resets_at: Some(resp.five_hour.resets_at),
            unlimited: false,
        },
        LimitWindow {
            name: "7d weekly".to_string(),
            percent_used: Some(resp.seven_day.utilization),
            limit: None,
            remaining: None,
            resets_at: Some(resp.seven_day.resets_at),
            unlimited: false,
        },
    ])
}
```

Inside `fetch`, where the parsed windows are used, materialize into `Vec` for `UsageState::Ok` and the cache:

```rust
            Ok(body) => match parse_response(&body) {
                Ok(windows) => {
                    let windows = windows.to_vec();
                    *self.last_ok.lock().unwrap() = Some(windows.clone());
                    UsageState::Ok(windows)
                }
                Err(e) => UsageState::Error(format!("Parse error: {}", e)),
            },
```

Update test assertion `assert_eq!(windows.len(), 2);` to still work — `[T;2]` has `.len()` so it stays valid.

- [ ] **Step 4: Replace `new()` with `Default`**

```rust
impl Default for ClaudeProvider {
    fn default() -> Self {
        Self { last_ok: Mutex::new(None) }
    }
}

impl ClaudeProvider {
    pub fn new() -> Self { Self::default() }
}
```

(Keep `new` as a convenience wrapper — `main.rs` already calls it.)

- [ ] **Step 5: Replace `match … None/Some` with `let-else` for credentials check**

Already adjusted in Task 4; verify the final shape in `fetch` is:

```rust
        let creds = match load_credentials() {
            CredLoad::NotConfigured => return UsageState::NotConfigured,
            CredLoad::Malformed(e) => return UsageState::Error(format!("Malformed credentials: {}", e)),
            CredLoad::Ok(c) => c,
        };
        if is_expired(creds.expires_at_ms) {
            let date = format_expiry_date(creds.expires_at_ms);
            return UsageState::Stale(format!("Expired on {} — run: claude login", date));
        }
```

- [ ] **Step 6: Run tests**

Run: `cargo test --lib`
Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add src/provider/mod.rs src/provider/claude.rs
git commit -m "provider: derive PartialEq, fixed-size windows array, Default impl"
```

---

## Task 7 — Pre-parsed icon cache (spec drift fix)

**Files:**
- Create: `src/icon.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write the failing test for `IconKind::for_state`**

Create `src/icon.rs`:

```rust
use crate::provider::UsageState;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IconKind {
    Normal,
    Alert,
    Unavailable,
}

const ALERT_THRESHOLD: f32 = 80.0;

impl IconKind {
    pub fn for_state(state: &UsageState) -> Self {
        match state {
            UsageState::Ok(windows) => {
                if windows.iter().any(|w| w.percent_used.unwrap_or(0.0) >= ALERT_THRESHOLD) {
                    IconKind::Alert
                } else {
                    IconKind::Normal
                }
            }
            _ => IconKind::Unavailable,
        }
    }
}

static ICON_NORMAL_PNG: &[u8] = include_bytes!("../icons/brain_normal.png");
static ICON_ALERT_PNG: &[u8] = include_bytes!("../icons/brain_alert.png");
static ICON_UNAVAILABLE_PNG: &[u8] = include_bytes!("../icons/brain_unavailable.png");

pub struct Icons {
    normal: tray_icon::Icon,
    alert: tray_icon::Icon,
    unavailable: tray_icon::Icon,
}

impl Icons {
    pub fn load() -> Self {
        Self {
            normal: parse(ICON_NORMAL_PNG),
            alert: parse(ICON_ALERT_PNG),
            unavailable: parse(ICON_UNAVAILABLE_PNG),
        }
    }

    pub fn get(&self, kind: IconKind) -> tray_icon::Icon {
        match kind {
            IconKind::Normal => self.normal.clone(),
            IconKind::Alert => self.alert.clone(),
            IconKind::Unavailable => self.unavailable.clone(),
        }
    }
}

fn parse(bytes: &[u8]) -> tray_icon::Icon {
    let img = image::load_from_memory(bytes)
        .expect("failed to decode icon")
        .into_rgba8();
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).expect("failed to create icon")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::LimitWindow;

    fn window(pct: Option<f32>) -> LimitWindow {
        LimitWindow {
            name: "t".into(),
            percent_used: pct,
            limit: None,
            remaining: None,
            resets_at: None,
            unlimited: false,
        }
    }

    #[test]
    fn normal_under_threshold() {
        let s = UsageState::Ok(vec![window(Some(79.9))]);
        assert_eq!(IconKind::for_state(&s), IconKind::Normal);
    }

    #[test]
    fn alert_at_threshold() {
        let s = UsageState::Ok(vec![window(Some(80.0))]);
        assert_eq!(IconKind::for_state(&s), IconKind::Alert);
    }

    #[test]
    fn unavailable_on_error() {
        assert_eq!(IconKind::for_state(&UsageState::Error("x".into())), IconKind::Unavailable);
    }

    #[test]
    fn unavailable_on_stale() {
        assert_eq!(IconKind::for_state(&UsageState::Stale("x".into())), IconKind::Unavailable);
    }

    #[test]
    fn unavailable_on_not_configured() {
        assert_eq!(IconKind::for_state(&UsageState::NotConfigured), IconKind::Unavailable);
    }

    #[test]
    fn normal_when_percent_unknown() {
        let s = UsageState::Ok(vec![window(None)]);
        assert_eq!(IconKind::for_state(&s), IconKind::Normal);
    }
}
```

- [ ] **Step 2: Wire `icon` module into `main.rs` and delete inline duplicates**

In `src/main.rs`:

1. Add `mod icon;` at the top with the other `mod` declarations.
2. Delete: `enum IconKind { … }`, `fn icon_for_state`, `static ICON_*: &[u8]` blocks, `fn parse_icon`, the inline `tests` mod entries that duplicate icon tests.
3. Import: `use icon::{Icons, IconKind};`
4. Change `struct App` to own the `Icons` cache:

```rust
struct App {
    tray: tray_icon::TrayIcon,
    icons: Icons,
    id_quit: tray_icon::menu::MenuId,
    id_refresh: tray_icon::menu::MenuId,
    claude: ClaudeProvider,
}
```

5. In `refresh`, use the cache:

```rust
    fn refresh(&mut self) {
        let state = self.claude.fetch();
        let build = Self::build_menu(self.claude.name(), &state);
        self.id_refresh = build.refresh;
        self.id_quit = build.quit;
        self.tray.set_menu(Some(Box::new(build.menu)));
        let kind = IconKind::for_state(&state);
        self.tray.set_icon(Some(self.icons.get(kind))).ok();
    }
```

6. In `main()`, build `Icons` once and load the initial icon from it:

```rust
    let icons = Icons::load();
    let claude = ClaudeProvider::new();
    let initial_state = UsageState::NotConfigured;
    let build = App::build_menu(claude.name(), &initial_state);
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(build.menu))
        .with_tooltip("AIUsageBar")
        .with_icon(icons.get(IconKind::for_state(&initial_state)))
        .build()
        .expect("failed to create tray icon");

    let mut app = App {
        tray,
        icons,
        id_quit: build.quit,
        id_refresh: build.refresh,
        claude,
    };
```

(`build_menu` returning a struct is finalized in Task 8 — for now the struct alias is `MenuBuild` defined there. If executed strictly in order, keep returning a tuple in this task and update both call sites in Task 8.)

- [ ] **Step 3: Run all tests**

Run: `cargo test --lib`
Expected: all pass (existing main.rs icon tests now live in `icon::tests`).

- [ ] **Step 4: Commit**

```bash
git add src/icon.rs src/main.rs
git commit -m "icon: pre-parse icons once, move IconKind to dedicated module"
```

---

## Task 8 — `build_menu`: struct return + helper

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add `MenuBuild` struct and helper**

Add near the top of `src/main.rs` (above `impl App`):

```rust
struct MenuBuild {
    menu: Menu,
    refresh: tray_icon::menu::MenuId,
    quit: tray_icon::menu::MenuId,
}

fn append_label(menu: &Menu, text: impl Into<String>) {
    menu.append(&MenuItem::new(text.into(), false, None))
        .expect("menu append failed");
}
```

- [ ] **Step 2: Rewrite `build_menu` to use struct + helper**

```rust
    fn build_menu(name: &str, state: &UsageState) -> MenuBuild {
        let menu = Menu::new();
        match state {
            UsageState::NotConfigured => append_label(&menu, format!("{}: not configured", name)),
            UsageState::Stale(msg) => append_label(&menu, format!("{} ⚠  {}", name, msg)),
            UsageState::Error(msg) => append_label(&menu, format!("{} ✕  {}", name, msg)),
            UsageState::Ok(windows) => {
                append_label(&menu, name.to_string());
                for w in windows {
                    let pct = w
                        .percent_used
                        .map(|p| format!("{:.1}%", p))
                        .unwrap_or_else(|| "∞".to_string());
                    let reset = w.resets_at.as_deref().unwrap_or("?");
                    append_label(&menu, format!("  {} — {}  resets {}", w.name, pct, reset));
                }
            }
        }
        let item_refresh = MenuItem::new("Refresh", true, None);
        let item_quit = MenuItem::new("Quit", true, None);
        menu.append(&item_refresh).expect("menu append failed");
        menu.append(&item_quit).expect("menu append failed");
        MenuBuild {
            refresh: item_refresh.id().clone(),
            quit: item_quit.id().clone(),
            menu,
        }
    }
```

- [ ] **Step 3: Update call sites to use struct fields (already partly in Task 7 — verify)**

Both `App::refresh` and `main()` should consume `MenuBuild { menu, refresh, quit }`.

- [ ] **Step 4: Document `about_to_wait` wake dependency**

In `about_to_wait`, prepend a one-line comment:

```rust
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Cocoa wakes the loop on tray/menu clicks; try_recv drains queued events.
        event_loop.set_control_flow(ControlFlow::Wait);
```

- [ ] **Step 5: Build and run tests**

Run: `cargo build && cargo test --lib`
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "main: build_menu returns struct, append helper, document wake path"
```

---

## Task 9 — `launch_at_login`: named exit codes + content-compare + narrow allow

**Files:**
- Modify: `src/launch_at_login.rs`

- [ ] **Step 1: Replace top-of-file `#![allow(dead_code)]`**

Delete L1 (`#![allow(dead_code)]`). Annotate only `disable`/`is_enabled`:

```rust
#[allow(dead_code)]
pub fn disable() -> Result<(), String> { … }

#[allow(dead_code)]
pub fn is_enabled() -> bool { … }
```

- [ ] **Step 2: Name the magic launchctl exit codes**

Above `enable` add:

```rust
// launchctl exit codes we treat as benign.
const LAUNCHCTL_ALREADY_LOADED: i32 = 36;
const LAUNCHCTL_NOT_LOADED: i32 = 3;
```

Replace `code != 36` and `code == 36 || code == 3` accordingly:

```rust
    if !out.status.success() && code != LAUNCHCTL_ALREADY_LOADED {
```

```rust
    let result = if out.status.success() || code == LAUNCHCTL_ALREADY_LOADED || code == LAUNCHCTL_NOT_LOADED {
```

- [ ] **Step 3: Skip plist rewrite when content unchanged**

In the release `enable()`, replace the unconditional write with:

```rust
    let content = plist_content(binary);
    let needs_write = std::fs::read_to_string(&plist).map(|cur| cur != content).unwrap_or(true);
    if needs_write {
        if let Some(parent) = plist.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&plist, &content).map_err(|e| e.to_string())?;
    }
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib launch_at_login::`
Expected: existing `plist_content_contains_label_and_binary` passes.

- [ ] **Step 5: Commit**

```bash
git add src/launch_at_login.rs
git commit -m "launch_at_login: name exit codes, skip identical plist writes"
```

---

## Task 10 — Build script + Cargo cleanup

**Files:**
- Modify: `build.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: Replace misleading Italian comment in `build.rs`**

```rust
fn main() {
    // Set the minimum macOS deployment target for the linked binary.
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=11.0");
}
```

- [ ] **Step 2: Trim `chrono` features in `Cargo.toml`**

Change:

```toml
chrono = { version = "0.4", default-features = false, features = ["clock"] }
```

- [ ] **Step 3: Build to confirm `chrono::Local` still works without default features**

Run: `cargo build`
Expected: clean build.

- [ ] **Step 4: Run all tests**

Run: `cargo test --lib`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add build.rs Cargo.toml
git commit -m "build: clarify build.rs comment, slim chrono default features"
```

---

## Task 11 — Final verification

- [ ] **Step 1: Clippy pass**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: no warnings. If `Default` + manual `new` triggers `new_without_default`, that's intentional — allow it inline with `#[allow(clippy::new_without_default)]` on the `impl ClaudeProvider` block.

- [ ] **Step 2: Full test run**

Run: `cargo test --lib`
Expected: ≥ 12 tests pass (existing) + ≥ 9 new (3 `parse_version`, 3 `load_result_*`, plus migrated `format_expiry_date_yyyy_mm_dd_shape`, `shared_client_is_reused`, and the 6 `icon::tests` replacing main.rs's 7 — net result: 21+ tests).

- [ ] **Step 3: Smoke test the binary**

Run: `make dev`
Manual check: tray loads, Claude provider shows windows, idle CPU ~0%, no token/body in `Console.app` for `aiusagebar` process.

- [ ] **Step 4: Final commit if any cleanup needed**

```bash
git status
git log --oneline -15
```

Expected: 10 review-fix commits on `master`. No untracked source changes.

---

## Self-Review

**Spec coverage:**
- Token leak (review item 1) → Task 1 ✓
- Body leak (review item 2) → Task 1 ✓
- HTTP timeout (review item 3) → Task 2 ✓
- UA parser defensive (review item — demoted) → Task 3 ✓
- Corrupt creds → Error (review item 4) → Task 4 ✓
- Local TZ date (review item) → Task 5 ✓
- `UsageState` PartialEq + windows array + Default + let-else + use hoist (review items 6, 7) → Task 6 ✓
- Pre-parsed icons / spec drift (review item 5) → Task 7 ✓
- `build_menu` struct + helper + Eq + `about_to_wait` comment (review items 8, 9, 10) → Task 7 + Task 8 ✓
- shared HTTP client (review item) → Task 2 ✓
- launch_at_login magic numbers + content compare + narrow allow (review items 13, 14, 17) → Task 9 ✓
- build.rs comment + chrono features (review items 18, 19) → Task 10 ✓
- USER env fallback — **NOT addressed** in this plan. Reason: `std::env::var("USER")` failing on macOS is exceedingly rare (every login shell sets it). Adding `libc::getpwuid` adds an unsafe FFI dependency for negligible benefit. **Documented decision: leave as-is.**
- Dev build error alert per REQUIREMENTS §8.2 — **NOT addressed** in this plan. Reason: requires NSAlert wiring + UI surface that doesn't exist yet (Settings submenu also not built). Tracked for the Settings/Preferences plan, not this cleanup.

**Placeholder scan:** no TBD/TODO/etc. All code blocks are concrete. All file paths exact.

**Type consistency:** `MenuBuild { menu, refresh, quit }` used identically in Task 7 + Task 8. `CredLoad::{NotConfigured, Malformed, Ok}` introduced in Task 4, consumed in `fetch` in same task. `Icons` + `IconKind::for_state` introduced together in Task 7. `parse_version` signature `(&str) -> Option<String>` matches test calls.
