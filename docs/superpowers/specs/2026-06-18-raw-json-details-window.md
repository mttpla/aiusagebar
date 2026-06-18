# Raw JSON Details Window

## Overview

Add a "Details…" menu item to each provider section. Clicking opens a macOS window showing the last raw HTTP response body from that provider's API — full fidelity, no parsing loss, including error bodies.

## Goals

- Inspect full API response as received
- See error bodies (4xx) for debugging (expired token, rate limit, etc.)
- Tech/nerd view; no formatting beautification required for v1

## Non-Goals

- Key-value or tree-formatted display (future enhancement)
- Response history — only the last response per provider
- Modifying or copying credentials

---

## Architecture

### 1. `src/http.rs` — Capture response body on all status codes

Change `http::get` signature:

```rust
pub fn get(
    url: &str,
    token: &str,
    extra_headers: &[(&str, &str)],
) -> (Result<String, HttpError>, Option<String>)
```

- First: `Ok(body)` on 200; `Err(HttpError::...)` on non-200 or network error
- Second: raw response body whenever the server responded (any status); `None` on network/IO error only

`get_public` is unchanged — no raw-body caching needed there.

All callers of `http::get` (provider closures in `do_fetch` and `do_copilot_fetch`) must be updated to destructure the new tuple.

### 2. `src/provider/mod.rs` — Add `raw_json` to trait

```rust
pub trait UsageProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn fetch_with_http_error(&self) -> (UsageState, Option<crate::http::HttpError>);
    fn raw_json(&self) -> Option<String>;
}
```

`raw_json()` returns the last cached body string, or `None` if the provider has never completed a fetch.

### 3. `src/provider/claude.rs` — Cache raw usage body

Add field to `ClaudeProvider`:

```rust
pub struct ClaudeProvider {
    last_ok: Mutex<Option<Vec<LimitWindow>>>,
    profile: Mutex<Option<ProfileData>>,
    last_raw_json: Mutex<Option<String>>,  // last usage endpoint body (any status)
}
```

`do_fetch` receives the raw body from the `http` closure (the second element of the new tuple) and stores it in `last_raw_json` on every call where the server responded. The profile endpoint body is not cached — usage body is the primary data.

`raw_json()` returns `last_raw_json.lock().unwrap().clone()`.

### 4. `src/provider/copilot.rs` — Cache raw body, concatenated per account

Add field to `CopilotProvider`:

```rust
pub struct CopilotProvider {
    last_raw_json: Mutex<Option<String>>,
}
```

`do_copilot_fetch` iterates accounts. For each account, appends to a local buffer:

```
--- @account1 ---
{"login":"account1",...}
--- @account2 ---
{"message":"Bad credentials"}
```

Includes both successful (200) and error bodies. Written atomically to `last_raw_json` at end of fetch loop, overwriting the previous value.

`raw_json()` returns `last_raw_json.lock().unwrap().clone()`.

### 5. `src/details.rs` — New module

```rust
pub fn show(provider_name: &str, raw_json: Option<&str>)
```

macOS only (`#[cfg(target_os = "macos")]`). Uses `NSAlert` with `NSScrollView` (600 × 300 pt) wrapping `NSTextView` as accessory view. `NSTextView` is non-editable, monospace font (`NSFont::monospacedSystemFont`), left-aligned.

Content preparation (pure function, testable):

```rust
pub fn prepare_content(raw_json: Option<&str>) -> String
```

- `None` → `"No data yet"`
- `Some(body)` → try `serde_json::from_str::<serde_json::Value>` + `to_string_pretty`; on failure show `body` as-is

Window title via `setMessageText`: `"Details — {provider_name}"`.

Single "OK" button closes the alert.

### 6. UI — "Details…" menu item per provider section

In `src/ui/claude.rs` (`append_claude_section`) and `src/ui/copilot.rs` (`append_copilot_section`):
- Append a clickable `MenuItem::new("Details…", true, None)` as last item of the section
- Return its `MenuId` alongside any existing returned IDs

`section_item_count` += 1 for every state variant (Details item is always present).

`MenuBuild` (in `src/ui/mod.rs`) gains:

```rust
pub details_claude: Option<MenuId>,
pub details_copilot: Option<MenuId>,
```

### 7. `src/main.rs` — Wire Details click

`App` gains:

```rust
id_details_claude: Option<tray_icon::menu::MenuId>,
id_details_copilot: Option<tray_icon::menu::MenuId>,
```

In `about_to_wait` event handler:

```rust
} else if self.id_details_claude.as_ref().is_some_and(|id| ev.id == *id) {
    let raw = self.providers.iter()
        .find(|p| p.kind() == ProviderKind::Claude)
        .and_then(|p| p.raw_json());
    details::show("Claude", raw.as_deref());
} else if self.id_details_copilot.as_ref().is_some_and(|id| ev.id == *id) {
    let raw = self.providers.iter()
        .find(|p| p.kind() == ProviderKind::Copilot)
        .and_then(|p| p.raw_json());
    details::show("Copilot", raw.as_deref());
}
```

`raw_json()` is read at click time — no extra cache in `App`.

---

## Testing

- `http.rs`: `get` returns body alongside error on non-200 (mock server or unit test with `ureq` mock)
- `provider/claude.rs`: `raw_json()` is `None` before any fetch; `Some` after 200; `Some` after 401 (if server returned body)
- `provider/copilot.rs`: multi-account concatenation correct; `None` before fetch
- `details.rs`: `prepare_content(None)` = `"No data yet"`; valid JSON pretty-prints; invalid input returns raw string
- Manual: click "Details…" in menu bar while Claude is authenticated → window opens with formatted JSON

---

## Files Changed

| File | Change |
|------|--------|
| `src/http.rs` | `get` returns `(Result<String, HttpError>, Option<String>)` |
| `src/provider/mod.rs` | add `raw_json()` to trait |
| `src/provider/claude.rs` | add `last_raw_json` field; update `do_fetch` |
| `src/provider/copilot.rs` | add `last_raw_json` field; update `do_copilot_fetch` |
| `src/details.rs` | new module: `show`, `prepare_content` |
| `src/main.rs` | new ID fields; new event handlers; `mod details` |
| `src/ui/claude.rs` | add Details item; update `section_item_count` |
| `src/ui/copilot.rs` | add Details item; update `section_item_count` |
| `src/ui/mod.rs` | add `details_claude`, `details_copilot` to `MenuBuild` |
