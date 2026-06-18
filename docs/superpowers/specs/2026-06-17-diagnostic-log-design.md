# Diagnostic log service

**Date:** 2026-06-17

## Problem

Errors shown in the menu bar (e.g. "Parse error: invalid type: null…") give no
actionable context. There is no way for the user to report what happened or for the
developer to understand root cause without attaching a debugger.

## Goals

- Central, in-memory FIFO log usable from any module.
- No disk writes, no new dependencies.
- User can copy the full log to clipboard and paste it in an email/TextEdit.
- Not visible/intrusive when nothing is wrong.

## Non-goals

- Structured log ingestion (no `log`/`tracing` crate integration).
- Log persistence across restarts.
- In-menu entry preview (entries can be large; clipboard is enough).
- Filtering or search.

---

## `src/diag.rs` — log service

### Public API

```rust
pub enum Level { Err, Warn, Info }

pub fn push(level: Level, msg: impl Into<String>);
pub fn is_empty() -> bool;
pub fn format_all() -> String;
```

`format_all()` returns newline-separated entries, ready for clipboard.

### Macro

```rust
// In src/diag.rs (macro_rules!, exported with #[macro_export])
macro_rules! diag {
    ($lvl:expr, $($arg:tt)*) => {
        $crate::diag::push($lvl, format!("[{}:{}] {}", file!(), line!(), format!($($arg)*)))
    };
}
```

`file!()` and `line!()` are injected automatically — callers never write them manually.

Usage anywhere in the codebase:
```rust
use crate::diag::Level;
diag!(Level::Err, "Claude parse error: {}", e);
diag!(Level::Info, "Last OK: 5h {}% · 7d {}%", a, b);
// produces: [src/provider/claude.rs:87] Claude parse error: invalid type: null…
```

**At every error site, the message must be specific and actionable** — include what operation was attempted, what the error was, and any relevant values (URL, status code, account name). Vague messages like "fetch failed" are not acceptable. Prefer `inspect_err` at call sites over putting logging on error types (error types lack call-site context):

```rust
// preferred
fetch(url)
    .inspect_err(|e| diag!(Level::Err, "Claude usage fetch failed ({}): {}", url, e))

// avoid — HttpError doesn't know the URL or which provider called it
impl HttpError { fn log_diag(&self) { … } }
```

### Entry format

```
[HH:MM:SS ERR] Claude parse error: invalid type: null, expected a string at line 1 column 48
[HH:MM:SS INF] Last OK (HH:MM:SS): 5h 42% · 7d 18%
[HH:MM:SS ERR] Raw body (truncated): {"five_hour":{"utilization":0.42,"resets_at":null...
```

Timestamp via `chrono::Local::now()` (already a dependency).
Level labels: `ERR`, `WRN`, `INF`.

### Storage

```rust
static DIAG: OnceLock<Mutex<VecDeque<Entry>>> = OnceLock::new();
const CAPACITY: usize = 100;
const MAX_MSG_BYTES: usize = 2048;
```

`push` trims `message` to `MAX_MSG_BYTES` appending `… (truncated)` if needed, then
pops the oldest entry when `len() == CAPACITY`.

RAM: typical ~20 KB, worst case ~200 KB (100 × 2 KB).

---

## Hook points (v1)

Three call sites, all in the Claude provider:

### 1. Parse error (`src/provider/claude.rs` — `parse_response`)

```rust
Err(e) => {
    diag!(Level::Err, "Claude parse error: {}", e);
    // inject last-ok snapshot immediately after
    if let Some(summary) = last_ok_summary() {
        diag!(Level::Info, "Last OK: {}", summary);
    }
    diag!(Level::Err, "Raw body: {}", truncate(body, 2048));
    Err(e.to_string())
}
```

`last_ok_summary()` reads from `ClaudeProvider.last_ok` (already cached) and formats
it as `"5h 42% · 7d 18% (reset 03:00)"`.

### 2. HTTP error (`src/http.rs` — `get`)

```rust
Err(HttpError::Status(status, body)) => {
    diag!(Level::Err, "HTTP {} from {}: {}", status, url, truncate(&body, 512));
}
```

### 3. Token load failure (`src/provider/claude.rs` — `do_fetch`)

```rust
CredLoad::Malformed(e) => {
    diag!(Level::Err, "Claude credentials malformed: {}", e);
}
```

---

## Menu integration

### Structure

```
Other ▶                        ← always visible (future home for Settings etc.)
  Diagnostics ▶                ← hidden when diag::is_empty()
    Copy diagnostic log        ← NSMenuItem, copies diag::format_all() to NSPasteboard
```

### NSPasteboard

Add `NSPasteboard` to `objc2-app-kit` features in `Cargo.toml`.

Copy action:
```rust
let pb = NSPasteboard::generalPasteboard();
pb.clearContents();
pb.setString_forType(&NSString::from_str(&diag::format_all()), NSPasteboardTypeString);
```

### Rebuild menu

Menu is rebuilt on every poll cycle (existing pattern). `is_empty()` check on each
rebuild controls Diagnostics visibility. No extra state needed.

---

## README — Troubleshooting section

The README must include a **Troubleshooting** section explaining:

- When something is wrong, the menu shows an error message in the affected provider row.
- Open **Other ▶ Diagnostics ▶ Copy diagnostic log** to copy the full log to clipboard.
- The Diagnostics submenu is hidden when there is nothing to report.
- Paste the log into an email or GitHub issue when reporting a bug.

Example copy in the README:
```
### Troubleshooting

If a provider row shows an error, open **Other ▶ Diagnostics ▶ Copy diagnostic log**
to copy the full diagnostic log to your clipboard. Paste it into a GitHub issue or email
when reporting a bug. The submenu is hidden when there is nothing to report.
```

---

## Files changed

| File | Change |
|------|--------|
| `src/diag.rs` | New module — entire log service |
| `src/main.rs` | Register `mod diag`; add "Other" + "Diagnostics" submenus |
| `src/provider/claude.rs` | Add 3 `diag!` call sites; add `last_ok_summary()` helper |
| `src/http.rs` | Add `diag!` call site on HTTP error |
| `Cargo.toml` | Add `NSPasteboard` to `objc2-app-kit` features |

## Rejected options

- **`log_buffer` / `memory_logger` crates**: integrate with `log` facade, count bytes
  not lines, add dependency for no gain over a 30-line `VecDeque` wrapper.
- **In-menu entry preview**: entries can be multi-line and large; NSMenuItem text
  items are not scrollable and would bloat the menu height unpredictably.
- **Disk log file**: adds I/O, path management, rotation logic. Overkill for
  occasional error reporting.
- **Always-visible Diagnostics item**: clutters menu when there is nothing to report.
