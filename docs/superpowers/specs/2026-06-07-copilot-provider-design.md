# Copilot Provider Design

**Date:** 2026-06-07  
**Updated:** 2026-06-08 (live testing corrections)

## Goal

Add a `CopilotProvider` that shows GitHub Copilot quota usage for all accounts logged in via the Copilot CLI, as windows inside a single menu section.

## Approach

Single `CopilotProvider` instance. No trait change. At `fetch()`:

1. Enumerate all Keychain entries with `service = "copilot-cli"` → `Vec<(account, token)>` where account is `"https://github.com:<login>"`.
2. Deduplicate by token value.
3. If list empty → `NotConfigured`.
4. For each token: call `GET https://api.github.com/copilot_internal/user` with `Authorization: Bearer <token>` and `User-Agent: aiusagebar/0.1`.
5. Parse `login` and `quota_snapshots` from response.
6. Build `LimitWindow`s generically from snapshot keys (do NOT hardcode field names).
7. Skip snapshots where `unlimited: true` or `quota_snapshots` is null/absent.
8. Aggregate across all accounts into one `Vec<LimitWindow>`.
9. Return `Ok(windows)` if at least one account succeeded; `Stale(msg)` if all failed with 401; `Error(msg)` if all failed with other errors.

## Token Sources

**Only source:** Keychain generic-password, service `"copilot-cli"`.

- `enumerate_generic_passwords("copilot-cli")` returns all accounts (e.g. `https://github.com:matteopaoliws`, `https://github.com:matteo-paoli_adessose`).
- Each keychain read triggers a macOS permission dialog — one dialog per account on first run; "Always Allow" persists.
- Env vars (`COPILOT_GITHUB_TOKEN`, `GH_TOKEN`, `GITHUB_TOKEN`) removed: they carry PATs without Copilot scope and were causing 403.

## API

**Endpoint:** `GET https://api.github.com/copilot_internal/user`  
**Auth:** `Authorization: Bearer <token>`  
**Required:** `User-Agent: aiusagebar/0.1` — GitHub returns HTTP 403 without it.

**Response shape (verified 2026-06-08):**
```json
{
  "login": "matteo-paoli_adessose",
  "copilot_plan": "business",
  "access_type_sku": "copilot_for_business_seat_quota",
  "quota_snapshots": {
    "chat":        { "unlimited": true, ... },
    "completions": { "unlimited": true, ... },
    "premium_interactions": {
      "entitlement": 7000,
      "remaining": 6604,
      "percent_remaining": 94.3,
      "unlimited": false,
      "quota_reset_at": 0,
      "token_based_billing": true,
      ...
    }
  }
}
```

Accounts without Copilot access return `"access_type_sku": "no_access"` and `"quota_snapshots": null` — parsed as `Ok(vec![])`, no windows shown.

EMU accounts (Enterprise Managed Users) use the same token format and endpoint — no special handling required.

`percent_used = 100.0 - percent_remaining`

Window name: `"<login> / <snapshot_key>"`.

Note: `quota_reset_date_utc` is absent in business plan responses. Parse `resets_at` as `None` when missing — `LimitWindow` handles this gracefully.

## Multi-account error handling

For each token:
- 200 → normal windows (empty vec if no quota snapshots)
- 401 → sentinel `LimitWindow { name: "GitHub — token expired, re-login", all quota fields: None }`
- Other → sentinel `LimitWindow { name: "GitHub — <msg>", all quota fields: None }`

If ALL accounts return 401 → `UsageState::Stale("Copilot tokens expired — run: copilot auth login")`.  
If at least one succeeds → `UsageState::Ok(successful_windows + error_sentinel_windows)`.

No changes to `LimitWindow` struct or `UsageState` enum.

## Keychain enumeration

Function in `src/keychain.rs`:

```rust
#[cfg(target_os = "macos")]
pub fn enumerate_generic_passwords(service: &str) -> Vec<(String, String)>
```

Uses `SecItemCopyMatching` with `kSecMatchLimitAll` + `kSecReturnAttributes`. Calls `get_generic_password` per account to read password bytes (triggers one macOS dialog per item on first run).

Returns `Vec<(account, password_string)>`.

## Files changed

| File | Change |
|---|---|
| `src/keychain.rs` | Add `enumerate_generic_passwords` |
| `src/provider/copilot.rs` | `CopilotProvider` — keychain-only tokens, `User-Agent` header |
| `src/provider/mod.rs` | `pub mod copilot;` |
| `src/main.rs` | Add `CopilotProvider::new()` to providers vec |

## Constraints

- No token refresh (same as Claude — tokens are not rotated by us).
- No polling interval enforcement at this stage (background polling is a separate plan).
- `~/.copilot/config.json` used only for user discovery reference — not for token extraction (contains logins but not tokens).
- Raw data display only — no UI polish.
