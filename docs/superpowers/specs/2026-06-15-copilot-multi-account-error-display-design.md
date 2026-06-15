# Spec: Fix Copilot multi-account error display

**Card:** #38  
**Date:** 2026-06-15  
**Files:** `src/provider/copilot.rs`, `src/ui/copilot.rs`

---

## Problem

### Bug 1 — Header shows "account unavailable" when accounts are healthy

`header_label` in `ui/copilot.rs:8` maps `UsageState::Ok(_, None)` → `"{name} — account unavailable"`. This pattern was designed for the Claude provider, where `None` in the second field means the profile endpoint failed. For Copilot, `do_copilot_fetch` always returns `UsageState::Ok(windows, None)` — `None` is the normal state because there is no single profile string to show (multi-account, logins are embedded in window names). Result: the header permanently reads `"GitHub Copilot — account unavailable"` even when everything works.

### Bug 2 — Error rows don't identify the failing account

When a Copilot token returns 401, `do_copilot_fetch` pushes a sentinel `LimitWindow` with name `"GitHub — token expired, re-login"`. But `load_copilot_tokens()` discards the keychain `account` field (`_account`), so there is no identity attached to the failure. If a user has two Copilot accounts and one is expired, they cannot tell which one to re-login.

---

## Design

### 1. Carry keychain account name through the fetch pipeline

Change `load_copilot_tokens` return type from `Vec<String>` to `Vec<(String, String)>` where the tuple is `(keychain_account_name, token)`.

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

Change `do_copilot_fetch` signature to accept `Vec<(String, String)>`. Use the account name in sentinel `LimitWindow` entries:

- 401: `"@{account} — token expired, re-login"`
- rate limited: `"@{account} — rate limited"`
- other error: `"@{account} — {error}"`

### 2. Fix the header for the Ok+None case

Change `header_label` in `ui/copilot.rs` so that `Ok(_, None)` renders just the provider name with no suffix:

```rust
UsageState::Ok(_, None) => name.to_string(),
```

This is correct for Copilot (no single profile to display) and has no effect on Claude because the Claude provider always populates the second field when the fetch succeeds.

---

## Behaviour after fix

| Scenario | Header | Rows |
|---|---|---|
| 1 account, OK | `GitHub Copilot` | `mttpla / premium_interactions — 5.7%  resets ...` |
| 2 accounts, both OK | `GitHub Copilot` | one row per account per window |
| 2 accounts, one 401 | `GitHub Copilot` | OK rows + `@alice — token expired, re-login` |
| 2 accounts, both 401 | `GitHub Copilot ⚠  Copilot tokens expired...` | (Stale state, no rows) |
| 1 account, other error | `GitHub Copilot ✕  {error}` | (Error state, no rows) |

---

## Test changes

- `do_copilot_fetch` tests: update all call sites to pass `Vec<(String, String)>` tuples.
- `fetch_mixed_success_and_401_returns_ok_with_sentinel`: assert sentinel name contains `@bad_account`.
- `header_label` unit test: assert `Ok(_, None)` → just `"GitHub Copilot"` (no "account unavailable").

---

## Out of scope

- No change to `UsageState` enum — the `Option<String>` second field remains.
- No change to Claude provider or `ui/claude.rs`.
- `header_label` for the Claude provider already works correctly — `Ok(_, None)` there is an actual error state.
