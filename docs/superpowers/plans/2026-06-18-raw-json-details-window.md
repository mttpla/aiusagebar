# Raw JSON Details Window Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a "Details…" menu item per provider that opens a macOS NSAlert window showing the last raw HTTP response body.

**Architecture:** Capture the raw response body in `http::get` (any status code). Each provider caches it in a `Mutex<Option<String>>`. The UI appends a clickable Details item; click dispatches `details::show(provider_name, raw_json)`.

**Tech Stack:** Rust, objc2 0.6, objc2-app-kit 0.3, tray-icon, ureq (existing).

## Global Constraints

- All string literals in `.rs` files must be English.
- Never add `#[allow(dead_code)]` — delete unused symbols instead.
- Run `cargo clippy -- -D warnings && cargo test` before every commit.
- Never write to Keychain or credential files (read-only constraint).
- `http::get` is changed; `http::get_public` is **not** changed.
- `details::show` is `#[cfg(target_os = "macos")]` only.

---

## File Map

| File | Action |
|---|---|
| `src/http.rs` | Change `get` return to `(Result<String, HttpError>, Option<String>)` |
| `src/provider/mod.rs` | Add `raw_json(&self) -> Option<String>` to trait |
| `src/provider/claude.rs` | Add `last_raw_json` field; update `do_fetch` param + caching |
| `src/provider/copilot.rs` | `CopilotProvider` gains field; `do_copilot_fetch` returns 3-tuple |
| `src/details.rs` | New module: `prepare_content` + macOS `show` |
| `src/ui/claude.rs` | `section_item_count` +1; return type `(Option<MenuId>, MenuId)` |
| `src/ui/copilot.rs` | Same changes as `ui/claude.rs` |
| `src/ui/mod.rs` | `MenuBuild` gains `details_*` fields; `build_menu` updated |
| `src/main.rs` | `App` gains `id_details_*`; wires click handlers |
| `Cargo.toml` | Add `NSScrollView`, `NSTextView` to objc2-app-kit features |

---

### Task 1: `http.rs` + mechanical caller updates

**Files:**
- Modify: `src/http.rs`
- Modify: `src/provider/claude.rs` (fetch_profile + do_fetch signature + test closures)
- Modify: `src/provider/copilot.rs` (do_copilot_fetch signature + test closures + CopilotProvider::fetch_with_http_error)

**Interfaces:**
- Produces: `pub fn get(url: &str, token: &str, extra_headers: &[(&str, &str)]) -> (Result<String, HttpError>, Option<String>)`
  - `Ok(body)` on 200; `Err(...)` on non-200 or network error
  - `Option<String>` = raw body whenever server responded; `None` on network/IO error
- Produces: `do_fetch` closure param type: `&dyn Fn(&str) -> (Result<String, HttpError>, Option<String>)`
- Produces: `do_copilot_fetch` closure param type: same

**NOTE:** This task touches 3 files to keep the build green throughout. `do_fetch` and `do_copilot_fetch` receive the raw body from the closure but discard it (`_raw`) — that wiring happens in Tasks 3 and 4.

- [ ] **Step 1: Write failing test for new `get` signature**

In `src/http.rs` tests:

```rust
#[test]
fn get_returns_tuple() {
    let _: fn(&str, &str, &[(&str, &str)]) -> (Result<String, HttpError>, Option<String>) = super::get;
}
```

Run: `cargo test -p aiusagebar http::tests::get_returns_tuple`
Expected: FAIL (wrong type / function not found with that signature)

- [ ] **Step 2: Rewrite `get` in `src/http.rs`**

Replace the entire `get` function (lines 25-43) with:

```rust
pub fn get(url: &str, token: &str, extra_headers: &[(&str, &str)]) -> (Result<String, HttpError>, Option<String>) {
    let mut req = agent()
        .get(url)
        .header("Authorization", &format!("Bearer {}", token));
    for (name, value) in extra_headers {
        req = req.header(*name, *value);
    }
    let resp = match req.call() {
        Ok(r) => r,
        Err(e) => return (Err(HttpError::Other(e.to_string())), None),
    };
    let status = resp.status().as_u16();
    let raw = resp.into_body().read_to_string().ok();
    let result = match status {
        200 => raw.clone().map(Ok).unwrap_or_else(|| Err(HttpError::Other("body read error".into()))),
        401 => Err(HttpError::Unauthorized),
        429 => Err(HttpError::RateLimited),
        c @ 500..=599 => Err(HttpError::ServerError(c)),
        code => Err(HttpError::Other(format!("HTTP {}", code))),
    };
    (result, raw)
}
```

- [ ] **Step 3: Run http.rs tests**

Run: `cargo test -p aiusagebar http::tests`
Expected: all pass (including new `get_returns_tuple`)

- [ ] **Step 4: Update `fetch_profile` in `src/provider/claude.rs`**

Replace:
```rust
fn fetch_profile(token: &str, ua: &str) -> Option<ProfileData> {
    crate::http::get(PROFILE_URL, token, &[("User-Agent", ua)])
        .ok()
        .and_then(|body| parse_profile_response(&body).ok())
}
```

With:
```rust
fn fetch_profile(token: &str, ua: &str) -> Option<ProfileData> {
    let (result, _) = crate::http::get(PROFILE_URL, token, &[("User-Agent", ua)]);
    result.ok().and_then(|body| parse_profile_response(&body).ok())
}
```

- [ ] **Step 5: Update `do_fetch` signature and body in `src/provider/claude.rs`**

Change the `http` parameter type and destructure the result (raw body ignored with `_raw`):

```rust
fn do_fetch(
    creds: CredLoad,
    http: &dyn Fn(&str) -> (Result<String, HttpError>, Option<String>),
    last_ok: &Mutex<Option<Vec<LimitWindow>>>,
    profile_string: Option<String>,
) -> (UsageState, Option<HttpError>) {
    let creds = match creds {
        CredLoad::NotConfigured => return (UsageState::NotConfigured, None),
        CredLoad::Malformed(e) => return (UsageState::Error(format!("Malformed credentials: {}", e)), None),
        CredLoad::Ok(c) => c,
    };
    if is_expired(creds.expires_at_ms) {
        let date = format_expiry_date(creds.expires_at_ms);
        return (UsageState::Stale(format!("Expired on {} — run: claude login", date)), None);
    }
    let (result, _raw) = http(&creds.access_token);
    match result {
        Ok(body) => match parse_response(&body) {
            Ok(windows) => {
                let windows = windows.to_vec();
                *last_ok.lock().unwrap() = Some(windows.clone());
                (UsageState::Ok(windows, profile_string), None)
            }
            Err(e) => (UsageState::Error(format!("Parse error: {}", e)), None),
        },
        Err(HttpError::Unauthorized) => {
            (UsageState::Stale("Token rejected — run: claude login".to_string()), Some(HttpError::Unauthorized))
        }
        Err(HttpError::RateLimited) => {
            let state = last_ok
                .lock()
                .unwrap()
                .clone()
                .map(|w| UsageState::Ok(w, profile_string))
                .unwrap_or_else(|| UsageState::Error("Rate limited (no cache)".to_string()));
            (state, Some(HttpError::RateLimited))
        }
        Err(HttpError::ServerError(c)) => (UsageState::Error(format!("Server error {c}")), Some(HttpError::ServerError(c))),
        Err(HttpError::Other(e)) => (UsageState::Error(e), None),
    }
}
```

- [ ] **Step 6: Update `ClaudeProvider::fetch_with_http_error` in `src/provider/claude.rs`**

The closure now matches the new type automatically — `crate::http::get` returns the tuple:

```rust
let (state, http_err) = do_fetch(
    creds,
    &|token| crate::http::get(USAGE_URL, token, &[("User-Agent", ua)]),
    &self.last_ok,
    profile_string,
);
```

No change needed to this call site since the closure return type is inferred from `http::get`.

- [ ] **Step 7: Update test closures in `src/provider/claude.rs`**

Every test closure that returns `Ok(...)` or `Err(...)` must return a tuple. Closures returning `unreachable!()` are unchanged (it diverges to any type). Find and update:

```rust
// do_fetch_401_returns_stale
&|_| (Err(HttpError::Unauthorized), None)

// do_fetch_429_no_cache_returns_error
&|_| (Err(HttpError::RateLimited), None)

// do_fetch_429_with_cache_returns_cached_ok
&|_| (Err(HttpError::RateLimited), None)

// do_fetch_200_bad_body_returns_error
&|_| (Ok("garbage".to_string()), Some("garbage".to_string()))

// do_fetch_200_valid_returns_ok_and_populates_cache
&|_| (Ok(valid_body().to_string()), Some(valid_body().to_string()))

// do_fetch_passes_profile_string_into_ok
&|_| (Ok(valid_body().to_string()), Some(valid_body().to_string()))

// do_fetch_none_profile_propagates_to_ok
&|_| (Ok(valid_body().to_string()), Some(valid_body().to_string()))

// do_fetch_429_with_cache_includes_profile_string
&|_| (Err(HttpError::RateLimited), None)
```

- [ ] **Step 8: Update `do_copilot_fetch` signature in `src/provider/copilot.rs`**

Change the `http` parameter type and destructure per-iteration (raw ignored with `_raw`):

```rust
pub fn do_copilot_fetch(
    tokens: Vec<(String, String)>,
    http: &dyn Fn(&str) -> (Result<String, HttpError>, Option<String>),
) -> (UsageState, Option<HttpError>) {
    if tokens.is_empty() {
        return (UsageState::NotConfigured, None);
    }

    let mut ok_windows: Vec<LimitWindow> = Vec::new();
    let mut stale_accounts: Vec<String> = Vec::new();
    let mut error_msgs: Vec<String> = Vec::new();
    let mut backoff_err: Option<HttpError> = None;

    for (account, token) in &tokens {
        let (result, _raw) = http(token);
        match result {
            Ok(body) => match parse_copilot_response(&body) {
                Ok(windows) => ok_windows.extend(windows),
                Err(e) => error_msgs.push(format!("@{} — {}", account, e)),
            },
            Err(HttpError::Unauthorized) => stale_accounts.push(account.clone()),
            Err(HttpError::RateLimited) => {
                error_msgs.push(format!("@{} — rate limited", account));
                backoff_err = Some(HttpError::RateLimited);
            }
            Err(HttpError::ServerError(c)) => {
                error_msgs.push(format!("@{} — server error {c}", account));
                if backoff_err.is_none() {
                    backoff_err = Some(HttpError::ServerError(c));
                }
            }
            Err(HttpError::Other(e)) => error_msgs.push(format!("@{} — {}", account, e)),
        }
    }

    if !ok_windows.is_empty() {
        for account in stale_accounts {
            ok_windows.push(LimitWindow {
                name: format!("@{} — token expired, re-login", account),
                percent_used: None,
                limit: None,
                remaining: None,
                resets_at: None,
                unlimited: false,
            });
        }
        for msg in error_msgs {
            ok_windows.push(LimitWindow {
                name: msg,
                percent_used: None,
                limit: None,
                remaining: None,
                resets_at: None,
                unlimited: false,
            });
        }
        return (UsageState::Ok(ok_windows, None), backoff_err);
    }

    if !stale_accounts.is_empty() {
        return (UsageState::Stale(
            "Copilot tokens expired — run: copilot auth login".to_string(),
        ), None);
    }

    (UsageState::Error(error_msgs.join("; ")), backoff_err)
}
```

- [ ] **Step 9: Update `CopilotProvider::fetch_with_http_error` in `src/provider/copilot.rs`**

```rust
fn fetch_with_http_error(&self) -> (UsageState, Option<crate::http::HttpError>) {
    do_copilot_fetch(
        load_copilot_tokens(),
        &|token| {
            crate::http::get(
                "https://api.github.com/copilot_internal/user",
                token,
                &[("User-Agent", "aiusagebar/0.1")],
            )
        },
    )
}
```

- [ ] **Step 10: Update test closures in `src/provider/copilot.rs`**

```rust
// fetch_empty_tokens_returns_not_configured: &|_| unreachable!() — no change

// fetch_all_401_returns_stale
&|_| (Err(HttpError::Unauthorized), None)

// fetch_200_valid_returns_ok_with_windows
&|_| (Ok(valid_body()), Some(valid_body()))

// fetch_mixed_success_and_401_returns_ok_with_sentinel
&|tok| {
    if tok == "good" {
        (Ok(valid_body()), Some(valid_body()))
    } else {
        (Err(HttpError::Unauthorized), None)
    }
}

// fetch_other_error_returns_error
&|_| (Err(HttpError::Other("connection refused".to_string())), None)

// fetch_error_sentinel_contains_account_name
&|tok| {
    if tok == "good" {
        (Ok(valid_body()), Some(valid_body()))
    } else {
        (Err(HttpError::Other("timeout".to_string())), None)
    }
}

// fetch_200_bad_body_returns_error
&|_| (Ok("not json".to_string()), Some("not json".to_string()))
```

- [ ] **Step 11: Run all tests**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: all pass, no warnings

- [ ] **Step 12: Commit**

```bash
git add src/http.rs src/provider/claude.rs src/provider/copilot.rs
git commit -m "feat: http::get returns raw body alongside result"
```

---

### Task 2: `provider/mod.rs` — `raw_json()` trait + stubs

**Files:**
- Modify: `src/provider/mod.rs`
- Modify: `src/provider/claude.rs` (add stub impl)
- Modify: `src/provider/copilot.rs` (add stub impl)

**Interfaces:**
- Consumes: existing `UsageProvider` trait
- Produces: `fn raw_json(&self) -> Option<String>` on trait (stub returns `None`)

- [ ] **Step 1: Add `raw_json` to trait in `src/provider/mod.rs`**

In the `UsageProvider` trait block, add after `fetch_with_http_error`:

```rust
fn raw_json(&self) -> Option<String>;
```

- [ ] **Step 2: Add stub to `ClaudeProvider` in `src/provider/claude.rs`**

In the `impl UsageProvider for ClaudeProvider` block, add:

```rust
fn raw_json(&self) -> Option<String> {
    None
}
```

- [ ] **Step 3: Add stub to `CopilotProvider` in `src/provider/copilot.rs`**

In the `impl crate::provider::UsageProvider for CopilotProvider` block, add:

```rust
fn raw_json(&self) -> Option<String> {
    None
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add src/provider/mod.rs src/provider/claude.rs src/provider/copilot.rs
git commit -m "feat: add raw_json() to UsageProvider trait with stub impls"
```

---

### Task 3: `provider/claude.rs` — full `raw_json` implementation

**Files:**
- Modify: `src/provider/claude.rs`

**Interfaces:**
- Consumes: `http::get` returning `(Result<String, HttpError>, Option<String>)` (Task 1)
- Produces: `ClaudeProvider::raw_json()` returns `Some(body)` after any fetch where server responded

- [ ] **Step 1: Write failing tests**

Add in `src/provider/claude.rs` test module:

```rust
#[test]
fn raw_json_none_before_any_fetch() {
    let p = super::ClaudeProvider::new();
    assert!(p.raw_json().is_none());
}

#[test]
fn do_fetch_stores_raw_body_on_200() {
    let raw_cache: Mutex<Option<String>> = Mutex::new(None);
    let _ = super::do_fetch(
        ok_creds(),
        &|_| (Ok(valid_body().to_string()), Some(valid_body().to_string())),
        &empty_cache(),
        &raw_cache,
        None,
    );
    assert_eq!(raw_cache.lock().unwrap().as_deref(), Some(valid_body()));
}

#[test]
fn do_fetch_stores_raw_body_on_401() {
    let raw_cache: Mutex<Option<String>> = Mutex::new(None);
    let _ = super::do_fetch(
        ok_creds(),
        &|_| (Err(HttpError::Unauthorized), Some(r#"{"error":"unauthorized"}"#.to_string())),
        &empty_cache(),
        &raw_cache,
        None,
    );
    assert_eq!(
        raw_cache.lock().unwrap().as_deref(),
        Some(r#"{"error":"unauthorized"}"#)
    );
}

#[test]
fn do_fetch_does_not_store_raw_on_network_error() {
    let raw_cache: Mutex<Option<String>> = Mutex::new(None);
    let _ = super::do_fetch(
        ok_creds(),
        &|_| (Err(HttpError::Other("connection refused".into())), None),
        &empty_cache(),
        &raw_cache,
        None,
    );
    assert!(raw_cache.lock().unwrap().is_none());
}
```

Run: `cargo test -p aiusagebar provider::claude::tests::raw_json_none_before_any_fetch`
Expected: FAIL (method doesn't compile — do_fetch takes wrong arity)

- [ ] **Step 2: Add `last_raw_json` field to `ClaudeProvider`**

Change:
```rust
pub struct ClaudeProvider {
    last_ok: Mutex<Option<Vec<LimitWindow>>>,
    profile: Mutex<Option<ProfileData>>,
}

impl Default for ClaudeProvider {
    fn default() -> Self {
        Self {
            last_ok: Mutex::new(None),
            profile: Mutex::new(None),
        }
    }
}
```

To:
```rust
pub struct ClaudeProvider {
    last_ok: Mutex<Option<Vec<LimitWindow>>>,
    profile: Mutex<Option<ProfileData>>,
    last_raw_json: Mutex<Option<String>>,
}

impl Default for ClaudeProvider {
    fn default() -> Self {
        Self {
            last_ok: Mutex::new(None),
            profile: Mutex::new(None),
            last_raw_json: Mutex::new(None),
        }
    }
}
```

- [ ] **Step 3: Update `do_fetch` to accept and store `last_raw_json`**

Add `last_raw_json` parameter and write to it when raw body is present:

```rust
fn do_fetch(
    creds: CredLoad,
    http: &dyn Fn(&str) -> (Result<String, HttpError>, Option<String>),
    last_ok: &Mutex<Option<Vec<LimitWindow>>>,
    last_raw_json: &Mutex<Option<String>>,
    profile_string: Option<String>,
) -> (UsageState, Option<HttpError>) {
    let creds = match creds {
        CredLoad::NotConfigured => return (UsageState::NotConfigured, None),
        CredLoad::Malformed(e) => return (UsageState::Error(format!("Malformed credentials: {}", e)), None),
        CredLoad::Ok(c) => c,
    };
    if is_expired(creds.expires_at_ms) {
        let date = format_expiry_date(creds.expires_at_ms);
        return (UsageState::Stale(format!("Expired on {} — run: claude login", date)), None);
    }
    let (result, raw) = http(&creds.access_token);
    if let Some(body) = raw {
        *last_raw_json.lock().unwrap() = Some(body);
    }
    match result {
        Ok(body) => match parse_response(&body) {
            Ok(windows) => {
                let windows = windows.to_vec();
                *last_ok.lock().unwrap() = Some(windows.clone());
                (UsageState::Ok(windows, profile_string), None)
            }
            Err(e) => (UsageState::Error(format!("Parse error: {}", e)), None),
        },
        Err(HttpError::Unauthorized) => {
            (UsageState::Stale("Token rejected — run: claude login".to_string()), Some(HttpError::Unauthorized))
        }
        Err(HttpError::RateLimited) => {
            let state = last_ok
                .lock()
                .unwrap()
                .clone()
                .map(|w| UsageState::Ok(w, profile_string))
                .unwrap_or_else(|| UsageState::Error("Rate limited (no cache)".to_string()));
            (state, Some(HttpError::RateLimited))
        }
        Err(HttpError::ServerError(c)) => (UsageState::Error(format!("Server error {c}")), Some(HttpError::ServerError(c))),
        Err(HttpError::Other(e)) => (UsageState::Error(e), None),
    }
}
```

- [ ] **Step 4: Update `ClaudeProvider::fetch_with_http_error` to pass `last_raw_json`**

```rust
let (state, http_err) = do_fetch(
    creds,
    &|token| crate::http::get(USAGE_URL, token, &[("User-Agent", ua)]),
    &self.last_ok,
    &self.last_raw_json,
    profile_string,
);
```

- [ ] **Step 5: Replace the stub `raw_json()` with real implementation**

```rust
fn raw_json(&self) -> Option<String> {
    self.last_raw_json.lock().unwrap().clone()
}
```

- [ ] **Step 6: Update all existing `do_fetch` test calls to add `&Mutex::new(None)` as 4th argument**

Every call to `super::do_fetch(creds, http_fn, cache, profile)` becomes `super::do_fetch(creds, http_fn, cache, &Mutex::new(None), profile)`.

Affected tests:
- `do_fetch_not_configured`
- `do_fetch_malformed_creds`
- `do_fetch_expired_token_returns_stale`
- `do_fetch_401_returns_stale`
- `do_fetch_429_no_cache_returns_error`
- `do_fetch_429_with_cache_returns_cached_ok` (uses its own `cache`, pass `&Mutex::new(None)` for raw)
- `do_fetch_200_bad_body_returns_error`
- `do_fetch_200_valid_returns_ok_and_populates_cache`
- `do_fetch_passes_profile_string_into_ok`
- `do_fetch_none_profile_propagates_to_ok`
- `do_fetch_429_with_cache_includes_profile_string`

Example (do_fetch_401_returns_stale):
```rust
#[test]
fn do_fetch_401_returns_stale() {
    let (state, _) = super::do_fetch(
        ok_creds(),
        &|_| (Err(HttpError::Unauthorized), None),
        &empty_cache(),
        &Mutex::new(None),
        None,
    );
    assert!(matches!(state, UsageState::Stale(ref s) if s.contains("Token rejected")));
}
```

- [ ] **Step 7: Run all tests**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: all pass including the 4 new raw_json tests

- [ ] **Step 8: Commit**

```bash
git add src/provider/claude.rs
git commit -m "feat(claude): cache last raw HTTP body, implement raw_json()"
```

---

### Task 4: `provider/copilot.rs` — full `raw_json` implementation

**Files:**
- Modify: `src/provider/copilot.rs`

**Interfaces:**
- Consumes: `http::get` returning tuple (Task 1)
- Produces: `CopilotProvider::raw_json()` returns concatenated per-account bodies with `--- @account ---` separators

- [ ] **Step 1: Write failing tests**

Add in `src/provider/copilot.rs` test module:

```rust
#[test]
fn raw_json_none_before_any_fetch() {
    let p = super::CopilotProvider::new();
    assert!(p.raw_json().is_none());
}

#[test]
fn fetch_single_account_raw_json_is_body() {
    let body = valid_body();
    let (_, _, raw) = do_copilot_fetch(
        vec![tok("alice", "tok_a")],
        &|_| (Ok(body.clone()), Some(body.clone())),
    );
    let raw = raw.unwrap();
    assert!(raw.contains("--- @alice ---"), "missing account header");
    assert!(raw.contains(&body), "missing body");
}

#[test]
fn fetch_multi_account_raw_json_has_both_sections() {
    let body1 = r#"{"login":"alice"}"#.to_string();
    let body2 = r#"{"login":"bob"}"#.to_string();
    let (_, _, raw) = do_copilot_fetch(
        vec![tok("alice", "tok_a"), tok("bob", "tok_b")],
        &|tok| {
            if tok == "tok_a" {
                (Ok(body1.clone()), Some(body1.clone()))
            } else {
                (Ok(body2.clone()), Some(body2.clone()))
            }
        },
    );
    let raw = raw.unwrap();
    assert!(raw.contains("--- @alice ---"));
    assert!(raw.contains("--- @bob ---"));
    assert!(raw.contains(&body1));
    assert!(raw.contains(&body2));
}

#[test]
fn fetch_empty_tokens_raw_json_is_none() {
    let (_, _, raw) = do_copilot_fetch(vec![], &|_| unreachable!());
    assert!(raw.is_none());
}
```

Run: `cargo test -p aiusagebar provider::copilot::tests::raw_json_none_before_any_fetch`
Expected: FAIL (do_copilot_fetch doesn't return 3-tuple yet)

- [ ] **Step 2: Change `CopilotProvider` to a struct with `last_raw_json` field**

```rust
pub struct CopilotProvider {
    last_raw_json: Mutex<Option<String>>,
}

impl CopilotProvider {
    pub fn new() -> Self {
        Self {
            last_raw_json: Mutex::new(None),
        }
    }
}
```

- [ ] **Step 3: Update `do_copilot_fetch` to return 3-tuple and build raw buffer**

```rust
pub fn do_copilot_fetch(
    tokens: Vec<(String, String)>,
    http: &dyn Fn(&str) -> (Result<String, HttpError>, Option<String>),
) -> (UsageState, Option<HttpError>, Option<String>) {
    if tokens.is_empty() {
        return (UsageState::NotConfigured, None, None);
    }

    let mut ok_windows: Vec<LimitWindow> = Vec::new();
    let mut stale_accounts: Vec<String> = Vec::new();
    let mut error_msgs: Vec<String> = Vec::new();
    let mut backoff_err: Option<HttpError> = None;
    let mut raw_buf = String::new();

    for (account, token) in &tokens {
        let (result, raw) = http(token);
        if let Some(body) = raw {
            if !raw_buf.is_empty() {
                raw_buf.push('\n');
            }
            raw_buf.push_str(&format!("--- @{} ---\n{}", account, body));
        }
        match result {
            Ok(body) => match parse_copilot_response(&body) {
                Ok(windows) => ok_windows.extend(windows),
                Err(e) => error_msgs.push(format!("@{} — {}", account, e)),
            },
            Err(HttpError::Unauthorized) => stale_accounts.push(account.clone()),
            Err(HttpError::RateLimited) => {
                error_msgs.push(format!("@{} — rate limited", account));
                backoff_err = Some(HttpError::RateLimited);
            }
            Err(HttpError::ServerError(c)) => {
                error_msgs.push(format!("@{} — server error {c}", account));
                if backoff_err.is_none() {
                    backoff_err = Some(HttpError::ServerError(c));
                }
            }
            Err(HttpError::Other(e)) => error_msgs.push(format!("@{} — {}", account, e)),
        }
    }

    let raw_json = if raw_buf.is_empty() { None } else { Some(raw_buf) };

    if !ok_windows.is_empty() {
        for account in stale_accounts {
            ok_windows.push(LimitWindow {
                name: format!("@{} — token expired, re-login", account),
                percent_used: None,
                limit: None,
                remaining: None,
                resets_at: None,
                unlimited: false,
            });
        }
        for msg in error_msgs {
            ok_windows.push(LimitWindow {
                name: msg,
                percent_used: None,
                limit: None,
                remaining: None,
                resets_at: None,
                unlimited: false,
            });
        }
        return (UsageState::Ok(ok_windows, None), backoff_err, raw_json);
    }

    if !stale_accounts.is_empty() {
        return (UsageState::Stale(
            "Copilot tokens expired — run: copilot auth login".to_string(),
        ), None, raw_json);
    }

    (UsageState::Error(error_msgs.join("; ")), backoff_err, raw_json)
}
```

- [ ] **Step 4: Update `CopilotProvider::fetch_with_http_error` to store raw**

```rust
fn fetch_with_http_error(&self) -> (UsageState, Option<crate::http::HttpError>) {
    let (state, err, raw) = do_copilot_fetch(
        load_copilot_tokens(),
        &|token| {
            crate::http::get(
                "https://api.github.com/copilot_internal/user",
                token,
                &[("User-Agent", "aiusagebar/0.1")],
            )
        },
    );
    *self.last_raw_json.lock().unwrap() = raw;
    (state, err)
}
```

- [ ] **Step 5: Replace stub `raw_json()` with real implementation**

```rust
fn raw_json(&self) -> Option<String> {
    self.last_raw_json.lock().unwrap().clone()
}
```

- [ ] **Step 6: Update all existing `do_copilot_fetch` test calls to destructure 3-tuple**

Replace `let (state, _) = do_copilot_fetch(...)` with `let (state, _, _) = do_copilot_fetch(...)` throughout. Affected tests:
- `fetch_empty_tokens_returns_not_configured`
- `fetch_all_401_returns_stale`
- `fetch_200_valid_returns_ok_with_windows`
- `fetch_mixed_success_and_401_returns_ok_with_sentinel`
- `fetch_other_error_returns_error`
- `fetch_error_sentinel_contains_account_name`
- `fetch_200_bad_body_returns_error`

- [ ] **Step 7: Run all tests**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: all pass including the 4 new copilot raw_json tests

- [ ] **Step 8: Commit**

```bash
git add src/provider/copilot.rs
git commit -m "feat(copilot): cache raw HTTP body per account, implement raw_json()"
```

---

### Task 5: `src/details.rs` + `Cargo.toml` — new details module

**Files:**
- Create: `src/details.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Produces: `pub fn prepare_content(raw_json: Option<&str>) -> String` (pure, all platforms)
- Produces: `pub fn show(provider_name: &str, raw_json: Option<&str>)` (macOS only, `#[cfg(target_os = "macos")]`)

- [ ] **Step 1: Add objc2-app-kit features to `Cargo.toml`**

In `Cargo.toml`, find the `objc2-app-kit` dependency and add `NSScrollView` and `NSTextView` to its features list:

```toml
objc2-app-kit = { version = "0.3", features = [
    "NSAlert", "NSTextField", "NSControl", "NSView", "NSText",
    "NSColor", "NSFont", "NSMenu", "NSMenuItem",
    "NSScrollView", "NSTextView",
] }
```

- [ ] **Step 2: Write failing tests for `prepare_content`**

Create `src/details.rs` with tests only:

```rust
pub fn prepare_content(raw_json: Option<&str>) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_content_none_is_no_data_yet() {
        assert_eq!(prepare_content(None), "No data yet");
    }

    #[test]
    fn prepare_content_valid_json_pretty_prints() {
        let input = r#"{"a":1,"b":2}"#;
        let out = prepare_content(Some(input));
        assert!(out.contains('\n'), "expected newlines from pretty-print, got: {out}");
        assert!(out.contains('"'));
    }

    #[test]
    fn prepare_content_invalid_json_returns_raw() {
        let input = "not json at all";
        assert_eq!(prepare_content(Some(input)), input);
    }

    #[test]
    fn prepare_content_empty_string_returns_empty() {
        assert_eq!(prepare_content(Some("")), "");
    }
}
```

Run: `cargo test -p aiusagebar details::tests`
Expected: FAIL (todo! panics)

- [ ] **Step 3: Implement `prepare_content`**

```rust
pub fn prepare_content(raw_json: Option<&str>) -> String {
    match raw_json {
        None => "No data yet".to_string(),
        Some(body) => serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| serde_json::to_string_pretty(&v).ok())
            .unwrap_or_else(|| body.to_string()),
    }
}
```

- [ ] **Step 4: Add macOS `show` function**

Append to `src/details.rs` after `prepare_content`:

```rust
#[cfg(target_os = "macos")]
pub fn show(provider_name: &str, raw_json: Option<&str>) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSAlert, NSFont, NSScrollView, NSTextView};
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

    let content = prepare_content(raw_json);

    let mtm = MainThreadMarker::new().expect("show() must be called on the main thread");

    let frame = NSRect {
        origin: NSPoint { x: 0.0, y: 0.0 },
        size: NSSize { width: 600.0, height: 300.0 },
    };

    let scroll = unsafe { NSScrollView::initWithFrame(NSScrollView::alloc(), frame) };
    scroll.setHasVerticalScroller(true);
    scroll.setHasHorizontalScroller(false);
    scroll.setAutohidesScrollers(true);

    let tv = unsafe { NSTextView::initWithFrame(NSTextView::alloc(), frame) };
    tv.setEditable(false);
    tv.setSelectable(true);
    unsafe {
        if let Some(font) = NSFont::monospacedSystemFontOfSize_weight(12.0, 0.0) {
            tv.setFont(Some(&font));
        }
    }
    tv.setString(&NSString::from_str(&content));

    scroll.setDocumentView(Some(&tv));

    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(&format!("Details — {}", provider_name)));
    alert.setAccessoryView(Some(&scroll));
    alert.addButtonWithTitle(&NSString::from_str("OK"));
    alert.runModal();
}
```

**Note:** objc2-app-kit 0.3 method names are derived from ObjC selectors. If `NSFont::monospacedSystemFontOfSize_weight` or `NSScrollView::initWithFrame` do not match the exact generated name, consult the crate's generated docs via `cargo doc --open` and adjust the method name accordingly. The same applies to `setHasVerticalScroller`, `setAutohidesScrollers`, `setDocumentView`, `setSelectable`, `setString`.

- [ ] **Step 5: Register the module in `src/main.rs`**

Add `mod details;` near the top of `src/main.rs` alongside other `mod` declarations.

- [ ] **Step 6: Run all tests**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: all pass

- [ ] **Step 7: Commit**

```bash
git add src/details.rs src/main.rs Cargo.toml Cargo.lock
git commit -m "feat: add details module with raw JSON window"
```

---

### Task 6: `src/ui/claude.rs` + `src/ui/copilot.rs` — Details menu item

**Files:**
- Modify: `src/ui/claude.rs`
- Modify: `src/ui/copilot.rs`

**Interfaces:**
- Consumes: existing `append_claude_section`, `append_copilot_section`
- Produces: `append_claude_section(menu, state) -> (Option<MenuId>, MenuId)` — `(setup_id, details_id)`
- Produces: `append_copilot_section(menu, state) -> (Option<MenuId>, MenuId)`
- Produces: `section_item_count` returns `2 + windows.len()` for Ok, `2` otherwise

- [ ] **Step 1: Update `section_item_count` tests in both files**

In `src/ui/claude.rs`, update existing tests:

```rust
#[test]
fn append_claude_section_count_ok_two_windows() {
    let state = UsageState::Ok(
        vec![
            LimitWindow { name: "daily".into(), percent_used: Some(50.0), ..Default::default() },
            LimitWindow { name: "monthly".into(), percent_used: Some(20.0), ..Default::default() },
        ],
        Some("max".into()),
    );
    assert_eq!(section_item_count(&state), 4); // 1 header + 2 windows + 1 details
}

#[test]
fn append_claude_section_count_not_configured() {
    assert_eq!(section_item_count(&UsageState::NotConfigured), 2); // header + details
}
```

In `src/ui/copilot.rs`, update existing tests:

```rust
#[test]
fn append_copilot_section_count_ok_one_window() {
    use crate::provider::UsageState;
    let state = UsageState::Ok(
        vec![make_window("monthly", Some(10.0), None)],
        None,
    );
    assert_eq!(section_item_count(&state), 3); // 1 header + 1 window + 1 details
}

#[test]
fn append_copilot_section_count_not_configured() {
    use crate::provider::UsageState;
    assert_eq!(section_item_count(&UsageState::NotConfigured), 2); // header + details
}
```

Run: `cargo test -p aiusagebar ui::claude::tests ui::copilot::tests`
Expected: FAIL (still returns old counts)

- [ ] **Step 2: Update `section_item_count` in `src/ui/claude.rs`**

```rust
pub(crate) fn section_item_count(state: &UsageState) -> usize {
    match state {
        UsageState::Ok(windows, _) => 2 + windows.len(),
        _ => 2,
    }
}
```

- [ ] **Step 3: Update `section_item_count` in `src/ui/copilot.rs`**

Same change:

```rust
pub(crate) fn section_item_count(state: &UsageState) -> usize {
    match state {
        UsageState::Ok(windows, _) => 2 + windows.len(),
        _ => 2,
    }
}
```

- [ ] **Step 4: Update `append_claude_section` return type and append Details item**

In `src/ui/claude.rs`, replace the entire `append_claude_section` function:

```rust
pub(crate) fn append_claude_section(menu: &Menu, state: &UsageState) -> (Option<MenuId>, MenuId) {
    if let UsageState::NotConfigured = state {
        let item = MenuItem::new(
            header_label(ProviderKind::Claude.display_name(), state),
            true,
            None,
        );
        let setup_id = item.id().clone();
        menu.append(&item).expect("menu append failed");
        let details = MenuItem::new("Details…", true, None);
        let details_id = details.id().clone();
        menu.append(&details).expect("menu append failed");
        return (Some(setup_id), details_id);
    }
    super::append_label(menu, header_label(ProviderKind::Claude.display_name(), state));
    if let UsageState::Ok(windows, _) = state {
        let now = Local::now();
        for w in windows {
            let reset = w
                .resets_at
                .as_deref()
                .map(|s| super::time::format_reset_local(s, now))
                .unwrap_or_else(|| "?".to_string());
            super::append_label(
                menu,
                format!("  {} — {}  resets {}", w.name, pct_label(w.percent_used), reset),
            );
        }
    }
    let details = MenuItem::new("Details…", true, None);
    let details_id = details.id().clone();
    menu.append(&details).expect("menu append failed");
    (None, details_id)
}
```

- [ ] **Step 5: Update `append_copilot_section` return type and append Details item**

In `src/ui/copilot.rs`, replace the entire `append_copilot_section` function:

```rust
pub(crate) fn append_copilot_section(menu: &Menu, state: &UsageState) -> (Option<MenuId>, MenuId) {
    if let UsageState::NotConfigured = state {
        let item = MenuItem::new(
            header_label(ProviderKind::Copilot.display_name(), state),
            true,
            None,
        );
        let setup_id = item.id().clone();
        menu.append(&item).expect("menu append failed");
        let details = MenuItem::new("Details…", true, None);
        let details_id = details.id().clone();
        menu.append(&details).expect("menu append failed");
        return (Some(setup_id), details_id);
    }
    super::append_label(menu, header_label(ProviderKind::Copilot.display_name(), state));
    if let UsageState::Ok(windows, _) = state {
        let now = Local::now();
        for w in windows {
            super::append_label(menu, row_label(w, now));
        }
    }
    let details = MenuItem::new("Details…", true, None);
    let details_id = details.id().clone();
    menu.append(&details).expect("menu append failed");
    (None, details_id)
}
```

- [ ] **Step 6: Run tests (expect failures from layout tests in ui/mod.rs — that's fine)**

Run: `cargo test -p aiusagebar ui::claude ui::copilot`
Expected: `section_item_count` tests pass; `ui::mod` layout tests will fail (fixed in Task 7)

- [ ] **Step 7: Commit**

```bash
git add src/ui/claude.rs src/ui/copilot.rs
git commit -m "feat(ui): add Details menu item to each provider section"
```

---

### Task 7: `src/ui/mod.rs` — `MenuBuild` + `build_menu` + layout tests

**Files:**
- Modify: `src/ui/mod.rs`

**Interfaces:**
- Consumes: `append_claude_section` and `append_copilot_section` returning `(Option<MenuId>, MenuId)` (Task 6)
- Produces: `MenuBuild.details_claude: Option<MenuId>`, `MenuBuild.details_copilot: Option<MenuId>`

- [ ] **Step 1: Add fields to `MenuBuild`**

```rust
pub struct MenuBuild {
    pub menu: Menu,
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
    pub update: Option<MenuId>,
    pub setup_claude: Option<MenuId>,
    pub setup_copilot: Option<MenuId>,
    pub details_claude: Option<MenuId>,
    pub details_copilot: Option<MenuId>,
}
```

- [ ] **Step 2: Update `build_menu` to capture details IDs**

In `build_menu`, replace the provider-section loop:

```rust
let mut setup_claude: Option<MenuId> = None;
let mut setup_copilot: Option<MenuId> = None;
let mut details_claude: Option<MenuId> = None;
let mut details_copilot: Option<MenuId> = None;
for (kind, state) in states {
    match kind {
        ProviderKind::Claude => {
            let (sc, dc) = claude::append_claude_section(&menu, state);
            setup_claude = sc;
            details_claude = Some(dc);
        }
        ProviderKind::Copilot => {
            let (sc, dc) = copilot::append_copilot_section(&menu, state);
            setup_copilot = sc;
            details_copilot = Some(dc);
        }
    }
}
```

And update the `MenuBuild { ... }` literal at the end of `build_menu` to include the new fields:

```rust
MenuBuild {
    menu,
    about: footer.about,
    refresh: footer.refresh,
    quit: footer.quit,
    update: update_id,
    setup_claude,
    setup_copilot,
    details_claude,
    details_copilot,
}
```

- [ ] **Step 3: Update layout index tests in `src/ui/mod.rs`**

With `section_item_count` now returning +1 for all states, all index assertions shift. Update each test:

**`menu_layout_indices_no_providers`** — unchanged (no providers, no sections):
```rust
assert_eq!(layout.refresh_idx, 0);
assert_eq!(layout.quit_idx, 3);
```

**`menu_layout_indices_claude_two_windows`** — Claude Ok with 2 windows:
`section_item_count` = 4 (header + 2 windows + details). refresh advances by 4.
```rust
assert_eq!(layout.header_indices[0].0, 0);
assert_eq!(layout.refresh_idx, 4);  // was 3
assert_eq!(layout.quit_idx, 7);     // was 6
```

**`build_layout_claude_window_items_indices`** — Claude Ok with 2 windows:
```rust
assert_eq!(layout.window_items.len(), 2);
assert_eq!(layout.window_items[0].0, 1);  // unchanged
assert_eq!(layout.window_items[1].0, 2);  // unchanged
assert_eq!(layout.window_items[0].1.name, "5h session");
assert_eq!(layout.window_items[1].1.name, "7d weekly");
// refresh_idx was not asserted in this test — no change needed
```

**`build_layout_copilot_window_items_indices`** — Claude Ok with 2 windows, Copilot Ok with 1 window:
- Claude section: 4 items (header at 0, windows at 1+2, details at 3)
- Copilot section: 3 items (header at 4, window at 5, details at 6)
```rust
assert_eq!(layout.window_items.len(), 3);
assert_eq!(layout.window_items[0].0, 1);   // unchanged
assert_eq!(layout.window_items[1].0, 2);   // unchanged
assert_eq!(layout.window_items[2].0, 5);   // was 4
assert_eq!(layout.window_items[2].1.name, "monthly");
assert_eq!(layout.refresh_idx, 7);         // was 5
assert_eq!(layout.quit_idx, 10);           // was 8
```

**`build_layout_non_ok_state_no_window_items`** — NotConfigured: `section_item_count` = 2. No window items, refresh at 2.
```rust
assert!(layout.window_items.is_empty());
// (refresh_idx not asserted in this test)
```

**`build_layout_with_update_shifts_all_indices_by_2`** — Claude Ok with 1 window, with update banner:
`section_item_count` = 3 (header + 1 window + details). idx starts at 2 (update banner = 2 items).
- header at 2, window at 3, details at 4, refresh at 5
```rust
assert_eq!(layout.header_indices[0].0, 2);  // unchanged
assert_eq!(layout.window_items[0].0, 3);    // unchanged
assert_eq!(layout.refresh_idx, 5);          // was 4
assert_eq!(layout.quit_idx, 8);             // was 7
```

**`build_layout_without_update_unchanged`** — Claude Ok with 1 window, no update:
`section_item_count` = 3 (header + 1 window + details). refresh at 3.
```rust
assert_eq!(layout.header_indices[0].0, 0);
assert_eq!(layout.refresh_idx, 3);          // was 2
```

- [ ] **Step 4: Run all tests**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat(ui): MenuBuild gains details_* IDs, update layout indices"
```

---

### Task 8: `src/main.rs` — App wiring

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `MenuBuild.details_claude`, `MenuBuild.details_copilot` (Task 7)
- Consumes: `details::show(provider_name, raw_json)` (Task 5)
- Consumes: `provider.raw_json()` (Tasks 3, 4)

- [ ] **Step 1: Add `id_details_*` fields to `App` struct**

In the `struct App { ... }` definition, add after `id_setup_copilot`:

```rust
id_details_claude: Option<tray_icon::menu::MenuId>,
id_details_copilot: Option<tray_icon::menu::MenuId>,
```

- [ ] **Step 2: Update `App` initialization in `main` (bottom of file)**

In the `App { ... }` literal where the struct is constructed, add:

```rust
id_details_claude: build.details_claude,
id_details_copilot: build.details_copilot,
```

- [ ] **Step 3: Update `rebuild_menu` (or equivalent) to store new IDs**

In the method that reassigns menu IDs from a `MenuBuild` (around line 90), add:

```rust
self.id_details_claude = build.details_claude;
self.id_details_copilot = build.details_copilot;
```

- [ ] **Step 4: Add Details click handlers in `about_to_wait`**

In the event handler chain (the `if ev.id == self.id_quit { ... } else if ...` block), append two new arms after the existing `id_setup_copilot` arm:

```rust
} else if self.id_details_claude.as_ref().is_some_and(|id| ev.id == *id) {
    let raw = self.providers.iter()
        .find(|p| p.kind() == crate::provider::ProviderKind::Claude)
        .and_then(|p| p.raw_json());
    crate::details::show("Claude", raw.as_deref());
} else if self.id_details_copilot.as_ref().is_some_and(|id| ev.id == *id) {
    let raw = self.providers.iter()
        .find(|p| p.kind() == crate::provider::ProviderKind::Copilot)
        .and_then(|p| p.raw_json());
    crate::details::show("Copilot", raw.as_deref());
}
```

- [ ] **Step 5: Run all tests**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: all pass

- [ ] **Step 6: Build and smoke-test manually**

```bash
make dev
```

Click the tray icon → each provider section should show a "Details…" item at the bottom. Click it → NSAlert window should open titled "Details — Claude" (or Copilot) with the raw JSON (or "No data yet" before first poll).

- [ ] **Step 7: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire Details menu items to show raw HTTP response window"
```
