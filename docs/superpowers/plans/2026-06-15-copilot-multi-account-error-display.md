# Plan: Fix Copilot multi-account error display

**Card:** #38  
**Spec:** specs/2026-06-15-copilot-multi-account-error-display-design.md  
**Date:** 2026-06-15

---

## Task 1 — Change `load_copilot_tokens` return type

**File:** `src/provider/copilot.rs`

Change return type from `Vec<String>` to `Vec<(String, String)>`. Keep the `account` field from keychain instead of discarding it. Deduplication stays on the token value (second element).

```rust
fn load_copilot_tokens() -> Vec<(String, String)> {
    let mut seen = std::collections::HashSet::new();
    let mut tokens = Vec::new();
    for (account, password) in crate::keychain::enumerate_generic_passwords("copilot-cli") {
        if seen.insert(password.clone()) {
            tokens.push((account, password));
        }
    }
    tokens
}
```

---

## Task 2 — Update `do_copilot_fetch` signature and sentinel names

**File:** `src/provider/copilot.rs`

- Signature: `tokens: Vec<(String, String)>` (was `Vec<String>`)
- Loop over `(account, token)` tuples
- 401 sentinel: `"@{account} — token expired, re-login"`
- rate limited sentinel: `"@{account} — rate limited"`
- other error sentinel: `"@{account} — {msg}"`

---

## Task 3 — Fix `header_label` for `Ok(_, None)`

**File:** `src/ui/copilot.rs`

```rust
UsageState::Ok(_, None) => name.to_string(),
```

---

## Task 4 — Update `CopilotProvider::fetch`

**File:** `src/provider/copilot.rs`

`load_copilot_tokens()` now returns `Vec<(String, String)>` — no other change needed in `fetch`, the call site compiles automatically.

---

## Task 5 — Update tests

**File:** `src/provider/copilot.rs` tests

- All `do_copilot_fetch` call sites: change `vec!["tok".to_string()]` → `vec![("account".to_string(), "tok".to_string())]`
- `fetch_mixed_success_and_401_returns_ok_with_sentinel`: assert sentinel name contains `"@bad_account"`
- Add test: `fetch_error_sentinel_contains_account_name` — single token, Other error, assert row name starts with `"@"`

**File:** `src/ui/copilot.rs` tests

- Add test: `header_ok_no_profile_shows_name_only` — `Ok(vec![], None)` → `"GitHub Copilot"` (no " — account unavailable")

---

## Task 6 — `cargo test`

Verify all tests pass.
