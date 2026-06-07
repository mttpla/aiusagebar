# Copilot Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `CopilotProvider` that discovers all logged-in Copilot accounts via Keychain enumeration and shows their quota windows in a single "GitHub" menu section.

**Architecture:** Single `CopilotProvider` (no trait change), discovers tokens by enumerating Keychain entries with `service="copilot-cli"` plus env vars. One API call per token, results aggregated into one `Vec<LimitWindow>`. Failed/stale accounts appear as sentinel windows. `main.rs` is refactored from a single hardcoded `ClaudeProvider` to a `Vec<Box<dyn UsageProvider>>`.

**Tech Stack:** Rust, `security-framework 3`, `core-foundation 0.9` (direct dep for Keychain enumeration), `serde_json` (generic `Value` parsing — no hardcoded field names), existing `crate::http::{get, HttpError}`.

---

### Task 1: Verify live API response format

**Files:**
- No code changes — manual investigation step

This is required before writing the parser, because Copilot migrated from `premium_interactions` to AI Credits in June 2026 and the `quota_snapshots` field names may differ from the API.md doc.

- [ ] **Step 1: Get a token for the first account**

```bash
TOKEN=$(security find-generic-password -s "copilot-cli" -a "https://github.com:matteopaoliws" -w 2>/dev/null)
echo "Token loaded: ${#TOKEN} chars"
```

Expected: non-zero length printed.

- [ ] **Step 2: Call the endpoint and inspect the response**

```bash
TOKEN=$(security find-generic-password -s "copilot-cli" -a "https://github.com:matteopaoliws" -w 2>/dev/null)
curl -s -H "Authorization: Bearer $TOKEN" https://api.github.com/copilot_internal/user | python3 -m json.tool | head -60
```

Expected: JSON with `login`, `quota_snapshots`, and at least one snapshot key.

- [ ] **Step 3: Record the actual snapshot key names**

Note the exact keys inside `quota_snapshots`. If they are NOT `premium_interactions`, `chat`, `completions`, note what they are. The parser in Task 3 is generic and will handle any key, but the test fixtures must use the real names.

- [ ] **Step 4: No commit** — this is an investigation step only.

---

### Task 2: Add Keychain enumeration + `core-foundation` dep

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/keychain.rs`

- [ ] **Step 1: Add `core-foundation` to `Cargo.toml`**

Add to the macOS-only dependencies block:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.5"
security-framework = "3"
core-foundation = "0.9"
```

- [ ] **Step 2: Write the failing test**

Add to the `tests` module in `src/keychain.rs`:

```rust
    #[test]
    fn enumerate_nonexistent_service_returns_empty() {
        let result = enumerate_generic_passwords("__aiusagebar_test_nonexistent_xyzzy__");
        assert!(result.is_empty());
    }
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cargo test enumerate_nonexistent 2>&1 | tail -8
```

Expected: FAIL with "cannot find function `enumerate_generic_passwords`".

- [ ] **Step 4: Implement `enumerate_generic_passwords`**

Add to `src/keychain.rs` (below the existing `read_generic_password`):

```rust
#[cfg(target_os = "macos")]
pub fn enumerate_generic_passwords(service: &str) -> Vec<(String, String)> {
    use core_foundation::base::TCFType;
    use core_foundation::string::{CFString, CFStringRef};
    use security_framework::item::{ItemClass, ItemSearchOptions, Limit, SearchResult};
    use security_framework::passwords::get_generic_password;

    let results = ItemSearchOptions::new()
        .class(ItemClass::generic_password())
        .service(service)
        .limit(Limit::All)
        .load_attributes(true)
        .search()
        .unwrap_or_default();

    results
        .into_iter()
        .filter_map(|r| {
            let SearchResult::Dict(dict) = r else { return None };
            let key = CFString::from_static_string("acct");
            let account = dict.find(key.as_concrete_TypeRef()).and_then(|v| {
                if v.type_of() == CFString::type_id() {
                    let s = unsafe {
                        CFString::wrap_under_get_rule(v.as_CFTypeRef() as CFStringRef)
                    };
                    Some(s.to_string())
                } else {
                    None
                }
            })?;
            let password = get_generic_password(service, &account).ok()?;
            String::from_utf8(password).ok().map(|p| (account, p))
        })
        .collect()
}

#[cfg(not(target_os = "macos"))]
pub fn enumerate_generic_passwords(_service: &str) -> Vec<(String, String)> {
    Vec::new()
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test enumerate 2>&1 | tail -8
```

Expected: `enumerate_nonexistent_service_returns_empty ... ok`.

- [ ] **Step 6: Smoke-test live Keychain enumeration**

```bash
cargo test -- --ignored copilot_keychain_smoke 2>&1 | tail -10
```

Add this `#[ignore]` test to verify the real Keychain works (run manually only):

```rust
    #[test]
    #[ignore]
    fn copilot_keychain_smoke() {
        let entries = enumerate_generic_passwords("copilot-cli");
        // Should find the 2 accounts on this machine
        assert!(!entries.is_empty(), "expected copilot-cli entries in Keychain");
        for (account, token) in &entries {
            println!("account={account} token_len={}", token.len());
            assert!(account.starts_with("https://github.com:"));
            assert!(!token.is_empty());
        }
    }
```

Run it with:

```bash
cargo test -- --ignored copilot_keychain_smoke 2>&1
```

Expected: passes, prints the two accounts.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/keychain.rs
git commit -m "feat: add Keychain enumeration for multi-account Copilot discovery"
```

---

### Task 3: Create `src/provider/copilot.rs` with response parser

**Files:**
- Create: `src/provider/copilot.rs`
- Modify: `src/provider/mod.rs` (add `pub mod copilot;`)

This task only covers the pure parsing logic. No HTTP calls, no Keychain calls — everything is testable with fake JSON strings.

- [ ] **Step 1: Register the module**

Add to `src/provider/mod.rs`:

```rust
pub mod claude;
pub mod copilot;
```

- [ ] **Step 2: Create `src/provider/copilot.rs` with the parser and its failing tests**

```rust
use serde_json::Value;
use crate::http::HttpError;
use crate::provider::{LimitWindow, UsageState};

fn parse_copilot_response(body: &str) -> Result<Vec<LimitWindow>, String> {
    todo!()
}

pub fn do_copilot_fetch(
    tokens: Vec<String>,
    http: &dyn Fn(&str) -> Result<String, HttpError>,
) -> UsageState {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::HttpError;

    // ── parse_copilot_response ────────────────────────────────────────────

    #[test]
    fn parse_single_limited_snapshot() {
        // Replace "ai_credits" with the actual key name you observed in Task 1.
        let body = r#"{
            "login": "mttpla",
            "quota_reset_date_utc": "2026-07-01T00:00:00Z",
            "quota_snapshots": {
                "ai_credits": {
                    "entitlement": 1000,
                    "remaining": 750,
                    "percent_remaining": 75.0,
                    "unlimited": false
                }
            }
        }"#;
        let windows = parse_copilot_response(body).unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].name, "mttpla / ai_credits");
        assert_eq!(windows[0].percent_used, Some(25.0));
        assert_eq!(windows[0].remaining, Some(750));
        assert_eq!(windows[0].limit, Some(1000));
        assert_eq!(windows[0].resets_at.as_deref(), Some("2026-07-01T00:00:00Z"));
        assert!(!windows[0].unlimited);
    }

    #[test]
    fn parse_skips_unlimited_snapshots() {
        let body = r#"{
            "login": "mttpla",
            "quota_reset_date_utc": "2026-07-01T00:00:00Z",
            "quota_snapshots": {
                "chat":        { "unlimited": true },
                "completions": { "unlimited": true }
            }
        }"#;
        let windows = parse_copilot_response(body).unwrap();
        assert_eq!(windows.len(), 0);
    }

    #[test]
    fn parse_multiple_snapshots_mixed() {
        let body = r#"{
            "login": "mttpla",
            "quota_reset_date_utc": "2026-07-01T00:00:00Z",
            "quota_snapshots": {
                "ai_credits": { "entitlement": 1000, "remaining": 500, "percent_remaining": 50.0, "unlimited": false },
                "chat":       { "unlimited": true }
            }
        }"#;
        let windows = parse_copilot_response(body).unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].percent_used, Some(50.0));
    }

    #[test]
    fn parse_missing_quota_snapshots_is_error() {
        let body = r#"{"login": "mttpla"}"#;
        assert!(parse_copilot_response(body).is_err());
    }

    #[test]
    fn parse_missing_percent_remaining_skips_snapshot() {
        // A snapshot without percent_remaining cannot compute percent_used — skip it.
        let body = r#"{
            "login": "mttpla",
            "quota_reset_date_utc": "2026-07-01T00:00:00Z",
            "quota_snapshots": {
                "mystery": { "unlimited": false }
            }
        }"#;
        let windows = parse_copilot_response(body).unwrap();
        assert_eq!(windows.len(), 0);
    }

    // ── do_copilot_fetch ──────────────────────────────────────────────────

    fn valid_body() -> String {
        r#"{"login":"mttpla","quota_reset_date_utc":"2026-07-01T00:00:00Z","quota_snapshots":{"ai_credits":{"entitlement":1000,"remaining":750,"percent_remaining":75.0,"unlimited":false}}}"#.to_string()
    }

    #[test]
    fn fetch_empty_tokens_returns_not_configured() {
        let state = do_copilot_fetch(vec![], &|_| unreachable!());
        assert_eq!(state, UsageState::NotConfigured);
    }

    #[test]
    fn fetch_all_401_returns_stale() {
        let state = do_copilot_fetch(
            vec!["tok".to_string()],
            &|_| Err(HttpError::Unauthorized),
        );
        assert!(matches!(state, UsageState::Stale(_)));
    }

    #[test]
    fn fetch_200_valid_returns_ok_with_windows() {
        let state = do_copilot_fetch(
            vec!["tok".to_string()],
            &|_| Ok(valid_body()),
        );
        assert!(matches!(state, UsageState::Ok(ref w) if !w.is_empty()));
    }

    #[test]
    fn fetch_200_bad_body_returns_error() {
        let state = do_copilot_fetch(
            vec!["tok".to_string()],
            &|_| Ok("garbage".to_string()),
        );
        assert!(matches!(state, UsageState::Error(_)));
    }

    #[test]
    fn fetch_mixed_success_and_401_returns_ok_with_sentinel() {
        let tokens = vec!["good".to_string(), "bad".to_string()];
        let state = do_copilot_fetch(tokens, &|tok| {
            if tok == "good" { Ok(valid_body()) } else { Err(HttpError::Unauthorized) }
        });
        let UsageState::Ok(windows) = state else { panic!("expected Ok") };
        assert!(windows.iter().any(|w| w.percent_used.is_some()), "real window missing");
        assert!(
            windows.iter().any(|w| w.percent_used.is_none() && w.name.contains("scaduto")),
            "sentinel window missing"
        );
    }
}
```

- [ ] **Step 3: Run tests to verify they fail with `todo!()`**

```bash
cargo test -p aiusagebar provider::copilot 2>&1 | tail -15
```

Expected: all copilot tests FAIL with `explicit panic` from `todo!()`.

- [ ] **Step 4: Implement `parse_copilot_response`**

Replace `todo!()` in `parse_copilot_response`:

```rust
fn parse_copilot_response(body: &str) -> Result<Vec<LimitWindow>, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let login = v["login"].as_str().unwrap_or("unknown");
    let reset = v["quota_reset_date_utc"].as_str().map(|s| s.to_string());
    let snapshots = v["quota_snapshots"]
        .as_object()
        .ok_or_else(|| "missing quota_snapshots".to_string())?;

    Ok(snapshots
        .iter()
        .filter_map(|(key, snap)| {
            if snap["unlimited"].as_bool().unwrap_or(false) {
                return None;
            }
            let pct_remaining = snap["percent_remaining"].as_f64()? as f32;
            Some(LimitWindow {
                name: format!("{} / {}", login, key),
                percent_used: Some(100.0 - pct_remaining),
                limit: snap["entitlement"].as_u64().map(|v| v as u32),
                remaining: snap["remaining"].as_u64().map(|v| v as u32),
                resets_at: reset.clone(),
                unlimited: false,
            })
        })
        .collect())
}
```

- [ ] **Step 5: Run parser tests**

```bash
cargo test provider::copilot::tests::parse 2>&1 | tail -15
```

Expected: all `parse_*` tests pass.

- [ ] **Step 6: Implement `do_copilot_fetch`**

Replace `todo!()` in `do_copilot_fetch`:

```rust
pub fn do_copilot_fetch(
    tokens: Vec<String>,
    http: &dyn Fn(&str) -> Result<String, HttpError>,
) -> UsageState {
    if tokens.is_empty() {
        return UsageState::NotConfigured;
    }

    let mut ok_windows: Vec<LimitWindow> = Vec::new();
    let mut stale_count: usize = 0;
    let mut error_msgs: Vec<String> = Vec::new();

    for token in &tokens {
        match http(token) {
            Ok(body) => match parse_copilot_response(&body) {
                Ok(windows) => ok_windows.extend(windows),
                Err(e) => error_msgs.push(format!("parse error: {}", e)),
            },
            Err(HttpError::Unauthorized) => stale_count += 1,
            Err(HttpError::RateLimited) => error_msgs.push("rate limited".to_string()),
            Err(HttpError::Other(e)) => error_msgs.push(e),
        }
    }

    if !ok_windows.is_empty() {
        for _ in 0..stale_count {
            ok_windows.push(LimitWindow {
                name: "GitHub — token scaduto, ri-logga".to_string(),
                percent_used: None,
                limit: None,
                remaining: None,
                resets_at: None,
                unlimited: false,
            });
        }
        for msg in &error_msgs {
            ok_windows.push(LimitWindow {
                name: format!("GitHub — {}", msg),
                percent_used: None,
                limit: None,
                remaining: None,
                resets_at: None,
                unlimited: false,
            });
        }
        UsageState::Ok(ok_windows)
    } else if stale_count > 0 {
        UsageState::Stale(
            "Token Copilot scaduti — esegui: copilot auth login".to_string(),
        )
    } else {
        UsageState::Error(error_msgs.join("; "))
    }
}
```

- [ ] **Step 7: Run all copilot tests**

```bash
cargo test provider::copilot 2>&1 | tail -20
```

Expected: all 9 tests pass.

- [ ] **Step 8: Run full suite to check no regressions**

```bash
cargo test 2>&1 | tail -5
```

Expected: all tests pass (count grows by ~9).

- [ ] **Step 9: Commit**

```bash
git add src/provider/copilot.rs src/provider/mod.rs
git commit -m "feat: add CopilotProvider parser and fetch logic"
```

---

### Task 4: Complete `CopilotProvider` struct + token loading

**Files:**
- Modify: `src/provider/copilot.rs` (add struct, impl, token loader)

- [ ] **Step 1: Add `CopilotProvider` struct and token loader to `src/provider/copilot.rs`**

Add after `do_copilot_fetch` (before the `#[cfg(test)]` block):

```rust
fn load_copilot_tokens() -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut add = |t: String| { if seen.insert(t.clone()) { tokens.push(t); } };

    for var in &["COPILOT_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"] {
        if let Ok(t) = std::env::var(var) { add(t); }
    }

    for (_account, password) in crate::keychain::enumerate_generic_passwords("copilot-cli") {
        add(password);
    }

    tokens
}

pub struct CopilotProvider;

impl CopilotProvider {
    pub fn new() -> Self { Self }
}

impl crate::provider::UsageProvider for CopilotProvider {
    fn name(&self) -> &'static str { "GitHub" }

    fn fetch(&self) -> UsageState {
        do_copilot_fetch(
            load_copilot_tokens(),
            &|token| crate::http::get(
                "https://api.github.com/copilot_internal/user",
                token,
                &[],
            ),
        )
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check 2>&1
```

Expected: no errors.

- [ ] **Step 3: Run full test suite**

```bash
cargo test 2>&1 | tail -5
```

Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add src/provider/copilot.rs
git commit -m "feat: add CopilotProvider struct with Keychain token loading"
```

---

### Task 5: Refactor `main.rs` for multi-provider + wire `CopilotProvider`

**Files:**
- Modify: `src/main.rs`

The current `App` struct hardcodes `claude: ClaudeProvider`. This task generalises it to `providers: Vec<Box<dyn UsageProvider>>` and wires up both providers.

- [ ] **Step 1: Replace `src/main.rs` with the multi-provider version**

```rust
mod http;
mod icon;
mod keychain;
mod launch_at_login;
mod provider;

use icon::{IconKind, Icons};
use provider::claude::ClaudeProvider;
use provider::copilot::CopilotProvider;
use provider::{UsageProvider, UsageState};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder, TrayIconEvent,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

struct MenuBuild {
    menu: Menu,
    refresh: tray_icon::menu::MenuId,
    quit: tray_icon::menu::MenuId,
}

fn append_label(menu: &Menu, text: impl Into<String>) {
    menu.append(&MenuItem::new(text.into(), false, None))
        .expect("menu append failed");
}

struct App {
    tray: tray_icon::TrayIcon,
    icons: Icons,
    id_quit: tray_icon::menu::MenuId,
    id_refresh: tray_icon::menu::MenuId,
    providers: Vec<Box<dyn UsageProvider>>,
}

impl App {
    fn build_menu(states: &[(&str, &UsageState)]) -> MenuBuild {
        let menu = Menu::new();
        for (name, state) in states {
            match state {
                UsageState::NotConfigured => {
                    append_label(&menu, format!("{}: not configured", name));
                }
                UsageState::Stale(msg) => {
                    append_label(&menu, format!("{} ⚠  {}", name, msg));
                }
                UsageState::Error(msg) => {
                    append_label(&menu, format!("{} ✕  {}", name, msg));
                }
                UsageState::Ok(windows) => {
                    append_label(&menu, name.to_string());
                    for w in windows {
                        let pct = w
                            .percent_used
                            .map(|p| format!("{:.1}%", p))
                            .unwrap_or_else(|| "—".to_string());
                        let reset = w.resets_at.as_deref().unwrap_or("?");
                        append_label(
                            &menu,
                            format!("  {} — {}  resets {}", w.name, pct, reset),
                        );
                    }
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

    fn refresh(&mut self) {
        let states: Vec<(&str, UsageState)> = self.providers
            .iter()
            .map(|p| (p.name(), p.fetch()))
            .collect();

        let icon_kind = states.iter().fold(IconKind::Normal, |best, (_, s)| {
            match (best, IconKind::for_state(s)) {
                (IconKind::Alert, _) | (_, IconKind::Alert) => IconKind::Alert,
                (IconKind::Unavailable, _) | (_, IconKind::Unavailable) => {
                    IconKind::Unavailable
                }
                _ => IconKind::Normal,
            }
        });

        let refs: Vec<(&str, &UsageState)> =
            states.iter().map(|(n, s)| (*n, s)).collect();
        let build = Self::build_menu(&refs);
        self.id_refresh = build.refresh;
        self.id_quit = build.quit;
        self.tray.set_menu(Some(Box::new(build.menu)));
        self.tray.set_icon(Some(self.icons.get(icon_kind))).ok();
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

        let _ = TrayIconEvent::receiver().try_recv();
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    set_accessory_policy();

    if let Err(e) = launch_at_login::enable() {
        eprintln!("[launch_at_login] {e}");
    }

    let providers: Vec<Box<dyn UsageProvider>> = vec![
        Box::new(ClaudeProvider::new()),
        Box::new(CopilotProvider::new()),
    ];

    let event_loop = EventLoop::new().expect("failed to create event loop");
    let icons = Icons::load();

    let initial_state = UsageState::NotConfigured;
    let initial_refs: Vec<(&str, &UsageState)> = providers
        .iter()
        .map(|p| (p.name(), &initial_state))
        .collect();
    let build = App::build_menu(&initial_refs);

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(build.menu))
        .with_tooltip("AIUsageBar")
        .with_icon(icons.get(IconKind::Unavailable))
        .build()
        .expect("failed to create tray icon");

    let mut app = App {
        tray,
        icons,
        id_quit: build.quit,
        id_refresh: build.refresh,
        providers,
    };
    event_loop.run_app(&mut app).expect("event loop error");
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
```

- [ ] **Step 2: Compile**

```bash
cargo check 2>&1
```

Expected: no errors.

- [ ] **Step 3: Run full test suite**

```bash
cargo test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 4: Build and run manually**

```bash
make dev
```

Expected: tray icon appears. Clicking it shows both "Anthropic" and "GitHub" sections. GitHub section shows real quota windows for both accounts (or sentinel "token scaduto" lines for expired ones). No crashes.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire CopilotProvider, generalise App to multi-provider"
```
