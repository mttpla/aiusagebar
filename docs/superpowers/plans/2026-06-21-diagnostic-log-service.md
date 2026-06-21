# Diagnostic Log Service Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a central in-memory diagnostic log that any module can write to, surfaced in the menu under "Other ▶ Diagnostics ▶ Copy diagnostic log" so users can copy errors to the clipboard for bug reports.

**Architecture:** A platform-neutral `diag` module wraps a `VecDeque<Entry>` behind a `OnceLock<Mutex<…>>`, exposing `push`/`is_empty`/`format_all` plus a `diag!` macro that auto-injects `file!():line!()`. Error sites in the Claude provider and the HTTP layer call `diag!`. A macOS `clipboard` module copies `format_all()` via `NSPasteboard`. The menu gains an always-visible "Other" submenu wired through the existing rebuild path.

**Tech Stack:** Rust, `tray-icon` 0.19 (menu/submenu), `objc2-app-kit` (NSPasteboard), `chrono` (timestamps), `ureq` (existing HTTP).

## Global Constraints

- HTTP layer is `ureq`, **not** `reqwest`. `HttpError` variants are exactly `Unauthorized | RateLimited | ServerError(u16) | Other(String)`. `get()` returns `(Result<String, HttpError>, Option<String>)`.
- No new crate dependencies. Only enable the `NSPasteboard` feature on the existing `objc2-app-kit`.
- All `.rs` string literals must be English (Italian only in runtime i18n, which does not exist yet).
- No `#[allow(dead_code)]`. Delete unused symbols instead.
- No `Co-Authored-By` trailer in commit messages.
- Run `cargo clippy -- -D warnings && cargo test` before every commit; all existing tests must stay green.
- Tokens are read-only; this plan touches no auth-writing code.
- Diagnostic buffer: capacity 100 entries, 2048-byte cap per message, no disk writes.

---

### Task 1: `diag` log service module

**Files:**
- Create: `src/diag.rs`
- Modify: `src/main.rs:1-12` (add `mod diag;`)
- Test: inline `#[cfg(test)] mod tests` in `src/diag.rs`

**Interfaces:**
- Consumes: `chrono::Local` (already a dependency).
- Produces:
  - `pub enum Level { Err, Warn, Info }`
  - `pub fn push(level: Level, msg: impl Into<String>)`
  - `pub fn is_empty() -> bool`
  - `pub fn format_all() -> String`
  - `pub fn truncate(s: &str, max_bytes: usize) -> String`
  - `#[macro_export] macro_rules! diag { ($lvl:expr, $($arg:tt)*) => { … } }` — callable crate-wide as `crate::diag!(Level::Err, "…", arg)`.

- [ ] **Step 1: Write the failing tests**

Create `src/diag.rs` with only the tests first (the module items come in Step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 2048), "hello");
    }

    #[test]
    fn truncate_long_string_appends_marker() {
        let s = "a".repeat(3000);
        let out = truncate(&s, 2048);
        assert!(out.ends_with("… (truncated)"), "got tail: {:?}", &out[out.len().saturating_sub(20)..]);
        assert!(out.len() <= 2048 + "… (truncated)".len());
    }

    #[test]
    fn truncate_respects_char_boundary() {
        // 'é' is 2 bytes; cutting at an odd byte must not panic and must stay valid UTF-8.
        let s = "é".repeat(2000); // 4000 bytes
        let out = truncate(&s, 2049); // odd boundary inside a char
        assert!(out.is_char_boundary(out.len()));
        assert!(out.ends_with("… (truncated)"));
    }

    #[test]
    fn format_entry_has_time_level_and_message() {
        let line = format_entry("12:34:56", Level::Err, "boom");
        assert_eq!(line, "[12:34:56 ERR] boom");
    }

    #[test]
    fn format_entry_warn_and_info_labels() {
        assert!(format_entry("00:00:00", Level::Warn, "x").contains("WRN"));
        assert!(format_entry("00:00:00", Level::Info, "x").contains("INF"));
    }

    // Single test that mutates the global buffer, to avoid cross-test races.
    #[test]
    fn push_is_empty_capacity_and_format_all() {
        assert!(is_empty(), "buffer must start empty in a fresh test process");
        push(Level::Err, "first");
        assert!(!is_empty());
        let all = format_all();
        assert!(all.contains("first"), "got: {all}");
        assert!(all.contains("ERR"), "got: {all}");

        // Overflow capacity: push 120 more, oldest must be evicted.
        for i in 0..120 {
            push(Level::Info, format!("entry {i}"));
        }
        let all = format_all();
        let line_count = all.lines().count();
        assert_eq!(line_count, 100, "buffer must cap at 100 lines, got {line_count}");
        assert!(!all.contains("first"), "oldest entry must be evicted");
        assert!(all.contains("entry 119"), "newest entry must be present");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib diag`
Expected: FAIL — `truncate`, `format_entry`, `Level`, `push`, `is_empty`, `format_all` not found.

- [ ] **Step 3: Write the module implementation**

Prepend to `src/diag.rs` (above the test module):

```rust
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

const CAPACITY: usize = 100;
const MAX_MSG_BYTES: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Err,
    Warn,
    Info,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Err => "ERR",
            Level::Warn => "WRN",
            Level::Info => "INF",
        }
    }
}

#[derive(Debug, Clone)]
struct Entry {
    time: String,
    level: Level,
    message: String,
}

static DIAG: OnceLock<Mutex<VecDeque<Entry>>> = OnceLock::new();

fn buffer() -> &'static Mutex<VecDeque<Entry>> {
    DIAG.get_or_init(|| Mutex::new(VecDeque::with_capacity(CAPACITY)))
}

/// Truncates `s` to at most `max_bytes`, respecting char boundaries, appending
/// "… (truncated)" when truncation occurs.
pub fn truncate(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… (truncated)", &s[..end])
}

fn format_entry(time: &str, level: Level, message: &str) -> String {
    format!("[{} {}] {}", time, level.label(), message)
}

pub fn push(level: Level, msg: impl Into<String>) {
    let message = truncate(&msg.into(), MAX_MSG_BYTES);
    let time = chrono::Local::now().format("%H:%M:%S").to_string();
    let entry = Entry { time, level, message };
    let mut buf = buffer().lock().unwrap();
    if buf.len() == CAPACITY {
        buf.pop_front();
    }
    buf.push_back(entry);
}

pub fn is_empty() -> bool {
    buffer().lock().unwrap().is_empty()
}

pub fn format_all() -> String {
    let buf = buffer().lock().unwrap();
    buf.iter()
        .map(|e| format_entry(&e.time, e.level, &e.message))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Pushes a diagnostic entry, auto-injecting the call-site `file!():line!()`.
/// Usage: `crate::diag!(crate::diag::Level::Err, "fetch failed: {}", e);`
#[macro_export]
macro_rules! diag {
    ($lvl:expr, $($arg:tt)*) => {
        $crate::diag::push($lvl, format!("[{}:{}] {}", file!(), line!(), format!($($arg)*)))
    };
}
```

- [ ] **Step 4: Register the module**

In `src/main.rs`, add after line 1 (`mod backoff;`):

```rust
mod diag;
```

(`#[macro_export]` hoists `diag!` to the crate root regardless of module order.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib diag`
Expected: PASS (6 tests).

- [ ] **Step 6: Lint and full test suite**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: PASS, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/diag.rs src/main.rs
git commit -m "feat: add in-memory diagnostic log service"
```

---

### Task 2: macOS clipboard helper

**Files:**
- Create: `src/clipboard.rs`
- Modify: `Cargo.toml:20-25` (add `NSPasteboard` feature), `src/main.rs` (add `mod clipboard;`)
- Test: inline `#[cfg(test)] mod tests` in `src/clipboard.rs`

**Interfaces:**
- Consumes: `objc2-app-kit` `NSPasteboard`, `objc2-foundation` `NSString`.
- Produces: `pub fn copy(text: &str)` — copies `text` to the general pasteboard (no-op off macOS).

- [ ] **Step 1: Add the Cargo feature**

In `Cargo.toml`, change the `objc2-app-kit` features list (lines 20-25) to add `"NSPasteboard"`:

```toml
objc2-app-kit = { version = "0.3", features = [
    "NSAlert", "NSTextField", "NSControl", "NSView", "NSText",
    "NSColor", "NSFont", "NSMenu", "NSMenuItem",
    "NSParagraphStyle", "NSAttributedString", "NSImage",
    "NSBox", "NSScrollView", "NSTextView", "NSPasteboard",
] }
```

- [ ] **Step 2: Write the failing test**

Create `src/clipboard.rs`:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn copy_has_expected_signature() {
        let _: fn(&str) = super::copy;
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --lib clipboard`
Expected: FAIL — `copy` not found.

- [ ] **Step 4: Write the implementation**

Prepend to `src/clipboard.rs`:

```rust
//! Clipboard helper.

/// Copies `text` to the macOS general pasteboard, replacing its contents.
#[cfg(target_os = "macos")]
pub fn copy(text: &str) {
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    use objc2_foundation::NSString;
    unsafe {
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        let ns = NSString::from_str(text);
        pb.setString_forType(&ns, NSPasteboardTypeString);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn copy(_text: &str) {}
```

- [ ] **Step 5: Register the module**

In `src/main.rs`, add after the `mod clipboard;` insertion point (next to the other `mod` lines, e.g. after `mod backoff;`):

```rust
mod clipboard;
```

- [ ] **Step 6: Build to verify the objc2 bindings compile**

Run: `cargo build`
Expected: PASS. If the compiler reports a signature mismatch on `generalPasteboard`, `clearContents`, or `setString_forType` (objc2 0.3 occasionally differs on `unsafe`/return types), adjust the call to match the exact binding the error names — keep the same three operations (get general pasteboard, clear, set string for `NSPasteboardTypeString`).

- [ ] **Step 7: Lint and test**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml src/clipboard.rs src/main.rs
git commit -m "feat: add macOS clipboard copy helper"
```

---

### Task 3: Wire diagnostic hook points

**Files:**
- Modify: `src/provider/claude.rs:139-159` (add `last_ok_summary` helper), `src/provider/claude.rs:193-230` (`do_fetch` malformed + parse-error arms), `src/http.rs:36-49` (`get` error logging)
- Test: inline tests in `src/provider/claude.rs`

**Interfaces:**
- Consumes: `crate::diag!` macro, `crate::diag::Level`, `crate::diag::truncate` (Task 1); `LimitWindow` (existing).
- Produces: `fn last_ok_summary(windows: &[LimitWindow]) -> String` (module-private helper in `claude.rs`).

- [ ] **Step 1: Write the failing test for `last_ok_summary`**

Add to the `tests` module in `src/provider/claude.rs`:

```rust
#[test]
fn last_ok_summary_formats_each_window() {
    let windows = vec![
        LimitWindow { name: "5h session".to_string(), percent_used: Some(42.0), ..Default::default() },
        LimitWindow { name: "7d weekly".to_string(), percent_used: Some(18.0), ..Default::default() },
    ];
    assert_eq!(super::last_ok_summary(&windows), "5h session 42% · 7d weekly 18%");
}

#[test]
fn last_ok_summary_handles_missing_percent() {
    let windows = vec![
        LimitWindow { name: "5h session".to_string(), percent_used: None, ..Default::default() },
    ];
    assert_eq!(super::last_ok_summary(&windows), "5h session —");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib last_ok_summary`
Expected: FAIL — `last_ok_summary` not found.

- [ ] **Step 3: Add the `last_ok_summary` helper**

Insert in `src/provider/claude.rs` immediately after `parse_response` (after line 159):

```rust
fn last_ok_summary(windows: &[LimitWindow]) -> String {
    windows
        .iter()
        .map(|w| match w.percent_used {
            Some(p) => format!("{} {:.0}%", w.name, p),
            None => format!("{} —", w.name),
        })
        .collect::<Vec<_>>()
        .join(" · ")
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib last_ok_summary`
Expected: PASS.

- [ ] **Step 5: Add the malformed-credentials hook**

In `src/provider/claude.rs`, change the `CredLoad::Malformed` arm in `do_fetch` (line 195) from:

```rust
        CredLoad::Malformed(e) => return (UsageState::Error(format!("Malformed credentials: {}", e)), None),
```

to:

```rust
        CredLoad::Malformed(e) => {
            crate::diag!(crate::diag::Level::Err, "Claude credentials malformed: {}", e);
            return (UsageState::Error(format!("Malformed credentials: {}", e)), None);
        }
```

- [ ] **Step 6: Add the parse-error hook**

In the same function, change the parse-error arm (lines 207-214) from:

```rust
        Ok(body) => match parse_response(&body) {
            Ok(windows) => {
                let windows = windows.to_vec();
                *last_ok.lock().unwrap() = Some(windows.clone());
                (UsageState::Ok(windows, profile_string), None)
            }
            Err(e) => (UsageState::Error(format!("Parse error: {}", e)), None),
        },
```

to:

```rust
        Ok(body) => match parse_response(&body) {
            Ok(windows) => {
                let windows = windows.to_vec();
                *last_ok.lock().unwrap() = Some(windows.clone());
                (UsageState::Ok(windows, profile_string), None)
            }
            Err(e) => {
                crate::diag!(crate::diag::Level::Err, "Claude parse error: {}", e);
                if let Some(windows) = last_ok.lock().unwrap().as_ref() {
                    crate::diag!(crate::diag::Level::Info, "Last OK: {}", last_ok_summary(windows));
                }
                crate::diag!(crate::diag::Level::Err, "Raw body: {}", crate::diag::truncate(&body, 2048));
                (UsageState::Error(format!("Parse error: {}", e)), None)
            }
        },
```

- [ ] **Step 7: Add the HTTP hook in `get`**

In `src/http.rs`, change the early network-error return (lines 36-39) from:

```rust
    let resp = match req.call() {
        Ok(r) => r,
        Err(e) => return (Err(HttpError::Other(e.to_string())), None),
    };
```

to:

```rust
    let resp = match req.call() {
        Ok(r) => r,
        Err(e) => {
            crate::diag!(crate::diag::Level::Err, "HTTP request to {} failed: {}", url, e);
            return (Err(HttpError::Other(e.to_string())), None);
        }
    };
```

Then change the result block (lines 42-49) from:

```rust
    let result = match status {
        200 => raw.clone().map(Ok).unwrap_or_else(|| Err(HttpError::Other("body read error".into()))),
        401 => Err(HttpError::Unauthorized),
        429 => Err(HttpError::RateLimited),
        c @ 500..=599 => Err(HttpError::ServerError(c)),
        code => Err(HttpError::Other(format!("HTTP {}", code))),
    };
    (result, raw)
```

to:

```rust
    let result = match status {
        200 => raw.clone().map(Ok).unwrap_or_else(|| Err(HttpError::Other("body read error".into()))),
        401 => Err(HttpError::Unauthorized),
        429 => Err(HttpError::RateLimited),
        c @ 500..=599 => Err(HttpError::ServerError(c)),
        code => Err(HttpError::Other(format!("HTTP {}", code))),
    };
    if result.is_err() {
        crate::diag!(
            crate::diag::Level::Err,
            "HTTP {} from {}: {}",
            status,
            url,
            crate::diag::truncate(raw.as_deref().unwrap_or(""), 512)
        );
    }
    (result, raw)
```

- [ ] **Step 8: Lint and full test suite**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: PASS, no warnings. (Existing `do_fetch`/`get` tests still pass; they now also push to the global diag buffer as a harmless side effect.)

- [ ] **Step 9: Commit**

```bash
git add src/provider/claude.rs src/http.rs
git commit -m "feat: log parse, credential, and HTTP errors to diagnostic log"
```

---

### Task 4: "Other ▶ Diagnostics" menu integration

**Files:**
- Modify: `src/ui/base.rs` (add `append_other`), `src/ui/mod.rs:12-22` (`MenuBuild` field), `src/ui/mod.rs:39-64` (`build_layout` offset), `src/ui/mod.rs:88-127` (`build_menu`), `src/ui/mod.rs:129-235` (`build_layout` tests), `src/main.rs` (`App` field + wiring + click handler)
- Test: inline `build_layout` tests in `src/ui/mod.rs`

**Interfaces:**
- Consumes: `crate::diag::is_empty` (Task 1), `crate::diag::format_all` (Task 1), `crate::clipboard::copy` (Task 2).
- Produces:
  - `base::append_other(menu: &Menu) -> Option<MenuId>` — appends the always-visible "Other" submenu; returns the "Copy diagnostic log" item id when the log is non-empty, else `None`.
  - `MenuBuild.copy_diag: Option<MenuId>`.

- [ ] **Step 1: Update the `build_layout` tests for the +1 Other offset**

The "Other" submenu is always one top-level item inserted before the footer, so every footer index shifts by 1. In `src/ui/mod.rs`, update these test assertions:

`menu_layout_indices_no_providers`:
```rust
        assert_eq!(layout.refresh_idx, 1);
        assert_eq!(layout.quit_idx, 4);
```

`menu_layout_indices_claude_two_windows`:
```rust
        assert_eq!(layout.refresh_idx, 5);
        assert_eq!(layout.quit_idx, 8);
```

`build_layout_copilot_window_items_indices`:
```rust
        assert_eq!(layout.refresh_idx, 8);
        assert_eq!(layout.quit_idx, 11);
```

`build_layout_with_update_shifts_all_indices_by_2`:
```rust
        // refresh was at 3 without Other/update; +1 Other +2 update = 6
        assert_eq!(layout.refresh_idx, 6);
        assert_eq!(layout.quit_idx, 9);
```

`build_layout_without_update_unchanged`:
```rust
        assert_eq!(layout.refresh_idx, 4);
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib build_layout`
Expected: FAIL — assertions mismatch (still using pre-offset values).

- [ ] **Step 3: Apply the offset in `build_layout`**

In `src/ui/mod.rs`, change the `MenuLayout` construction (lines 56-63) from:

```rust
    // Footer layout: Refresh(idx), separator(idx+1), About(idx+2), Quit(idx+3)
    MenuLayout {
        header_indices,
        window_items,
        refresh_idx: idx,
        quit_idx: idx + 3,
        last_updated: last_updated.map(str::to_owned),
    }
```

to:

```rust
    // "Other" submenu (always present) sits at `idx`; the footer follows it.
    // Footer: Refresh(idx+1), separator(idx+2), About(idx+3), Quit(idx+4)
    MenuLayout {
        header_indices,
        window_items,
        refresh_idx: idx + 1,
        quit_idx: idx + 4,
        last_updated: last_updated.map(str::to_owned),
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib build_layout`
Expected: PASS.

- [ ] **Step 5: Add `append_other` to `base.rs`**

In `src/ui/base.rs`, change the import (line 1) to include `Submenu`:

```rust
use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};
```

Then add this function below `append_footer`:

```rust
/// Appends the always-visible "Other ▶" submenu. When the diagnostic log has
/// entries it contains "Diagnostics ▶ Copy diagnostic log" and returns the copy
/// item's id; when empty it shows a disabled "No diagnostics" placeholder and
/// returns None.
pub(crate) fn append_other(menu: &Menu) -> Option<MenuId> {
    let other = Submenu::new("Other", true);
    let copy_id = if crate::diag::is_empty() {
        let placeholder = MenuItem::new("No diagnostics", false, None);
        other.append(&placeholder).expect("menu append failed");
        None
    } else {
        let diagnostics = Submenu::new("Diagnostics", true);
        let copy = MenuItem::new("Copy diagnostic log", true, None);
        let id = copy.id().clone();
        diagnostics.append(&copy).expect("menu append failed");
        other.append(&diagnostics).expect("menu append failed");
        Some(id)
    };
    menu.append(&other).expect("menu append failed");
    copy_id
}
```

- [ ] **Step 6: Add the `copy_diag` field to `MenuBuild`**

In `src/ui/mod.rs`, add to the `MenuBuild` struct (after `details_copilot`, line 21):

```rust
    pub copy_diag: Option<MenuId>,
```

- [ ] **Step 7: Build the Other submenu in `build_menu`**

In `src/ui/mod.rs`, change the footer section of `build_menu` (line 107) from:

```rust
    let footer = base::append_footer(&menu);
```

to:

```rust
    let copy_diag = base::append_other(&menu);
    let footer = base::append_footer(&menu);
```

Then add `copy_diag` to the returned `MenuBuild` (after `details_copilot,` near line 125):

```rust
        copy_diag,
```

- [ ] **Step 8: Add the `App` field and wiring in `main.rs`**

In `src/main.rs`, add to the `App` struct (after `id_details_copilot`, line 50):

```rust
    id_copy_diag: Option<tray_icon::menu::MenuId>,
```

In `refresh_all`, after `self.id_details_copilot = build.details_copilot;` (line 98), add:

```rust
        self.id_copy_diag = build.copy_diag;
```

In `main()`, in the `App { … }` initializer (after `id_details_copilot: build.details_copilot,`, line 220), add:

```rust
        id_copy_diag: build.copy_diag,
```

- [ ] **Step 9: Add the click handler**

In `src/main.rs`, in the `MenuEvent` chain, add a new arm after the `id_details_copilot` arm (after line 154):

```rust
            } else if self.id_copy_diag.as_ref().is_some_and(|id| ev.id == *id) {
                crate::clipboard::copy(&crate::diag::format_all());
```

- [ ] **Step 10: Build, lint, and full test suite**

Run: `cargo build && cargo clippy -- -D warnings && cargo test`
Expected: PASS, no warnings.

- [ ] **Step 11: Manual acceptance**

Run: `make dev`. The menu shows "Other ▶". With no errors yet it contains a disabled "No diagnostics". Trigger an error (e.g. temporarily point `USAGE_URL` at a bad host, or run while rate-limited) so the log fills, then confirm "Other ▶ Diagnostics ▶ Copy diagnostic log" appears and pastes readable lines into TextEdit. Revert any temporary change before committing.

- [ ] **Step 12: Commit**

```bash
git add src/ui/base.rs src/ui/mod.rs src/main.rs
git commit -m "feat: add Other submenu with Copy diagnostic log action"
```

---

### Task 5: README Troubleshooting section

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add the Troubleshooting section**

Append (or insert before any existing license/footer section) in `README.md`:

```markdown
## Troubleshooting

If a provider row shows an error, open **Other ▶ Diagnostics ▶ Copy diagnostic log**
to copy the full diagnostic log to your clipboard. Paste it into a GitHub issue or email
when reporting a bug. The Diagnostics submenu is hidden when there is nothing to report.
```

- [ ] **Step 2: Verify it renders**

Run: `grep -n "Troubleshooting" README.md`
Expected: the new heading line is found.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add Troubleshooting section for diagnostic log"
```

---

## Self-Review

**Spec coverage:**
- `diag.rs` service (API, macro, storage, truncation, capacity) → Task 1. ✓
- `diag!` auto-injects `file!():line!()` → Task 1 Step 3. ✓
- Hook points: parse error + last-ok snapshot + raw body → Task 3 Step 6; token load failure → Task 3 Step 5; HTTP error → Task 3 Step 7. ✓ (Adapted from the spec's outdated `reqwest`/`HttpError::Status` shape to the real `ureq` `get` returning a tuple, logging all non-200 plus network failures — per the "log 429 too" decision.)
- Menu "Other ▶ Diagnostics ▶ Copy diagnostic log", hidden Diagnostics when empty → Task 4. ✓ (Per decision: "Other" always visible with a disabled "No diagnostics" placeholder when empty; placed before the footer.)
- `NSPasteboard` clipboard copy + Cargo feature → Task 2. ✓
- README Troubleshooting → Task 5. ✓

**Deviations from spec (intentional, per user decisions):**
- HTTP hook adapted to `ureq` `get` signature; logs every non-200 including 429.
- "Other" always visible with placeholder rather than conditional; constant +1 index offset keeps `styled.rs` unchanged (it reads `layout.refresh_idx`/`quit_idx`).

**Placeholder scan:** No TBD/TODO/"handle edge cases"; all steps contain concrete code or commands. The single tolerance is Task 2 Step 6, which is a real compile-verification step for the objc2 binding (genuinely environment-dependent), not a placeholder.

**Type consistency:** `Level`, `push`, `is_empty`, `format_all`, `truncate` used consistently across Tasks 1/3/4. `append_other → Option<MenuId>` matches `MenuBuild.copy_diag: Option<MenuId>` and `App.id_copy_diag: Option<MenuId>`. `last_ok_summary(&[LimitWindow]) -> String` matches its call site in Task 3 Step 6.
