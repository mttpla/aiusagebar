# Claude Account Identity Display — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show the logged-in Anthropic account (`Claude — mttpla@gmail.com (pro)`) inline in the Claude section header by fetching `GET /api/oauth/profile`.

**Architecture:** `UsageState::Ok` gains an `Option<String>` profile field; `ClaudeProvider` lazily fetches the profile into a `Mutex<Option<ProfileData>>` on first call and resets it on any `/usage` error; `main.rs::build_menu` renders the profile string as a header suffix.

**Tech Stack:** Rust, serde_json (already in Cargo.toml), `crate::http::get` (existing helper), same Bearer + User-Agent auth as `/usage`.

---

## File Map

| File | Change |
|---|---|
| `src/provider/mod.rs` | `Ok(Vec<LimitWindow>)` → `Ok(Vec<LimitWindow>, Option<String>)` |
| `src/icon.rs` | Fix `for_state` match pattern + all test literals |
| `src/provider/claude.rs` | New serde structs, `ProfileData`, `fetch_profile`, updated `do_fetch` signature, `profile` mutex on provider, `name()` rename |
| `src/main.rs` | `build_menu` renders identity header from new `Ok` fields |

---

## Task 1: Extend `UsageState::Ok` and fix all compile errors

**Files:**
- Modify: `src/provider/mod.rs`
- Modify: `src/icon.rs`
- Modify: `src/provider/claude.rs`
- Modify: `src/main.rs`

This task has no new logic — it only extends the enum variant and silences the cascade of compile errors. All existing tests must still pass at the end.

- [ ] **Step 1: Change `UsageState::Ok` in `src/provider/mod.rs`**

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum UsageState {
    NotConfigured,
    Stale(String),
    Ok(Vec<LimitWindow>, Option<String>),
    Error(String),
}
```

- [ ] **Step 2: Fix `src/icon.rs` — `for_state` match pattern**

Change line 13:
```rust
// before
UsageState::Ok(windows) => {
// after
UsageState::Ok(windows, _) => {
```

- [ ] **Step 3: Fix `src/icon.rs` — all test `UsageState::Ok` literals**

Every `UsageState::Ok(vec![...])` in the test module needs a second field. Replace all occurrences:

```rust
// pattern to replace (all tests in icon.rs)
UsageState::Ok(vec![...])
// becomes
UsageState::Ok(vec![...], None)
```

Affected tests: `normal_under_threshold`, `alert_at_threshold`, `normal_when_percent_unknown`, `alert_when_any_window_above_threshold`, `alert_ignores_none_with_high_other`, `normal_when_all_windows_none`, `fold_alert_beats_error` (`high`), `fold_alert_beats_unavailable_regardless_of_order` (`high`), `fold_error_beats_normal` (`ok`), `fold_all_normal` (`a`, `b`), `alert_when_percent_high_regardless_of_unlimited_flag`.

- [ ] **Step 4: Fix `src/provider/claude.rs` — `do_fetch` return sites**

In `do_fetch`, change the two `UsageState::Ok(...)` constructions:

```rust
// in the Ok(body) => Ok(windows) arm:
UsageState::Ok(windows, None)   // was: UsageState::Ok(windows)

// in the RateLimited arm:
.map(|w| UsageState::Ok(w, None))   // was: .map(UsageState::Ok)
```

- [ ] **Step 5: Fix `src/provider/claude.rs` — test match patterns**

Two tests match on `UsageState::Ok`:

```rust
// do_fetch_429_with_cache_returns_cached_ok
assert!(matches!(state, UsageState::Ok(ref w, _) if w[0].percent_used == Some(42.0)));

// do_fetch_200_valid_returns_ok_and_populates_cache
assert!(matches!(state, UsageState::Ok(ref w, _) if w.len() == 2));
```

- [ ] **Step 6: Fix `src/main.rs` — `build_menu` match pattern (temporary)**

Change the `Ok` arm to compile; full UI is in Task 4:

```rust
UsageState::Ok(windows, _) => {
    append_label(&menu, name.to_string());
    for w in windows {
        // ... unchanged
    }
}
```

- [ ] **Step 7: Verify compilation and tests pass**

```bash
cargo test 2>&1 | tail -5
```

Expected: all tests pass, zero warnings about unused variables.

- [ ] **Step 8: Commit**

```bash
git add src/provider/mod.rs src/icon.rs src/provider/claude.rs src/main.rs
git commit -m "refactor(provider): extend UsageState::Ok with optional profile string"
```

---

## Task 2: Add profile serde types and `parse_profile_response` (TDD)

**Files:**
- Modify: `src/provider/claude.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block in `claude.rs`:

```rust
#[test]
fn parse_profile_max_plan() {
    let body = r#"{"account":{"email":"a@b.com","has_claude_pro":true,"has_claude_max":true}}"#;
    let pd = super::parse_profile_response(body).unwrap();
    assert_eq!(pd.email, "a@b.com");
    assert_eq!(pd.plan, "max");
}

#[test]
fn parse_profile_pro_plan() {
    let body = r#"{"account":{"email":"a@b.com","has_claude_pro":true,"has_claude_max":false}}"#;
    let pd = super::parse_profile_response(body).unwrap();
    assert_eq!(pd.plan, "pro");
}

#[test]
fn parse_profile_free_plan() {
    let body = r#"{"account":{"email":"a@b.com","has_claude_pro":false,"has_claude_max":false}}"#;
    let pd = super::parse_profile_response(body).unwrap();
    assert_eq!(pd.plan, "free");
}

#[test]
fn parse_profile_missing_account_field_is_error() {
    assert!(super::parse_profile_response("{}").is_err());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test parse_profile 2>&1 | tail -10
```

Expected: compile error — `parse_profile_response` not found.

- [ ] **Step 3: Add types and implementation**

Add after the existing `const USAGE_URL` line in `claude.rs`:

```rust
const PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";

#[derive(Deserialize)]
struct ProfileAccount {
    email: String,
    has_claude_pro: bool,
    has_claude_max: bool,
}

#[derive(Deserialize)]
struct ProfileResponse {
    account: ProfileAccount,
}

struct ProfileData {
    email: String,
    plan: String,
}

fn plan_label(has_max: bool, has_pro: bool) -> &'static str {
    if has_max { "max" } else if has_pro { "pro" } else { "free" }
}

fn parse_profile_response(body: &str) -> Result<ProfileData, String> {
    let resp: ProfileResponse = serde_json::from_str(body).map_err(|e| e.to_string())?;
    Ok(ProfileData {
        email: resp.account.email,
        plan: plan_label(resp.account.has_claude_max, resp.account.has_claude_pro).to_string(),
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test parse_profile 2>&1 | tail -10
```

Expected: 4 tests pass.

- [ ] **Step 5: Run full test suite**

```bash
cargo test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/provider/claude.rs
git commit -m "feat(claude): add profile serde types and parse_profile_response"
```

---

## Task 3: Wire profile into `ClaudeProvider` and update `do_fetch`

**Files:**
- Modify: `src/provider/claude.rs`

- [ ] **Step 1: Write failing tests for profile passthrough in `do_fetch`**

Add to the test module:

```rust
#[test]
fn do_fetch_passes_profile_string_into_ok() {
    let cache = empty_cache();
    let state = super::do_fetch(
        ok_creds(),
        &|_| Ok(valid_body().to_string()),
        &cache,
        Some("a@b.com (pro)".to_string()),
    );
    assert!(
        matches!(state, UsageState::Ok(_, ref p) if p.as_deref() == Some("a@b.com (pro)")),
        "profile string must be preserved in Ok variant"
    );
}

#[test]
fn do_fetch_none_profile_propagates_to_ok() {
    let cache = empty_cache();
    let state = super::do_fetch(
        ok_creds(),
        &|_| Ok(valid_body().to_string()),
        &cache,
        None,
    );
    assert!(matches!(state, UsageState::Ok(_, None)));
}

#[test]
fn do_fetch_429_with_cache_includes_profile_string() {
    let cache = Mutex::new(Some(vec![LimitWindow {
        name: "5h session".to_string(),
        percent_used: Some(42.0),
        limit: None,
        remaining: None,
        resets_at: None,
        unlimited: false,
    }]));
    let state = super::do_fetch(
        ok_creds(),
        &|_| Err(HttpError::RateLimited),
        &cache,
        Some("a@b.com (pro)".to_string()),
    );
    assert!(
        matches!(state, UsageState::Ok(ref w, ref p)
            if w[0].percent_used == Some(42.0) && p.as_deref() == Some("a@b.com (pro)")),
        "cached Ok must carry profile string on rate limit"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test do_fetch_passes_profile 2>&1 | tail -10
```

Expected: compile error — `do_fetch` called with wrong number of arguments.

- [ ] **Step 3: Update `do_fetch` signature to accept `profile_string`**

Change the `do_fetch` function signature and its two `UsageState::Ok` sites:

```rust
fn do_fetch(
    creds: CredLoad,
    http: &dyn Fn(&str) -> Result<String, HttpError>,
    last_ok: &Mutex<Option<Vec<LimitWindow>>>,
    profile_string: Option<String>,
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
                UsageState::Ok(windows, profile_string)
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
                .map(|w| UsageState::Ok(w, profile_string))
                .unwrap_or_else(|| UsageState::Error("Rate limited (no cache)".to_string()))
        }
        Err(HttpError::Other(e)) => UsageState::Error(e),
    }
}
```

- [ ] **Step 4: Update existing `do_fetch` test call sites to add `None`**

Each existing direct call to `super::do_fetch(...)` in the test module needs a fourth `None` argument. Affected tests: `do_fetch_not_configured`, `do_fetch_malformed_creds`, `do_fetch_expired_token_returns_stale`, `do_fetch_401_returns_stale`, `do_fetch_429_no_cache_returns_error`, `do_fetch_429_with_cache_returns_cached_ok`, `do_fetch_200_bad_body_returns_error`, `do_fetch_200_valid_returns_ok_and_populates_cache`.

Example (apply same pattern to all):
```rust
// before
let state = super::do_fetch(CredLoad::NotConfigured, &|_| unreachable!(), &empty_cache());
// after
let state = super::do_fetch(CredLoad::NotConfigured, &|_| unreachable!(), &empty_cache(), None);
```

- [ ] **Step 5: Add `fetch_profile` helper and `profile` field to `ClaudeProvider`**

Add `fetch_profile` after `parse_profile_response`:

```rust
fn fetch_profile(token: &str, ua: &str) -> Option<ProfileData> {
    crate::http::get(PROFILE_URL, token, &[("User-Agent", ua)])
        .ok()
        .and_then(|body| parse_profile_response(&body).ok())
}
```

Update `ClaudeProvider`:

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

- [ ] **Step 6: Update `ClaudeProvider::fetch()` to lazy-fetch profile and reset on error**

```rust
fn fetch(&self) -> UsageState {
    let ua = get_user_agent();
    let creds = load_credentials();

    {
        let mut profile = self.profile.lock().unwrap();
        if profile.is_none() {
            if let CredLoad::Ok(ref c) = creds {
                *profile = fetch_profile(&c.access_token, ua);
            }
        }
    }

    let profile_string = self
        .profile
        .lock()
        .unwrap()
        .as_ref()
        .map(|p| format!("{} ({})", p.email, p.plan));

    let state = do_fetch(
        creds,
        &|token| crate::http::get(USAGE_URL, token, &[("User-Agent", ua)]),
        &self.last_ok,
        profile_string,
    );

    if matches!(state, UsageState::Stale(_) | UsageState::Error(_)) {
        *self.profile.lock().unwrap() = None;
    }

    state
}
```

- [ ] **Step 7: Run all tests**

```bash
cargo test 2>&1 | tail -10
```

Expected: all tests pass including the 3 new profile passthrough tests.

- [ ] **Step 8: Commit**

```bash
git add src/provider/claude.rs
git commit -m "feat(claude): wire profile lazy-fetch into ClaudeProvider"
```

---

## Task 4: UI rendering + rename provider to "Claude"

**Files:**
- Modify: `src/main.rs`
- Modify: `src/provider/claude.rs`

- [ ] **Step 1: Update `build_menu` in `src/main.rs` to render identity header**

Replace the `Ok` arm in `build_menu` (currently `Ok(windows, _)`):

```rust
UsageState::Ok(windows, profile) => {
    let header = match profile {
        Some(p) => format!("{} — {}", name, p),
        None => format!("{} — account unavailable", name),
    };
    append_label(&menu, header);
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
```

- [ ] **Step 2: Rename provider in `src/provider/claude.rs`**

```rust
fn name(&self) -> &'static str { "Claude" }
```

- [ ] **Step 3: Build and smoke-test**

```bash
make dev
```

Open the tray menu. With a valid Claude token the header should read `Claude — you@example.com (pro|max|free)`. With no credentials it should read `Claude: not configured`. On fetch error it should read `Claude — account unavailable`.

- [ ] **Step 4: Run full test suite**

```bash
cargo test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/provider/claude.rs
git commit -m "feat(claude): show account identity in section header"
```

---

## Self-Review

**Spec coverage:**

| Spec requirement | Covered by |
|---|---|
| Rename section header "Anthropic" → "Claude" | Task 4 Step 2 (`name()`) |
| Fetch `GET /api/oauth/profile` with same auth/UA | Task 3 Steps 5-6 |
| `ProfileData` internal struct | Task 2 Step 3 |
| `profile: Mutex<Option<ProfileData>>` on provider | Task 3 Step 5 |
| `UsageState::Ok` gains `Option<String>` | Task 1 |
| Plan derivation: `has_claude_max → "max"`, etc. | Task 2 Step 3 (`plan_label`) |
| Lazy fetch on `profile == None` | Task 3 Step 6 |
| Reset to `None` on any `/usage` error | Task 3 Step 6 (`matches!(state, Stale|Error)`) |
| No retry on 401/403/429 within same call | `fetch_profile` returns `None` on HTTP error (no special casing needed — `None` stays `None`) |
| `Ok(windows, Some(p))` → `"{name} — {p}"` header | Task 4 Step 1 |
| `Ok(windows, None)` → `"{name} — account unavailable"` | Task 4 Step 1 |
| `NotConfigured / Stale / Error` → `"{name}"` no suffix | existing arms unchanged |

**Placeholder scan:** None found. All code blocks are complete.

**Type consistency:**
- `ProfileData.email: String`, `ProfileData.plan: String` — used identically in `parse_profile_response` (Task 2) and `ClaudeProvider::fetch` format string (Task 3).
- `do_fetch` 4th param is `Option<String>` in signature (Task 3 Step 3) and all call sites (Tasks 1, 3).
- `UsageState::Ok(Vec<LimitWindow>, Option<String>)` — destructured as `Ok(windows, profile)` in Task 4 and `Ok(ref w, _)` in icon tests.
