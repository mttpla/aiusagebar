# Diagnostic Instrumentation Sweep Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `diag!` call sites to every currently-silent error path in `keychain.rs`, `http.rs::get_public`, and `claude.rs`, so all reachable failures surface in the diagnostic log.

**Architecture:** `diag!` (from `src/diag.rs`, shipped in card #44) pushes a timestamped, file:line-tagged entry to a 100-slot ring buffer. This sweep adds emission at the error sites that #44 left uncovered. Logging only — no return values, error variants, or control flow change. Two complementary layers for HTTP: `http.rs` already logs status+URL+body inside `get()`; this plan adds provider-context logging at the `claude.rs` call-site arms, and fills the entirely-silent `get_public`.

**Tech Stack:** Rust, `ureq` (HTTP), `security-framework` v3 (Keychain), `serde_json`.

## Global Constraints

- **No new dependencies.** (card rule + project hard constraint)
- **Logging only.** No new error variants, no error-type method changes, no control-flow change. (card "Out of scope")
- **Messages must be specific:** include operation, URL/service, and the error value. Bad: `"fetch failed"`. Good: `"Claude usage fetch failed at {}: {}", url, e`. (card rule)
- **Use `inspect_err` / explicit `match`, not methods on error types** — error types lack call-site context. (card rule)
- **All string literals in English.** Italian only ever at runtime i18n, never in source. (project rule)
- **Never `pub`; default `pub(crate)`/private** in this binary crate. (project rule)
- **Do NOT log the normal "not configured" path.** A missing Keychain item (`errSecItemNotFound`, OSStatus `-25300`) is the expected NotConfigured state and is read on every 180s poll — logging it would spam the ring buffer. Only genuine read failures get a `diag!`.
- Copilot provider **is in scope** (Task 4) — it exists and is built (multi-account). Codex remains out of scope (not yet written).
- Before every commit: `cargo clippy -- -D warnings && cargo test` must pass. (project rule)

---

### Task 1: Instrument `keychain.rs` read failures (with not-found filtering)

Both functions currently swallow `security_framework::base::Error` silently. Restructure to inspect the error, log only genuine failures, and log malformed (non-UTF-8) items. A pure helper decides "loggable vs expected not-found" so the decision is unit-testable without touching the live Keychain.

**Files:**
- Modify: `src/keychain.rs` — `read_generic_password` (lines 1-7), `enumerate_generic_passwords` `.search()` call (lines 21-27); add helper + const; add tests in the existing `tests` module.

**Interfaces:**
- Consumes: `crate::diag!` macro, `crate::diag::Level::Err` (existing).
- Produces: `fn keychain_error_is_loggable(code: i32) -> bool` (private, `#[cfg(target_os = "macos")]`); `const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300` (private, macos-only). No public surface change.

- [ ] **Step 1: Write the failing test for the not-found filter**

Add to the `#[cfg(test)] mod tests` block in `src/keychain.rs`:

```rust
    #[cfg(target_os = "macos")]
    #[test]
    fn item_not_found_is_not_loggable() {
        // errSecItemNotFound is the expected "no credential" path — must not log.
        assert!(!super::keychain_error_is_loggable(super::ERR_SEC_ITEM_NOT_FOUND));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn genuine_read_error_is_loggable() {
        // errSecInteractionNotAllowed (-25308): a real failure worth logging.
        assert!(super::keychain_error_is_loggable(-25308));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib keychain 2>&1 | tail -20`
Expected: FAIL — `cannot find function keychain_error_is_loggable` / `cannot find value ERR_SEC_ITEM_NOT_FOUND`.

- [ ] **Step 3: Add the const and pure helper**

Insert near the top of `src/keychain.rs`, after the first `use`/before `read_generic_password` (macos block):

```rust
/// OSStatus for "no such Keychain item" — the expected NotConfigured path.
#[cfg(target_os = "macos")]
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

/// True for Keychain errors worth logging — genuine read failures, not the
/// expected `errSecItemNotFound` that simply means the provider is unconfigured.
#[cfg(target_os = "macos")]
fn keychain_error_is_loggable(code: i32) -> bool {
    code != ERR_SEC_ITEM_NOT_FOUND
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib keychain 2>&1 | tail -20`
Expected: PASS — both new tests green.

- [ ] **Step 5: Instrument `read_generic_password`**

Replace the macos `read_generic_password` body (current lines 2-7):

```rust
#[cfg(target_os = "macos")]
pub(crate) fn read_generic_password(service: &str, account: &str) -> Option<String> {
    use security_framework::passwords::get_generic_password;
    let bytes = match get_generic_password(service, account) {
        Ok(b) => b,
        Err(e) => {
            if keychain_error_is_loggable(e.code()) {
                crate::diag!(
                    crate::diag::Level::Err,
                    "Keychain read failed for service {} (status {}): {}",
                    service,
                    e.code(),
                    e
                );
            }
            return None;
        }
    };
    match String::from_utf8(bytes) {
        Ok(s) => Some(s),
        Err(e) => {
            crate::diag!(
                crate::diag::Level::Err,
                "Keychain item for service {} is not valid UTF-8: {}",
                service,
                e
            );
            None
        }
    }
}
```

- [ ] **Step 6: Instrument `enumerate_generic_passwords` search failure**

In the macos `enumerate_generic_passwords`, replace the `.search().unwrap_or_default();` call (current lines 26-27) so the result is bound via `match`:

```rust
    let results = match ItemSearchOptions::new()
        .class(ItemClass::generic_password())
        .service(service)
        .limit(Limit::All)
        .load_attributes(true)
        .search()
    {
        Ok(r) => r,
        Err(e) => {
            if keychain_error_is_loggable(e.code()) {
                crate::diag!(
                    crate::diag::Level::Err,
                    "Keychain enumerate failed for service {} (status {}): {}",
                    service,
                    e.code(),
                    e
                );
            }
            Vec::new()
        }
    };
```

- [ ] **Step 7: Verify the full suite + clippy stay green**

Run: `cargo clippy -- -D warnings && cargo test 2>&1 | tail -20`
Expected: clippy clean; all tests pass (the existing `missing_service_returns_none` and `enumerate_nonexistent_service_returns_empty` still pass — they hit `errSecItemNotFound`, which is filtered, so they return None/empty silently as before).

- [ ] **Step 8: Commit**

```bash
git add src/keychain.rs
git commit -m "feat: log Keychain read/enumerate failures, skip not-found"
```

---

### Task 2: Instrument `http.rs::get_public`

`get()` already logs network + all status errors (lines 36-61). `get_public` (lines 64-80) is entirely silent despite producing the same `HttpError` variants and being used by `update_check.rs`. Add `diag!` at its network, body-read, and status-error sites, mirroring `get()`'s message style. Pure side-effect addition — return values are unchanged, so no new unit test (the existing structural tests and `cargo test` are the gate).

**Files:**
- Modify: `src/http.rs` — `get_public` (lines 64-80).

**Interfaces:**
- Consumes: `crate::diag!`, `crate::diag::Level::Err` (existing).
- Produces: nothing new — signature `fn get_public(url: &str) -> Result<String, HttpError>` unchanged.

- [ ] **Step 1: Confirm the baseline compiles and tests pass**

Run: `cargo test --lib http 2>&1 | tail -20`
Expected: PASS — `shared_agent_is_reused`, `get_public_function_exists_and_compiles`, `get_returns_tuple` all green. (Establishes the starting point; this task adds no behavior to assert via unit test.)

- [ ] **Step 2: Rewrite `get_public` with diagnostics**

Replace the whole `get_public` function (current lines 64-80):

```rust
pub(crate) fn get_public(url: &str) -> Result<String, HttpError> {
    let resp = agent()
        .get(url)
        .header("User-Agent", concat!("aiusagebar/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| {
            crate::diag!(crate::diag::Level::Err, "HTTP request to {} failed: {}", url, e);
            HttpError::Other(e.to_string())
        })?;
    let status = resp.status().as_u16();
    let result = match status {
        200 => resp.into_body().read_to_string().map_err(|e| {
            crate::diag!(crate::diag::Level::Err, "Reading body from {} failed: {}", url, e);
            HttpError::Other(e.to_string())
        }),
        401 => Err(HttpError::Unauthorized),
        429 => Err(HttpError::RateLimited),
        c @ 500..=599 => Err(HttpError::ServerError(c)),
        code => Err(HttpError::Other(format!("HTTP {}", code))),
    };
    if result.is_err() && status != 200 {
        crate::diag!(crate::diag::Level::Err, "HTTP {} from {}", status, url);
    }
    result
}
```

Note: the `status != 200` guard prevents double-logging the 200 body-read failure (already logged inside the `map_err`).

- [ ] **Step 3: Verify compile, tests, and clippy**

Run: `cargo clippy -- -D warnings && cargo test --lib http 2>&1 | tail -20`
Expected: clippy clean; the three `http` tests pass; behavior unchanged.

- [ ] **Step 4: Commit**

```bash
git add src/http.rs
git commit -m "feat: log network/status/body errors in http get_public"
```

---

### Task 3: Instrument `claude.rs` HTTP-error arms and profile fetch

`do_fetch` already logs malformed creds + parse errors (lines 207, 228-232). The four `HttpError` match arms (lines 236-249) and `fetch_profile` (lines 192-195) swallow failures with no provider context. This is the "provider layer" of the both-layers HTTP logging decision — `http.rs::get` logs status+URL+body; here we add the provider/operation context. Logging-only; the existing `do_fetch_*` tests already exercise every arm and assert the returned state, so they are the regression gate (no new tests).

**Files:**
- Modify: `src/provider/claude.rs` — `fetch_profile` (lines 192-195) and the four `Err(HttpError::...)` arms in `do_fetch` (lines 236-249).

**Interfaces:**
- Consumes: `crate::diag!`, `crate::diag::Level::Err`, module consts `USAGE_URL` / `PROFILE_URL` (existing).
- Produces: nothing new — `do_fetch` and `fetch_profile` signatures unchanged.

- [ ] **Step 1: Confirm baseline `do_fetch` tests pass**

Run: `cargo test --lib claude 2>&1 | tail -25`
Expected: PASS — including `do_fetch_401_returns_stale`, `do_fetch_429_no_cache_returns_error`, `do_fetch_429_with_cache_returns_cached_ok`. These arms get `diag!` added below; the asserts on returned state must remain green afterward.

- [ ] **Step 2: Add provider context to `fetch_profile`**

Replace `fetch_profile` (current lines 192-195):

```rust
fn fetch_profile(token: &str, ua: &str) -> Option<ProfileData> {
    let (result, _) = crate::http::get(PROFILE_URL, token, &[("User-Agent", ua)]);
    let body = result.ok()?;
    parse_profile_response(&body)
        .inspect_err(|e| crate::diag!(crate::diag::Level::Err, "Claude profile parse failed: {}", e))
        .ok()
}
```

(The HTTP-status failure of the profile request is already logged inside `http::get`; this adds the parse-failure case, which was silent.)

- [ ] **Step 3: Add diag! to the four HTTP-error arms in `do_fetch`**

Replace the four error arms (current lines 236-249) with:

```rust
        Err(HttpError::Unauthorized) => {
            crate::diag!(crate::diag::Level::Err, "Claude usage fetch unauthorized (401) at {}", USAGE_URL);
            (UsageState::Stale("Token rejected — run: claude login".to_string()), Some(HttpError::Unauthorized))
        }
        Err(HttpError::RateLimited) => {
            crate::diag!(crate::diag::Level::Err, "Claude usage fetch rate limited (429) at {}", USAGE_URL);
            let state = last_ok
                .lock()
                .unwrap()
                .clone()
                .map(|w| UsageState::Ok(w, profile_string))
                .unwrap_or_else(|| UsageState::Error("Rate limited (no cache)".to_string()));
            (state, Some(HttpError::RateLimited))
        }
        Err(HttpError::ServerError(c)) => {
            crate::diag!(crate::diag::Level::Err, "Claude usage fetch server error {} at {}", c, USAGE_URL);
            (UsageState::Error(format!("Server error {c}")), Some(HttpError::ServerError(c)))
        }
        Err(HttpError::Other(e)) => {
            crate::diag!(crate::diag::Level::Err, "Claude usage fetch failed at {}: {}", USAGE_URL, e);
            (UsageState::Error(e), None)
        }
```

Note: in the `Other(e)` arm the `diag!` borrows `e` before it is moved into `UsageState::Error(e)` — ordering is correct, no clone needed.

- [ ] **Step 4: Verify the suite and clippy**

Run: `cargo clippy -- -D warnings && cargo test 2>&1 | tail -25`
Expected: clippy clean; all tests pass — the `do_fetch_*` arm tests assert returned state, which is unchanged; the added `diag!` calls have no effect on assertions.

- [ ] **Step 5: Commit**

```bash
git add src/provider/claude.rs
git commit -m "feat: log Claude HTTP-error arms and profile parse failures"
```

---

### Task 4: Instrument `copilot.rs` per-account error arms

`do_copilot_fetch` loops over accounts and turns each failure into a user-facing sentinel `LimitWindow`, but emits no `diag!`. Add provider-context logging (including the account name) to the parse-error path and the four `HttpError` arms. Per the both-layers decision, `http.rs::get` already logs status+URL+body; this adds the Copilot/account operation context. Logging-only; the existing `do_copilot_fetch` tests already exercise every arm and assert returned state, so they are the regression gate (no new tests).

**Files:**
- Modify: `src/provider/copilot.rs` — the `match result` arms in `do_copilot_fetch` (lines 72-89).

**Interfaces:**
- Consumes: `crate::diag!`, `crate::diag::Level::Err` (existing). `account` is the loop binding `&String` from `for (account, token) in &tokens`.
- Produces: nothing new — `do_copilot_fetch` signature unchanged.

- [ ] **Step 1: Confirm baseline `do_copilot_fetch` tests pass**

Run: `cargo test --lib copilot 2>&1 | tail -25`
Expected: PASS — including `fetch_all_401_returns_stale`, `fetch_other_error_returns_error`, `fetch_200_bad_body_returns_error`, `fetch_mixed_success_and_401_returns_ok_with_sentinel`. These arms get `diag!` added below; the asserts on returned state must remain green afterward.

- [ ] **Step 2: Add diag! to the five error arms**

Replace the `match result { ... }` block (current lines 72-89):

```rust
        match result {
            Ok(body) => match parse_copilot_response(&body) {
                Ok(windows) => ok_windows.extend(windows),
                Err(e) => {
                    crate::diag!(crate::diag::Level::Err, "Copilot parse failed for @{}: {}", account, e);
                    error_msgs.push(format!("@{} — {}", account, e));
                }
            },
            Err(HttpError::Unauthorized) => {
                crate::diag!(crate::diag::Level::Err, "Copilot usage fetch unauthorized (401) for @{}", account);
                stale_accounts.push(account.clone());
            }
            Err(HttpError::RateLimited) => {
                crate::diag!(crate::diag::Level::Err, "Copilot usage fetch rate limited (429) for @{}", account);
                error_msgs.push(format!("@{} — rate limited", account));
                backoff_err = Some(HttpError::RateLimited);
            }
            Err(HttpError::ServerError(c)) => {
                crate::diag!(crate::diag::Level::Err, "Copilot usage fetch server error {} for @{}", c, account);
                error_msgs.push(format!("@{} — server error {c}", account));
                if backoff_err.is_none() {
                    backoff_err = Some(HttpError::ServerError(c));
                }
            }
            Err(HttpError::Other(e)) => {
                crate::diag!(crate::diag::Level::Err, "Copilot usage fetch failed for @{}: {}", account, e);
                error_msgs.push(format!("@{} — {}", account, e));
            }
        }
```

- [ ] **Step 3: Verify the suite and clippy**

Run: `cargo clippy -- -D warnings && cargo test 2>&1 | tail -25`
Expected: clippy clean; all tests pass — the `do_copilot_fetch` arm tests assert returned state, which is unchanged.

- [ ] **Step 4: Commit**

```bash
git add src/provider/copilot.rs
git commit -m "feat: log Copilot per-account HTTP and parse errors"
```

---

## Self-Review

**Spec coverage (card #46 scope):**
- `src/http.rs` — all `HttpError` variants → `get()` already covered by #44; `get_public` now covered (Task 2). ✓
- `src/keychain.rs` — token read failures + malformed item → Task 1. ✓ (Malformed JSON of the *credentials file* lives in `claude.rs::parse_credentials_payload`, already logged at line 207 by #44 — not a keychain concern.)
- `src/provider/claude.rs` — error paths not covered by #44 → 4 HTTP arms + profile parse, Task 3. ✓
- `src/provider/copilot.rs` — parse-error + 4 HTTP arms in `do_copilot_fetch` → Task 4. ✓ (Card listed Copilot as out-of-scope/not-written; that was stale — the provider now exists and is built, so it is now in scope per user request.)
- Codex → still out of scope (not written). ✓
- Out of scope honored: no success-path logging added, no new error variants. ✓

**Placeholder scan:** none — every step shows full code and exact commands.

**Type consistency:** `keychain_error_is_loggable(i32) -> bool` and `ERR_SEC_ITEM_NOT_FOUND: i32` referenced identically in Task 1 steps 1, 3, 5, 6. `USAGE_URL`/`PROFILE_URL` are existing module consts. `e.code()` is `security_framework::base::Error::code() -> OSStatus (i32)` (security-framework v3). All `diag!` calls use the existing `(Level, fmt, args...)` macro form.

**Testing honesty:** Only Task 1 introduces new observable logic (the not-found filter) and is TDD'd as a pure function. Tasks 2-3 are side-effect-only additions to already-tested code paths; their gate is "existing tests + clippy stay green," stated explicitly rather than fabricating racy assertions against the process-global diag ring buffer.
