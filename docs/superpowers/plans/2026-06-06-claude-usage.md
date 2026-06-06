# Claude Usage Display Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fetch Claude Code quota usage from `api.anthropic.com/api/oauth/usage` and display parsed windows in the macOS menu bar menu.

**Architecture:** Four focused modules (`provider/mod.rs`, `provider/claude.rs`, `http.rs`, `keychain.rs`) plus updates to `main.rs`. The HTTP and Keychain modules are provider-agnostic and will be reused by Codex and Copilot. Menu shows one item per `LimitWindow`; no polling timer in this plan (follow-up). Manual "Aggiorna" item triggers re-fetch.

**Tech Stack:** Rust, reqwest 0.12 (blocking + rustls-tls), serde/serde_json, security-framework 3 (macOS Keychain), chrono 0.4, dirs 5, tray-icon 0.19

---

## File map

| Action  | Path                       | Responsibility                                              |
|---------|----------------------------|-------------------------------------------------------------|
| Modify  | `Cargo.toml`               | add reqwest, serde, serde_json, security-framework, chrono, dirs |
| Create  | `src/provider/mod.rs`      | `LimitWindow`, `UsageState`, `UsageProvider` trait          |
| Create  | `src/http.rs`              | generic `get()`, `HttpError` enum                           |
| Create  | `src/keychain.rs`          | macOS Keychain generic-password reader                      |
| Create  | `src/provider/claude.rs`   | credential loading, expiry check, `ClaudeProvider`          |
| Modify  | `src/main.rs`              | dynamic menu from `ClaudeProvider`, "Aggiorna" item         |

---

## Task 1: Add dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Replace `[dependencies]` section**

```toml
[dependencies]
tray-icon = "0.19"
winit = { version = "0.30", features = ["rwh_06"] }
image = { version = "0.25", default-features = false, features = ["png"] }
reqwest = { version = "0.12", default-features = false, features = ["blocking", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = "0.4"
dirs = "5"

[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.5"
security-framework = "3"
```

- [ ] **Step 2: Verify compile**

```bash
cargo check
```
Expected: compiles, no errors (warnings OK).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add reqwest, serde, security-framework, chrono, dirs"
```

---

## Task 2: Core provider types

**Files:**
- Create: `src/provider/mod.rs`

- [ ] **Step 1: Create file with types and tests**

```rust
pub mod claude;

#[derive(Debug, Clone, PartialEq)]
pub struct LimitWindow {
    pub name: String,
    pub percent_used: Option<f32>,
    pub limit: Option<u32>,
    pub remaining: Option<u32>,
    pub resets_at: Option<String>,
    pub unlimited: bool,
}

#[derive(Debug, Clone)]
pub enum UsageState {
    NotConfigured,
    Stale(String),
    Ok(Vec<LimitWindow>),
    Error(String),
}

pub trait UsageProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn fetch(&self) -> UsageState;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_window_fields() {
        let w = LimitWindow {
            name: "5h".to_string(),
            percent_used: Some(42.0),
            limit: None,
            remaining: None,
            resets_at: None,
            unlimited: false,
        };
        assert_eq!(w.percent_used, Some(42.0));
        assert!(!w.unlimited);
    }

    #[test]
    fn usage_state_error_carries_message() {
        let s = UsageState::Error("timeout".to_string());
        if let UsageState::Error(msg) = s {
            assert_eq!(msg, "timeout");
        } else {
            panic!("wrong variant");
        }
    }
}
```

- [ ] **Step 2: Add `mod provider;` to `src/main.rs`**

Insert at the top of `src/main.rs`, before the existing `use` statements:

```rust
mod http;
mod keychain;
mod provider;
```

- [ ] **Step 3: Run tests**

```bash
cargo test provider::tests
```
Expected: 2 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/provider/mod.rs src/main.rs
git commit -m "feat: add UsageProvider trait and core types"
```

---

## Task 3: Generic HTTP helper

**Files:**
- Create: `src/http.rs`

`get()` adds `Authorization: Bearer <token>` plus any caller-supplied extra headers. Returns the body string on 200, or a typed error for 401/429/other.

- [ ] **Step 1: Create file**

```rust
#[derive(Debug, PartialEq)]
pub enum HttpError {
    Unauthorized,
    RateLimited,
    Other(String),
}

pub fn get(url: &str, token: &str, extra_headers: &[(&str, &str)]) -> Result<String, HttpError> {
    let client = reqwest::blocking::Client::new();
    let mut builder = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token));
    for (name, value) in extra_headers {
        builder = builder.header(*name, *value);
    }
    let resp = builder
        .send()
        .map_err(|e| HttpError::Other(e.to_string()))?;
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
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test http::tests
```
Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/http.rs
git commit -m "feat: generic HTTP GET helper with HttpError"
```

---

## Task 4: Keychain reader

**Files:**
- Create: `src/keychain.rs`

`security-framework` is macOS-only — gate with `#[cfg(target_os = "macos")]`.

- [ ] **Step 1: Create file**

```rust
#[cfg(target_os = "macos")]
pub fn read_generic_password(service: &str, account: &str) -> Option<String> {
    use security_framework::passwords::get_generic_password;
    get_generic_password(service, account)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

#[cfg(not(target_os = "macos"))]
pub fn read_generic_password(_service: &str, _account: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_service_returns_none() {
        let result = read_generic_password("__aiusagebar_test_nonexistent_xyzzy__", "test");
        assert!(result.is_none());
    }
}
```

- [ ] **Step 2: Run test**

```bash
cargo test keychain::tests
```
Expected: 1 test passes. No Keychain dialog (service does not exist → immediate `errSecItemNotFound`).

- [ ] **Step 3: Commit**

```bash
git add src/keychain.rs
git commit -m "feat: macOS Keychain generic-password reader"
```

---

## Task 5: Claude credential loading and expiry check

**Files:**
- Create: `src/provider/claude.rs`

Credential JSON schema (same for Keychain value and `~/.claude/.credentials.json`):
```json
{
  "claudeAiOauth": {
    "accessToken": "sk-ant-oat01-…",
    "expiresAt": 1749123456789
  }
}
```
`expiresAt` is Unix epoch **milliseconds**.

- [ ] **Step 1: Create file with credential types, loader, expiry check, and tests**

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct CredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: OauthEntry,
}

#[derive(Deserialize)]
struct OauthEntry {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresAt")]
    expires_at: u64,
}

pub struct ClaudeCredentials {
    pub access_token: String,
    pub expires_at_ms: u64,
}

pub fn load_credentials() -> Option<ClaudeCredentials> {
    let json = load_credentials_json()?;
    let file: CredentialsFile = serde_json::from_str(&json).ok()?;
    Some(ClaudeCredentials {
        access_token: file.claude_ai_oauth.access_token,
        expires_at_ms: file.claude_ai_oauth.expires_at,
    })
}

fn load_credentials_json() -> Option<String> {
    let account = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    if let Some(json) = crate::keychain::read_generic_password("Claude Code-credentials", &account) {
        return Some(json);
    }
    let path = dirs::home_dir()?.join(".claude").join(".credentials.json");
    std::fs::read_to_string(path).ok()
}

pub fn is_expired(expires_at_ms: u64) -> bool {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    expires_at_ms < now_ms
}

pub fn format_expiry_date(expires_at_ms: u64) -> String {
    use chrono::{DateTime, TimeZone, Utc};
    let secs = (expires_at_ms / 1000) as i64;
    match Utc.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d").to_string(),
        _ => "?".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_credentials_json() {
        let json = r#"{"claudeAiOauth":{"accessToken":"tok123","expiresAt":9999999999000}}"#;
        let file: CredentialsFile = serde_json::from_str(json).unwrap();
        assert_eq!(file.claude_ai_oauth.access_token, "tok123");
        assert_eq!(file.claude_ai_oauth.expires_at, 9999999999000);
    }

    #[test]
    fn parse_missing_expires_at_fails() {
        let json = r#"{"claudeAiOauth":{"accessToken":"tok123"}}"#;
        assert!(serde_json::from_str::<CredentialsFile>(json).is_err());
    }

    #[test]
    fn expired_epoch_plus_1s_is_expired() {
        assert!(is_expired(1_000));
    }

    #[test]
    fn far_future_token_not_expired() {
        assert!(!is_expired(9_999_999_999_000));
    }

    #[test]
    fn format_expiry_date_known_timestamp() {
        // 1749081600000 ms = 2025-06-05 00:00:00 UTC
        assert_eq!(format_expiry_date(1749081600000), "2025-06-05");
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test provider::claude::tests
```
Expected: 5 tests pass (pure logic — no Keychain or network).

- [ ] **Step 3: Commit**

```bash
git add src/provider/claude.rs
git commit -m "feat: Claude credential loader with expiry check"
```

---

## Task 6: Claude API fetch and `UsageProvider` impl

**Files:**
- Modify: `src/provider/claude.rs`

### Background: User-Agent

The Claude usage endpoint requires a UA that matches the Claude Code CLI exactly. Wrong/missing UA → persistent HTTP 429. The format is `claude-code/<version>` where version matches the installed Claude Code binary. We read it once via subprocess and cache it.

- [ ] **Step 1: Append fetch logic to `src/provider/claude.rs`**

Add the following after the existing code in `src/provider/claude.rs`:

```rust
use std::sync::{Mutex, OnceLock};
use crate::http::HttpError;
use crate::provider::{LimitWindow, UsageState, UsageProvider};

static USER_AGENT: OnceLock<String> = OnceLock::new();

fn get_user_agent() -> &'static str {
    USER_AGENT.get_or_init(|| {
        std::process::Command::new("claude")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.split_whitespace().next().map(|v| format!("claude-code/{}", v)))
            .unwrap_or_else(|| "claude-code/2.1.153".to_string())
    })
}

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";

#[derive(Deserialize)]
struct UsageResponse {
    five_hour: WindowData,
    seven_day: WindowData,
}

#[derive(Deserialize)]
struct WindowData {
    used_percentage: f32,
    resets_at: String,
}

fn parse_response(body: &str) -> Result<Vec<LimitWindow>, String> {
    let resp: UsageResponse = serde_json::from_str(body).map_err(|e| e.to_string())?;
    Ok(vec![
        LimitWindow {
            name: "5h session".to_string(),
            percent_used: Some(resp.five_hour.used_percentage),
            limit: None,
            remaining: None,
            resets_at: Some(resp.five_hour.resets_at),
            unlimited: false,
        },
        LimitWindow {
            name: "7d weekly".to_string(),
            percent_used: Some(resp.seven_day.used_percentage),
            limit: None,
            remaining: None,
            resets_at: Some(resp.seven_day.resets_at),
            unlimited: false,
        },
    ])
}

pub struct ClaudeProvider {
    last_ok: Mutex<Option<Vec<LimitWindow>>>,
}

impl ClaudeProvider {
    pub fn new() -> Self {
        Self { last_ok: Mutex::new(None) }
    }
}

impl UsageProvider for ClaudeProvider {
    fn name(&self) -> &'static str { "Anthropic" }

    fn fetch(&self) -> UsageState {
        let creds = match load_credentials() {
            None => return UsageState::NotConfigured,
            Some(c) => c,
        };
        if is_expired(creds.expires_at_ms) {
            let date = format_expiry_date(creds.expires_at_ms);
            return UsageState::Stale(format!("Scaduto dal {} — esegui: claude login", date));
        }
        let ua = get_user_agent();
        match crate::http::get(USAGE_URL, &creds.access_token, &[("User-Agent", ua)]) {
            Ok(body) => match parse_response(&body) {
                Ok(windows) => {
                    *self.last_ok.lock().unwrap() = Some(windows.clone());
                    UsageState::Ok(windows)
                }
                Err(e) => UsageState::Error(format!("Parse error: {}", e)),
            },
            Err(HttpError::Unauthorized) => {
                UsageState::Stale("Token rifiutato — esegui: claude login".to_string())
            }
            Err(HttpError::RateLimited) => {
                self.last_ok
                    .lock()
                    .unwrap()
                    .clone()
                    .map(UsageState::Ok)
                    .unwrap_or_else(|| UsageState::Error("Rate limited (no cache)".to_string()))
            }
            Err(HttpError::Other(e)) => UsageState::Error(e),
        }
    }
}
```

- [ ] **Step 2: Extend the `tests` module in the same file**

Add these test functions inside the existing `mod tests { … }` block:

```rust
    #[test]
    fn parse_valid_response() {
        let body = r#"{
            "five_hour": {"used_percentage": 39.0, "resets_at": "2026-06-06T14:00:00Z"},
            "seven_day":  {"used_percentage": 15.0, "resets_at": "2026-06-10T08:00:00Z"}
        }"#;
        let windows = super::parse_response(body).unwrap();
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].name, "5h session");
        assert_eq!(windows[0].percent_used, Some(39.0));
        assert_eq!(windows[1].name, "7d weekly");
        assert_eq!(windows[1].percent_used, Some(15.0));
    }

    #[test]
    fn parse_missing_field_is_error() {
        assert!(super::parse_response("{}").is_err());
    }
```

- [ ] **Step 3: Run all tests**

```bash
cargo test provider::claude::tests
```
Expected: 7 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/provider/claude.rs
git commit -m "feat: ClaudeProvider fetch with 401/429 handling"
```

---

## Task 7: Wire Claude data into the menu

**Files:**
- Modify: `src/main.rs`

Replace the static "Mostra Matteo" / "Esci" menu with a dynamic menu built from `ClaudeProvider::fetch()`. On startup (`resumed`) fetch once. "Aggiorna" menu item triggers a manual re-fetch. Left-click on tray icon also refreshes.

- [ ] **Step 1: Replace `src/main.rs` entirely**

```rust
mod http;
mod keychain;
mod provider;

use provider::claude::ClaudeProvider;
use provider::{UsageProvider, UsageState};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder, TrayIconEvent,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

struct App {
    tray: tray_icon::TrayIcon,
    id_quit: tray_icon::menu::MenuId,
    id_refresh: tray_icon::menu::MenuId,
    claude: ClaudeProvider,
}

impl App {
    fn build_menu(state: &UsageState) -> (Menu, tray_icon::menu::MenuId, tray_icon::menu::MenuId) {
        let menu = Menu::new();
        match state {
            UsageState::NotConfigured => {
                menu.append(&MenuItem::new("Anthropic: non configurato", false, None)).unwrap();
            }
            UsageState::Stale(msg) => {
                menu.append(&MenuItem::new(format!("Anthropic ⚠  {}", msg), false, None))
                    .unwrap();
            }
            UsageState::Error(msg) => {
                menu.append(&MenuItem::new(format!("Anthropic ✕  {}", msg), false, None))
                    .unwrap();
            }
            UsageState::Ok(windows) => {
                menu.append(&MenuItem::new("Anthropic", false, None)).unwrap();
                for w in windows {
                    let pct = w
                        .percent_used
                        .map(|p| format!("{:.1}%", p))
                        .unwrap_or_else(|| "∞".to_string());
                    let reset = w.resets_at.as_deref().unwrap_or("?");
                    menu.append(&MenuItem::new(
                        format!("  {} — {}  resets {}", w.name, pct, reset),
                        false,
                        None,
                    ))
                    .unwrap();
                }
            }
        }
        let item_refresh = MenuItem::new("Aggiorna", true, None);
        let item_quit = MenuItem::new("Esci", true, None);
        menu.append(&item_refresh).unwrap();
        menu.append(&item_quit).unwrap();
        let id_refresh = item_refresh.id().clone();
        let id_quit = item_quit.id().clone();
        (menu, id_refresh, id_quit)
    }

    fn refresh(&mut self) {
        let state = self.claude.fetch();
        let (menu, id_refresh, id_quit) = Self::build_menu(&state);
        self.id_refresh = id_refresh;
        self.id_quit = id_quit;
        self.tray.set_menu(Some(Box::new(menu)));
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        self.refresh();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);

        if let Ok(ev) = MenuEvent::receiver().try_recv() {
            if ev.id == self.id_quit {
                event_loop.exit();
            } else if ev.id == self.id_refresh {
                self.refresh();
            }
        }

        if let Ok(tray_event) = TrayIconEvent::receiver().try_recv() {
            if let tray_icon::TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                ..
            } = tray_event
            {
                self.refresh();
            }
        }
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    set_accessory_policy();

    let event_loop = EventLoop::new().expect("Impossibile creare event loop");
    let icon = load_icon();
    let claude = ClaudeProvider::new();
    let (initial_menu, id_refresh, id_quit) = App::build_menu(&UsageState::NotConfigured);

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(initial_menu))
        .with_tooltip("AIUsageBar")
        .with_icon(icon)
        .build()
        .expect("Impossibile creare la tray icon");

    let mut app = App { tray, id_quit, id_refresh, claude };
    event_loop.run_app(&mut app).expect("Errore nell'event loop");
}

#[cfg(target_os = "macos")]
fn set_accessory_policy() {
    use objc2::runtime::AnyClass;
    unsafe {
        let cls = AnyClass::get("NSApplication").unwrap();
        let app: *mut objc2::runtime::AnyObject = objc2::msg_send![cls, sharedApplication];
        let _: bool = objc2::msg_send![app, setActivationPolicy: 1_i64];
    }
}

fn load_icon() -> tray_icon::Icon {
    let icon_path = std::path::Path::new("icons/app_icon.png");
    let (rgba, width, height) = if icon_path.exists() {
        let img = image::open(icon_path)
            .expect("Impossibile aprire icons/app_icon.png")
            .into_rgba8();
        let (w, h) = img.dimensions();
        (img.into_raw(), w, h)
    } else {
        eprintln!("icons/app_icon.png not found, using placeholder icon.");
        let size = 32u32;
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        for _ in 0..(size * size) {
            pixels.extend_from_slice(&[0xCC, 0x00, 0x00, 0xFF]);
        }
        (pixels, size, size)
    };
    tray_icon::Icon::from_rgba(rgba, width, height).expect("Impossibile creare l'icona")
}
```

- [ ] **Step 2: Build**

```bash
cargo build
```
Expected: compiles cleanly. The first Keychain read will trigger a macOS permission dialog — click "Always Allow" once.

- [ ] **Step 3: Run and verify**

```bash
cargo run
```

Open the menu bar icon. Expected results:

| Scenario | Menu shows |
|---|---|
| Claude credentials found, token valid | `Anthropic` header + `  5h session — 39.0%  resets 2026-06-06T14:00:00Z` + `  7d weekly — 15.0%  resets …` |
| Token expired | `Anthropic ⚠  Scaduto dal 2025-XX-XX — esegui: claude login` |
| No credentials anywhere | `Anthropic: non configurato` |
| Network error | `Anthropic ✕  <error message>` |

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: show Claude usage windows in tray menu"
```

---

## Self-review checklist

- [x] § 4.2 five_hour + seven_day windows → Task 6 `parse_response`
- [x] § 4.2 pre-flight expiry check → Task 5/6 `is_expired` + Stale message with date
- [x] § 4.2 HTTP 401 → Stale "Token rifiutato" → Task 6 `Err(HttpError::Unauthorized)`
- [x] § 4.4 `NotConfigured` state → Task 6 `None` credentials path
- [x] HTTP GET abstracted, reusable → Task 3 `src/http.rs`
- [x] Keychain abstracted → Task 4 `src/keychain.rs`
- [x] File fallback for credentials → Task 5 `load_credentials_json`
- [x] 429 → return cached state → Task 6 `HttpError::RateLimited` arm
- [x] User-Agent auto-detected from `claude --version` → Task 6 `get_user_agent()`
- [x] No token write, no refresh → read-only paths throughout
