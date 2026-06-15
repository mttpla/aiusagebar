# Plan: Reduce keychain prompts to one per service

**Card:** #39  
**Spec:** specs/2026-06-15-keychain-single-prompt-design.md  
**Date:** 2026-06-15

---

## Task 1 — Rewrite `enumerate_generic_passwords`

**File:** `src/keychain.rs`

- Add `.load_data(true)` to `ItemSearchOptions` chain.
- Remove `use security_framework::passwords::get_generic_password` (no longer needed here).
- Add `use core_foundation::data::{CFData, CFDataRef}`.
- Replace `find_map` (account only) with a `for` loop that extracts both `"acct"` (CFString) and `"v_Data"` (CFData) from the result dict.
- Return `account.zip(password)` — if either missing, item silently skipped.

---

## Task 2 — `cargo test`

Verify existing tests pass. No new tests needed: behaviour is identical, only macOS-level access count changes.
