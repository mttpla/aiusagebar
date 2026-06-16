# Auto-update check via GitHub Releases

**Date:** 2026-06-16
**Card:** #20

## Goal

Once per 24 hours, check whether a newer release exists on GitHub and surface it in the tray menu as a clickable row that opens the release page. No automatic download, no Sparkle framework.

## Architecture

### New: `src/update_check.rs`

Single public function:

```rust
pub fn check() -> Option<String>
```

Steps:
1. Call `http::get_public("https://api.github.com/repos/mttpla/aiusagebar/releases/latest")`.
2. On any error (network, HTTP non-200 including 404), return `None` silently.
3. Parse JSON: extract `tag_name: String` and `assets: Vec<_>`.
4. If `assets` is empty, return `None` — no downloadable binary, nothing to offer.
5. Strip leading `v` from `tag_name`. If parse fails, return `None`.
6. Compare remote version to `env!("CARGO_PKG_VERSION")` via `is_newer(current, remote)`.
7. Return `Some(remote_version)` if newer, `None` otherwise.

#### `is_newer(current: &str, remote: &str) -> bool`

Pure function (pub for tests). Parses both strings as `X.Y.Z` tuples `(u32, u32, u32)`. Returns `false` on any parse failure. Compares lexicographically (major → minor → patch). No new crate dependency — our version format is controlled and always semver.

**Test matrix:**

| current | remote | expected |
|---------|--------|----------|
| `0.3.2` | `0.4.0` | `true` |
| `0.4.0` | `0.4.0` | `false` (same) |
| `0.5.0` | `0.4.0` | `false` (current newer) |
| `0.3.2` | `0.3.3` | `true` (patch bump) |
| `1.0.0` | `2.0.0` | `true` (major bump) |
| `0.3.2` | `not-a-version` | `false` (malformed) |
| `0.3.2` | `` | `false` (empty) |

**`check()` test cases:**

| scenario | expected |
|----------|----------|
| 200, newer tag, non-empty assets | `Some("0.4.0")` |
| 200, same tag, non-empty assets | `None` |
| 200, newer tag, empty assets | `None` |
| 404 (no releases) | `None` |
| network error | `None` |
| malformed JSON | `None` |

### `src/http.rs` — new function

```rust
pub fn get_public(url: &str) -> Result<String, HttpError>
```

Same as `get()` but without `Authorization` header. GitHub releases endpoint is public. Reuses the existing shared `agent()`.

### `src/main.rs` — `App` struct

New fields:
- `next_update_check_after: DateTime<Local>` — initialized to `Local::now() + chrono::Duration::hours(24)`. Wall-clock so the 24h window advances during sleep.
- `update_available: Option<String>` — initialized to `None`
- `id_update: Option<tray_icon::menu::MenuId>` — `None` when no update row is shown

In `about_to_wait`:
```
if Local::now() >= next_update_check_after {
    update_available = update_check::check();
    next_update_check_after = Local::now() + chrono::Duration::hours(24);
}
```

`chrono` is already a dependency and `DateTime<Local>` is already used in `App`.

After any state change that triggers a menu rebuild, pass `update_available.as_deref()` to `ui::build_menu`.

Event handling: if `Some(id) = id_update && ev.id == id`, run:
```rust
let _ = std::process::Command::new("open")
    .arg("https://github.com/mttpla/aiusagebar/releases/latest")
    .spawn();
```

### `src/ui/mod.rs` — `build_menu` signature change

```rust
pub fn build_menu(
    states: &[(ProviderKind, &UsageState)],
    last_updated: Option<&str>,
    update: Option<&str>,        // new
) -> MenuBuild
```

`MenuBuild` gains:
```rust
pub update: Option<MenuId>,
```

If `update = Some(version)`, prepend to menu:
1. `MenuItem` (enabled, clickable): `↑ Update available {version}`
2. `PredefinedMenuItem::separator()`

If `update = None`, nothing prepended. Menu structure is otherwise unchanged.

`build_layout` is index-tracking only and does not need to change (it does not model the update row).

## Out of scope

- Persistent last-check timestamp across restarts — check resets to 24h on every launch.
- Per-asset architecture filtering (arm64 vs x86) — single binary, implicit.
- Dismissing the update row — it reappears every refresh until the app is updated.
- Rate-limit handling beyond silent `None` return.
