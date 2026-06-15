# Spec: Reduce keychain prompts to one per service

**Date:** 2026-06-15  
**Files:** `src/keychain.rs`

---

## Problem

`enumerate_generic_passwords` currently makes N+1 keychain accesses per service:

1. `ItemSearchOptions::search()` with `load_attributes(true)` — enumerates items and reads attributes. macOS may show a prompt here.
2. `get_generic_password(service, &account)` — reads the actual password bytes. macOS shows a prompt for each call.

For the Copilot section with 1 account: 2 prompts. With N accounts: N+1 prompts. The user sees this as 3 prompts on startup (1 Claude + 2 Copilot for a single Copilot account), which is confusing.

Note: `read_generic_password` (used by Claude) is already a single call — it is not affected.

---

## Solution

Add `.load_data(true)` to the `ItemSearchOptions` search. This sets `kSecReturnData = true` alongside `kSecReturnAttributes = true`. macOS then bundles account name + password bytes in the same result dict in a single authorized access. No separate `get_generic_password` call per account.

The password bytes appear in the result dict under the key `"v_Data"` (the CFString representation of `kSecValueData`). They are a `CFData` value and must be cast from the raw `CFTypeRef` pointer accordingly.

Expected prompt count after fix:
- Copilot with 1 account: **1 prompt** (was 2)
- Copilot with N accounts: **1 prompt** (was N+1)
- Claude: unchanged (1 prompt, already a single `read_generic_password` call)

---

## Interface

`enumerate_generic_passwords` signature is unchanged: `fn(service: &str) -> Vec<(String, String)>`. All callers are unaffected.

---

## Implementation details

- Remove `use security_framework::passwords::get_generic_password` (no longer needed in this function).
- Add `use core_foundation::data::{CFData, CFDataRef}`.
- Add `.load_data(true)` to the search options chain.
- Replace the `find_map` over keys (which only extracted `"acct"`) with a loop over key-value pairs that extracts both `"acct"` (CFString) and `"v_Data"` (CFData).
- Combine with `account.zip(password)` — if either is missing, the item is silently skipped (same behaviour as before when `get_generic_password` failed).

---

## What doesn't change

- `read_generic_password` — untouched.
- Deduplication logic in `load_copilot_tokens` — untouched.
- All call sites — untouched.
- Tests — the existing `enumerate_nonexistent_service_returns_empty` test still passes (no real keychain items for that service). No new tests needed: the behaviour is identical, only the number of macOS-level accesses changes.

---

## Risk

Low. The `"v_Data"` key is documented in Security.framework and stable since macOS 10.6. The CFData cast is the same pattern already used for CFString in the same loop. If `load_data` is unexpectedly absent from a result, `account.zip(None)` returns `None` and the item is skipped — graceful degradation, same as a failed `get_generic_password`.
