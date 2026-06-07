# Copilot Provider Design

**Date:** 2026-06-07

## Goal

Add a `CopilotProvider` that shows GitHub Copilot quota usage for all accounts logged in via the Copilot CLI, as windows inside a single menu section.

## Approach

Single `CopilotProvider` instance. No trait change. At `fetch()`:

1. Enumerate all Keychain entries with `service = "copilot-cli"` → `Vec<(account, token)>` where account is `"https://github.com:<login>"`.
2. Collect env var tokens (`COPILOT_GITHUB_TOKEN` → `GH_TOKEN` → `GITHUB_TOKEN`) with login = `None` (resolved after API call).
3. Deduplicate by token value.
4. If list empty → `NotConfigured`.
5. For each `(login_hint, token)`: call `GET https://api.github.com/copilot_internal/user` with `Authorization: Bearer <token>`.
6. Parse `login` and `quota_snapshots` from response.
7. Build `LimitWindow`s generically from snapshot keys (do NOT hardcode field names — they changed from `premium_interactions` to AI Credits in June 2026).
8. Skip snapshots where `unlimited: true`.
9. Aggregate across all accounts into one `Vec<LimitWindow>`.
10. Return `Ok(windows)` if at least one account succeeded; `Stale(msg)` if all failed with 401; `Error(msg)` if all failed with other errors.

## API

**Endpoint:** `GET https://api.github.com/copilot_internal/user`
**Auth:** `Authorization: Bearer <token>`

**Response shape (as of 2026-06-07 — verify with live call at implementation time):**
```json
{
  "login": "username",
  "copilot_plan": "individual_pro",
  "quota_reset_date_utc": "2026-07-01T00:00:00.000Z",
  "quota_snapshots": {
    "<key>": {
      "entitlement": 1500,
      "remaining": 1327,
      "percent_remaining": 88.5,
      "unlimited": false
    },
    "<key2>": { "unlimited": true }
  }
}
```

Snapshot key names changed with the June 2026 AI Credits migration. Parse generically: iterate all keys, skip `unlimited: true`, build `LimitWindow` from any entry that has `percent_remaining`.

`percent_used = 100.0 - percent_remaining`

Window name: `"<login> / <snapshot_key>"` (raw, no UI polish for now).

## Multi-account error handling

For each `(login, token)`:
- 200 → normal windows
- 401 → sentinel `LimitWindow { name: "GitHub (<login>) — token scaduto, ri-logga", all quota fields: None }`
- Other → sentinel `LimitWindow { name: "GitHub (<login>) — errore: <msg>", all quota fields: None }`

If ALL accounts return 401 → `UsageState::Stale("Tutti i token Copilot scaduti — esegui: copilot auth login")`.
If at least one succeeds → `UsageState::Ok(successful_windows + error_sentinel_windows)`.

No changes to `LimitWindow` struct or `UsageState` enum.

## Keychain enumeration

New function in `src/keychain.rs`:

```rust
#[cfg(target_os = "macos")]
pub fn enumerate_generic_passwords(service: &str) -> Vec<(String, String)>
```

Uses `SecItemCopyMatching` with:
- `kSecClass = kSecClassGenericPassword`
- `kSecAttrService = service`
- `kSecMatchLimit = kSecMatchLimitAll`
- `kSecReturnAttributes = true`
- `kSecReturnData = true`

Returns `Vec<(account, password_string)>`.

## Files changed

| File | Change |
|---|---|
| `src/keychain.rs` | Add `enumerate_generic_passwords` |
| `src/provider/copilot.rs` | New file — `CopilotProvider` |
| `src/provider/mod.rs` | `pub mod copilot;` |
| `src/main.rs` | Add `CopilotProvider::new()` to providers vec |

## Constraints

- No token refresh (same as Claude — tokens are not rotated by us).
- No polling interval enforcement at this stage (background polling is a separate plan).
- `~/.copilot/config.json` and `~/.config/gh/hosts.yml` NOT used for discovery — Keychain enumeration is sufficient.
- Raw data display only — no UI polish.
