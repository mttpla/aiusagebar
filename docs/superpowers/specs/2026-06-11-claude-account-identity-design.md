# Claude Account Identity Display

**Date:** 2026-06-11
**Status:** Approved

## Problem

The Claude section in the menu shows usage windows but no indication of which account is being monitored. Copilot already shows the GitHub `login` per account. Claude should do the same.

## Goal

Rename the section header from "Anthropic" to "Claude" and append account identity inline: `Claude — mttpla@gmail.com (pro)`. On failure: `Claude — account unavailable`.

## Endpoint

`GET https://api.anthropic.com/api/oauth/profile`

Same auth as `/api/oauth/usage`: `Authorization: Bearer <token>`, same `User-Agent`. Response fields used:

```json
{
  "account": {
    "email": "mttpla@gmail.com",
    "has_claude_pro": true,
    "has_claude_max": false
  }
}
```

Plan derivation: `has_claude_max → "max"`, `has_claude_pro → "pro"`, otherwise `"free"`.

## Data Structures

### New internal type in `claude.rs`

```rust
struct ProfileData {
    email: String,
    plan: String,
}
```

Not public — consumed only inside `do_fetch`, exposed as `Option<String>` via `UsageState`.

### `ClaudeProvider` gains one field

```rust
pub struct ClaudeProvider {
    last_ok: Mutex<Option<Vec<LimitWindow>>>,
    profile: Mutex<Option<ProfileData>>,   // None = unfetched or reset
}
```

### `UsageState::Ok` extended (in `provider/mod.rs`)

```rust
Ok(Vec<LimitWindow>, Option<String>)  // String = "email (plan)"
```

`None` means profile unavailable. All callers (`main.rs`, tests) updated to match the new tuple.

## Fetch Trigger Logic

`/profile` is called when `profile` lock contains `None`:

| Trigger | Action on `profile` |
|---|---|
| First `fetch()` call | `None` → fetch → populate |
| Manual refresh | Reset to `None` → fetch on next cycle |
| `/usage` returns any error | Reset to `None` → fetch on next cycle |
| `/profile` returns 401 or 403 | Leave `None`, do **not** retry (auth issue) |
| `/profile` returns 429 | Leave `None`, do **not** retry (rate limited) |
| `/profile` network/parse error | Leave `None`, retry on next trigger (transient) |

## UI Rendering

`main.rs` builds the section header from `provider.name()` + profile suffix:

| State | Header rendered |
|---|---|
| `Ok(windows, Some("mttpla@gmail.com (pro)"))` | `Claude — mttpla@gmail.com (pro)` |
| `Ok(windows, None)` | `Claude — account unavailable` |
| `NotConfigured / Stale / Error` | `Claude` (no suffix, existing behaviour) |

No separate disabled menu item for identity — it lives entirely in the section header.

## Provider Name Change

`ClaudeProvider::name()` returns `"Claude"` (was `"Anthropic"`). Affects section header and tooltip format.

## Affected Files

- `src/provider/mod.rs` — `UsageState::Ok` variant: add `Option<String>` field
- `src/provider/claude.rs` — `ProfileData` (internal), `ClaudeProvider` new field, fetch logic, `name()` rename
- `src/main.rs` — build header string from name + profile suffix, match new `Ok` tuple
- Tests in `claude.rs` — update `UsageState::Ok` matches
