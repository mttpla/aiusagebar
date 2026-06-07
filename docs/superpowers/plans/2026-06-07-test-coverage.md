# Test Coverage Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove 4 tautological tests, add multi-window icon threshold tests, and add fetch() end-to-end tests via a small logic-extraction refactor.

**Architecture:** No new files. Task 1 deletes noise. Task 2 adds tests directly to the existing `src/icon.rs` test module. Task 3 extracts `do_fetch` from `ClaudeProvider::fetch` behind a closure boundary, then tests all branches without network access.

**Tech Stack:** Rust, cargo test, existing `LimitWindow`/`UsageState`/`HttpError` types.

---

### Task 1: Delete tautological tests

**Files:**
- Modify: `src/http.rs` (lines 41–50, delete 2 tests)
- Modify: `src/provider/mod.rs` (lines 30–52, delete 2 tests)

- [ ] **Step 1: Delete the two dead tests from `src/http.rs`**

Remove `http_error_variants_are_distinct` and `http_error_other_carries_message`. Keep `shared_client_is_reused`. The `tests` block should look like:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_client_is_reused() {
        let a = super::client() as *const reqwest::blocking::Client;
        let b = super::client() as *const reqwest::blocking::Client;
        assert_eq!(a, b, "client() must return the same instance across calls");
    }
}
```

- [ ] **Step 2: Delete the two dead tests from `src/provider/mod.rs`**

Remove `limit_window_fields` and `usage_state_error_carries_message`. The entire `#[cfg(test)]` block becomes:

```rust
#[cfg(test)]
mod tests {}
```

Or just delete the block entirely — no tests remain there.

- [ ] **Step 3: Verify tests still compile and pass**

```bash
cargo test 2>&1 | tail -20
```

Expected: all remaining tests pass, total count drops by 4.

- [ ] **Step 4: Commit**

```bash
git add src/http.rs src/provider/mod.rs
git commit -m "test: remove tautological derive-testing tests"
```

---

### Task 2: Multi-window icon threshold tests

**Files:**
- Modify: `src/icon.rs` (append to existing `tests` module)

These tests cover `IconKind::for_state` when `UsageState::Ok` contains multiple `LimitWindow` entries — the real-world case once Codex/Copilot providers are added.

- [ ] **Step 1: Add 4 tests to the `tests` module in `src/icon.rs`**

The existing `window(pct)` helper already exists in the test module. Add after the last existing `#[test]`:

```rust
    #[test]
    fn alert_when_any_window_above_threshold() {
        let s = UsageState::Ok(vec![window(Some(50.0)), window(Some(90.0))]);
        assert_eq!(IconKind::for_state(&s), IconKind::Alert);
    }

    #[test]
    fn alert_ignores_none_with_high_other() {
        let s = UsageState::Ok(vec![window(None), window(Some(85.0))]);
        assert_eq!(IconKind::for_state(&s), IconKind::Alert);
    }

    #[test]
    fn normal_when_all_windows_none() {
        let s = UsageState::Ok(vec![window(None), window(None)]);
        assert_eq!(IconKind::for_state(&s), IconKind::Normal);
    }

    #[test]
    fn alert_when_percent_high_regardless_of_unlimited_flag() {
        let w = LimitWindow {
            name: "t".into(),
            percent_used: Some(90.0),
            limit: None,
            remaining: None,
            resets_at: None,
            unlimited: true,
        };
        let s = UsageState::Ok(vec![w]);
        assert_eq!(IconKind::for_state(&s), IconKind::Alert);
    }
```

- [ ] **Step 2: Run tests to confirm all pass**

```bash
cargo test icon 2>&1
```

Expected output includes `test icon::tests::alert_when_any_window_above_threshold ... ok` and 3 others.

- [ ] **Step 3: Commit**

```bash
git add src/icon.rs
git commit -m "test: add multi-window icon threshold coverage"
```

---

### Task 3: Extract `do_fetch` and add end-to-end fetch tests

**Files:**
- Modify: `src/provider/claude.rs`

The `ClaudeProvider::fetch` method currently mixes I/O (credential loading, HTTP) with branching logic. Extract all logic into `do_fetch` which receives `CredLoad` and an HTTP closure. The real `fetch` becomes a one-liner that wires them. Tests call `do_fetch` directly with fake closures.

- [ ] **Step 1: Extract `do_fetch` from `ClaudeProvider::fetch`**

Replace the current `impl UsageProvider for ClaudeProvider` block with:

```rust
fn do_fetch(
    creds: CredLoad,
    http: &dyn Fn(&str) -> Result<String, HttpError>,
    last_ok: &Mutex<Option<Vec<LimitWindow>>>,
) -> UsageState {
    let creds = match creds {
        CredLoad::NotConfigured => return UsageState::NotConfigured,
        CredLoad::Malformed(e) => return UsageState::Error(format!("Malformed credentials: {}", e)),
        CredLoad::Ok(c) => c,
    };
    if is_expired(creds.expires_at_ms) {
        let date = format_expiry_date(creds.expires_at_ms);
        return UsageState::Stale(format!("Expired on {} — run: claude login", date));
    }
    match http(&creds.access_token) {
        Ok(body) => match parse_response(&body) {
            Ok(windows) => {
                let windows = windows.to_vec();
                *last_ok.lock().unwrap() = Some(windows.clone());
                UsageState::Ok(windows)
            }
            Err(e) => UsageState::Error(format!("Parse error: {}", e)),
        },
        Err(HttpError::Unauthorized) => {
            UsageState::Stale("Token rejected — run: claude login".to_string())
        }
        Err(HttpError::RateLimited) => {
            last_ok
                .lock()
                .unwrap()
                .clone()
                .map(UsageState::Ok)
                .unwrap_or_else(|| UsageState::Error("Rate limited (no cache)".to_string()))
        }
        Err(HttpError::Other(e)) => UsageState::Error(e),
    }
}

impl UsageProvider for ClaudeProvider {
    fn name(&self) -> &'static str { "Anthropic" }

    fn fetch(&self) -> UsageState {
        let ua = get_user_agent();
        do_fetch(
            load_credentials(),
            &|token| crate::http::get(USAGE_URL, token, &[("User-Agent", ua)]),
            &self.last_ok,
        )
    }
}
```

- [ ] **Step 2: Confirm it compiles**

```bash
cargo check 2>&1
```

Expected: no errors.

- [ ] **Step 3: Add 8 tests to the `tests` module in `src/provider/claude.rs`**

First, add two import lines inside `mod tests` right after the existing `use super::*;`:

```rust
    use crate::provider::{LimitWindow, UsageState};
    use crate::http::HttpError;
    use std::sync::Mutex;
```

Then append the helpers and tests after the last existing `#[test]`:

```rust
    fn ok_creds() -> CredLoad {
        CredLoad::Ok(ClaudeCredentials {
            access_token: "tok".to_string(),
            expires_at_ms: 9_999_999_999_000,
        })
    }

    fn last_ok() -> Mutex<Option<Vec<LimitWindow>>> {
        Mutex::new(None)
    }

    fn valid_body() -> &'static str {
        r#"{"five_hour":{"utilization":50.0,"resets_at":"2026-12-01T00:00:00Z"},"seven_day":{"utilization":20.0,"resets_at":"2026-12-07T00:00:00Z"}}"#
    }

    #[test]
    fn do_fetch_not_configured() {
        let state = super::do_fetch(CredLoad::NotConfigured, &|_| unreachable!(), &last_ok());
        assert_eq!(state, UsageState::NotConfigured);
    }

    #[test]
    fn do_fetch_malformed_creds() {
        let state = super::do_fetch(
            CredLoad::Malformed("bad json".to_string()),
            &|_| unreachable!(),
            &last_ok(),
        );
        assert!(matches!(state, UsageState::Error(ref e) if e.contains("Malformed")));
    }

    #[test]
    fn do_fetch_expired_token_returns_stale() {
        let creds = CredLoad::Ok(ClaudeCredentials {
            access_token: "tok".to_string(),
            expires_at_ms: 1_000,
        });
        let state = super::do_fetch(creds, &|_| unreachable!(), &last_ok());
        assert!(matches!(state, UsageState::Stale(ref s) if s.contains("Expired on")));
    }

    #[test]
    fn do_fetch_401_returns_stale() {
        let state = super::do_fetch(
            ok_creds(),
            &|_| Err(HttpError::Unauthorized),
            &last_ok(),
        );
        assert!(matches!(state, UsageState::Stale(ref s) if s.contains("Token rejected")));
    }

    #[test]
    fn do_fetch_429_no_cache_returns_error() {
        let state = super::do_fetch(
            ok_creds(),
            &|_| Err(HttpError::RateLimited),
            &last_ok(),
        );
        assert!(matches!(state, UsageState::Error(ref s) if s.contains("Rate limited")));
    }

    #[test]
    fn do_fetch_429_with_cache_returns_cached_ok() {
        let cache = Mutex::new(Some(vec![LimitWindow {
            name: "5h session".to_string(),
            percent_used: Some(42.0),
            limit: None,
            remaining: None,
            resets_at: None,
            unlimited: false,
        }]));
        let state = super::do_fetch(ok_creds(), &|_| Err(HttpError::RateLimited), &cache);
        assert!(matches!(state, UsageState::Ok(ref w) if w[0].percent_used == Some(42.0)));
    }

    #[test]
    fn do_fetch_200_bad_body_returns_error() {
        let state = super::do_fetch(
            ok_creds(),
            &|_| Ok("garbage".to_string()),
            &last_ok(),
        );
        assert!(matches!(state, UsageState::Error(ref s) if s.contains("Parse error")));
    }

    #[test]
    fn do_fetch_200_valid_returns_ok_and_populates_cache() {
        let cache = last_ok();
        let state = super::do_fetch(ok_creds(), &|_| Ok(valid_body().to_string()), &cache);
        assert!(matches!(state, UsageState::Ok(ref w) if w.len() == 2));
        assert!(cache.lock().unwrap().is_some(), "cache must be populated after success");
    }
```

- [ ] **Step 4: Run the new tests**

```bash
cargo test do_fetch 2>&1
```

Expected: all 8 pass.

- [ ] **Step 5: Run full test suite**

```bash
cargo test 2>&1 | tail -20
```

Expected: all tests pass. Count should be 29 (25 existing − 4 deleted) + 4 icon + 8 do_fetch = 33.

- [ ] **Step 6: Commit**

```bash
git add src/provider/claude.rs
git commit -m "refactor: extract do_fetch for testability, add 8 fetch branch tests"
```
