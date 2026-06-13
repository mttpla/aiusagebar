# UI Progress Bar Rows Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace plain-text `LimitWindow` menu rows with custom 42pt `NSView` items showing label, colored percentage, a 4pt progress bar, and a reset-time detail line.

**Architecture:** Extend `MenuLayout` with per-window index + data so `style_menu` can swap each plain `NSMenuItem`'s content for a custom `NSView` via `setView:`, keeping the same post-build patching pattern used for headers/Refresh/Quit. Pure helpers (`bar_fill_width`, `format_reset`, `bar_fill_color`) stay side-effect-free and unit-tested; only the ObjC view builder needs `unsafe`.

**Tech Stack:** Rust, objc2 0.6, objc2-app-kit 0.3 (`NSView`, `NSTextField`, `NSBox`, `NSColor`, `NSFont`), objc2-foundation 0.3 (`NSRect`, `NSPoint`, `NSSize`, `NSString`), chrono 0.4 (reset-time formatting).

---

## File Map

| File | Change |
|---|---|
| `Cargo.toml` | Add `"NSBox"` to `objc2-app-kit` features |
| `src/ui/mod.rs` | Add `LimitWindow` import; add `window_items` field to `MenuLayout`; rewrite `build_layout` inner loop to populate it |
| `src/ui/styled.rs` | Add `NSBox`, `NSRect`, `NSPoint`, `NSSize` imports; add `bar_fill_color`, `bar_fill_width`, `format_reset`, `make_progress_row_view`; extend `style_menu` to call `setView:` on each window item |

---

## Task 1: Add NSBox Cargo Feature

**Files:**
- Modify: `Cargo.toml:20-24`

- [ ] **Step 1: Add `"NSBox"` to the `objc2-app-kit` features list**

```toml
objc2-app-kit = { version = "0.3", features = [
    "NSAlert", "NSTextField", "NSControl", "NSView", "NSText",
    "NSColor", "NSFont", "NSMenu", "NSMenuItem",
    "NSParagraphStyle", "NSAttributedString", "NSImage",
    "NSBox",
] }
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat(ui): add NSBox objc2-app-kit feature for progress bars"
```

---

## Task 2: Extend `MenuLayout` with `window_items`

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Write two failing tests in `src/ui/mod.rs`**

Add inside the existing `#[cfg(test)] mod tests` block:

```rust
#[test]
fn build_layout_window_items_indices() {
    // About(0) + sep(1) + header(2) + win0(3) + win1(4) → refresh=5
    let state = UsageState::Ok(
        vec![
            LimitWindow { name: "5h session".into(), percent_used: Some(39.0), ..Default::default() },
            LimitWindow { name: "7d weekly".into(), percent_used: Some(15.0), ..Default::default() },
        ],
        Some("max".into()),
    );
    let layout = build_layout(&[("Claude", &state)], None);
    assert_eq!(layout.window_items.len(), 2);
    assert_eq!(layout.window_items[0].0, 3); // first window at index 3
    assert_eq!(layout.window_items[1].0, 4);
    assert_eq!(layout.window_items[0].1.name, "5h session");
    assert_eq!(layout.window_items[1].1.name, "7d weekly");
}

#[test]
fn build_layout_non_ok_state_no_window_items() {
    let layout = build_layout(&[("Claude", &UsageState::NotConfigured)], None);
    assert!(layout.window_items.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p aiusagebar ui::tests
```

Expected: compile error — `window_items` field does not exist.

- [ ] **Step 3: Implement the changes**

Replace the `MenuLayout` struct and `build_layout` function in `src/ui/mod.rs`:

```rust
use crate::provider::{LimitWindow, UsageState};
```

(Replace the existing `use crate::provider::UsageState;` line.)

Replace `MenuLayout`:

```rust
pub(crate) struct MenuLayout {
    pub header_indices: Vec<(usize, ProviderKind)>,
    pub window_items: Vec<(usize, LimitWindow)>,
    pub refresh_idx: usize,
    pub quit_idx: usize,
    pub last_updated: Option<String>,
}
```

Replace `build_layout`:

```rust
pub(crate) fn build_layout(
    states: &[(&str, &UsageState)],
    last_updated: Option<&str>,
) -> MenuLayout {
    let mut idx: usize = 2; // About(0) + separator(1)
    let mut header_indices: Vec<(usize, ProviderKind)> = Vec::new();
    let mut window_items: Vec<(usize, LimitWindow)> = Vec::new();

    for (name, state) in states {
        match *name {
            "Claude" => {
                header_indices.push((idx, ProviderKind::Claude));
                if let UsageState::Ok(windows, _) = state {
                    for (i, w) in windows.iter().enumerate() {
                        window_items.push((idx + 1 + i, w.clone()));
                    }
                }
                idx += claude::section_item_count(state);
            }
            "Copilot" => {
                header_indices.push((idx, ProviderKind::Copilot));
                if let UsageState::Ok(windows, _) = state {
                    for (i, w) in windows.iter().enumerate() {
                        window_items.push((idx + 1 + i, w.clone()));
                    }
                }
                idx += copilot::section_item_count(state);
            }
            _ => idx += 1,
        }
    }

    MenuLayout {
        header_indices,
        window_items,
        refresh_idx: idx,
        quit_idx: idx + 1,
        last_updated: last_updated.map(str::to_owned),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p aiusagebar
```

Expected: all tests pass including the two new ones.

- [ ] **Step 5: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat(ui): extend MenuLayout with window_items for progress bar wiring"
```

---

## Task 3: Pure Helpers in `styled.rs`

**Files:**
- Modify: `src/ui/styled.rs`

These functions have no ObjC side effects and can be fully unit-tested. The test module at the bottom of `styled.rs` is `#[cfg(test)]` inside the `#[cfg(target_os = "macos")]` module — tests run on macOS only, which is fine for this project.

- [ ] **Step 1: Write failing tests**

Add this test module at the bottom of `src/ui/styled.rs`, before the closing `}`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::LimitWindow;

    fn make_window(name: &str, pct: Option<f32>, resets_at: Option<&str>) -> LimitWindow {
        LimitWindow {
            name: name.to_owned(),
            percent_used: pct,
            resets_at: resets_at.map(str::to_owned),
            ..Default::default()
        }
    }

    // bar_fill_width

    #[test]
    fn bar_fill_width_50_pct() {
        let w = bar_fill_width(Some(50.0));
        assert!((w - 135.0).abs() < 0.01, "got {w}");
    }

    #[test]
    fn bar_fill_width_100_pct() {
        let w = bar_fill_width(Some(100.0));
        assert!((w - 270.0).abs() < 0.01, "got {w}");
    }

    #[test]
    fn bar_fill_width_over_100_clamped() {
        assert!((bar_fill_width(Some(150.0)) - 270.0).abs() < 0.01);
    }

    #[test]
    fn bar_fill_width_none_is_zero() {
        assert_eq!(bar_fill_width(None), 0.0);
    }

    // format_reset

    #[test]
    fn format_reset_none_resets_at_returns_empty() {
        let w = make_window("5h session", None, None);
        assert_eq!(format_reset(&w), "");
    }

    #[test]
    fn format_reset_7d_window_returns_absolute_date() {
        let w = make_window("7d weekly", None, Some("2026-06-20T08:00:00Z"));
        assert_eq!(format_reset(&w), "resets Jun 20");
    }

    #[test]
    fn format_reset_5h_window_future_returns_relative_format() {
        use chrono::{Duration, Local};
        let future = (Local::now() + Duration::hours(3) + Duration::minutes(30))
            .to_rfc3339();
        let w = make_window("5h session", None, Some(&future));
        let s = format_reset(&w);
        assert!(s.starts_with("resets in"), "got: {s}");
        assert!(s.contains('h') || s.contains('m'), "got: {s}");
    }

    #[test]
    fn format_reset_5h_window_past_returns_zero() {
        let past = "2020-01-01T00:00:00Z";
        let w = make_window("5h session", None, Some(past));
        let s = format_reset(&w);
        assert_eq!(s, "resets in 0m", "got: {s}");
    }

    #[test]
    fn format_reset_unknown_window_returns_raw_with_prefix() {
        let w = make_window("Daily", None, Some("2026-06-20T08:00:00Z"));
        let s = format_reset(&w);
        assert_eq!(s, "resets 2026-06-20T08:00:00Z");
    }
}
```

- [ ] **Step 2: Run to verify they fail**

```bash
cargo test -p aiusagebar ui::styled::tests 2>&1 | head -20
```

Expected: compile error — `bar_fill_width`, `format_reset` not found.

- [ ] **Step 3: Implement the helpers**

Add these to `src/ui/styled.rs`, before the `// ── Style pass ──` section:

First, extend the imports at the top of the file. Add to the `use objc2_foundation::{...}` line:

```rust
use objc2_foundation::{NSArray, NSDictionary, NSMutableAttributedString, NSPoint, NSRange, NSRect, NSSize, NSString};
```

Add `NSBox` to the `use objc2_app_kit::{...}` block:

```rust
use objc2_app_kit::{
    NSBox, NSColor, NSFont, NSFontAttributeName, NSForegroundColorAttributeName, NSMenu,
    NSMutableParagraphStyle, NSParagraphStyleAttributeName, NSTextAlignment, NSTextField,
    NSTextTab, NSView,
};
```

Add these functions after the `refresh_attr_str` function (before the `// ── Style pass ──` section):

```rust
// ── Progress bar helpers ───────────────────────────────────────────────────

fn bar_fill_color(pct: f32) -> Retained<NSColor> {
    if pct < 60.0 {
        srgb(0.204, 0.780, 0.349) // #34C759 green
    } else if pct <= 80.0 {
        srgb(1.0, 0.624, 0.039)   // #FF9F0A amber
    } else {
        srgb(1.0, 0.231, 0.188)   // #FF3B30 red
    }
}

fn bar_fill_width(pct: Option<f32>) -> f64 {
    pct.map(|p| (p / 100.0 * 270.0) as f64)
        .unwrap_or(0.0)
        .clamp(0.0, 270.0)
}

fn format_reset(window: &crate::provider::LimitWindow) -> String {
    use chrono::DateTime;
    let Some(ref resets_at) = window.resets_at else {
        return String::new();
    };
    let name = window.name.to_lowercase();
    if name.contains("5h") || name.contains("session") {
        if let Ok(dt) = DateTime::parse_from_rfc3339(resets_at) {
            let now = chrono::Local::now();
            let secs = dt.signed_duration_since(now).num_seconds().max(0);
            let h = secs / 3600;
            let m = (secs % 3600) / 60;
            return if h > 0 {
                format!("resets in {}h {}m", h, m)
            } else {
                format!("resets in {}m", m)
            };
        }
        resets_at.clone()
    } else if name.contains("7d") || name.contains("weekly") {
        if let Ok(dt) = DateTime::parse_from_rfc3339(resets_at) {
            let local = dt.with_timezone(&chrono::Local);
            return format!("resets {}", local.format("%b %-d"));
        }
        resets_at.clone()
    } else {
        format!("resets {}", resets_at)
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p aiusagebar
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/ui/styled.rs
git commit -m "feat(ui): add bar_fill_color, bar_fill_width, format_reset helpers"
```

---

## Task 4: Build `make_progress_row_view` and Wire into `style_menu`

**Files:**
- Modify: `src/ui/styled.rs`

No unit tests for the ObjC view builder — visual acceptance via `make dev` is the test. `cargo check` catches type errors.

- [ ] **Step 1: Add `make_progress_row_view` to `styled.rs`**

Add this function after `format_reset`, still before the `// ── Style pass ──` section:

```rust
unsafe fn make_progress_row_view(window: &crate::provider::LimitWindow) -> objc2::rc::Retained<NSView> {
    let container = NSView::initWithFrame(
        NSView::alloc(),
        NSRect {
            origin: NSPoint { x: 0.0, y: 0.0 },
            size: NSSize { width: 290.0, height: 42.0 },
        },
    );

    // ── Name label (top-left, gray 11.5pt) ──
    let name_field = NSTextField::labelWithString(&NSString::from_str(&window.name));
    name_field.setFont(Some(&NSFont::systemFontOfSize(11.5)));
    name_field.setTextColor(Some(&NSColor::secondaryLabelColor()));
    name_field.setFrame(NSRect {
        origin: NSPoint { x: 8.0, y: 26.0 },
        size: NSSize { width: 155.0, height: 14.0 },
    });
    container.addSubview(&*name_field);

    // ── Pct label (top-right, bold 11.5pt, threshold color) ──
    let pct_str = window
        .percent_used
        .map(|p| format!("{:.1}%", p))
        .unwrap_or_else(|| "—".to_string());
    let pct_field = NSTextField::labelWithString(&NSString::from_str(&pct_str));
    pct_field.setFont(Some(&NSFont::boldSystemFontOfSize(11.5)));
    let threshold = bar_fill_color(window.percent_used.unwrap_or(0.0));
    pct_field.setTextColor(Some(&threshold));
    pct_field.setAlignment(NSTextAlignment::Right);
    pct_field.setFrame(NSRect {
        origin: NSPoint { x: 163.0, y: 26.0 },
        size: NSSize { width: 119.0, height: 14.0 },
    });
    container.addSubview(&*pct_field);

    // ── Bar background (separator gray) ──
    let bar_bg: objc2::rc::Retained<NSBox> =
        objc2::msg_send![NSBox::alloc(), initWithFrame: NSRect {
            origin: NSPoint { x: 8.0, y: 18.0 },
            size: NSSize { width: 270.0, height: 4.0 },
        }];
    bar_bg.setBoxType(objc2_app_kit::NSBoxType::NSBoxCustom);
    bar_bg.setFillColor(&NSColor::separatorColor());
    bar_bg.setBorderWidth(0.0);
    container.addSubview(&*bar_bg);

    // ── Bar fill (threshold color) ──
    let fill_w = bar_fill_width(window.percent_used);
    if fill_w > 0.0 {
        let bar_fill: objc2::rc::Retained<NSBox> =
            objc2::msg_send![NSBox::alloc(), initWithFrame: NSRect {
                origin: NSPoint { x: 8.0, y: 18.0 },
                size: NSSize { width: fill_w, height: 4.0 },
            }];
        bar_fill.setBoxType(objc2_app_kit::NSBoxType::NSBoxCustom);
        bar_fill.setFillColor(&bar_fill_color(window.percent_used.unwrap_or(0.0)));
        bar_fill.setBorderWidth(0.0);
        container.addSubview(&*bar_fill);
    }

    // ── Detail line (bottom, gray 10.5pt) ──
    let detail = format_reset(window);
    if !detail.is_empty() {
        let detail_field = NSTextField::labelWithString(&NSString::from_str(&detail));
        detail_field.setFont(Some(&NSFont::systemFontOfSize(10.5)));
        detail_field.setTextColor(Some(&NSColor::secondaryLabelColor()));
        detail_field.setFrame(NSRect {
            origin: NSPoint { x: 8.0, y: 2.0 },
            size: NSSize { width: 270.0, height: 14.0 },
        });
        container.addSubview(&*detail_field);
    }

    container
}
```

- [ ] **Step 2: Wire into `style_menu`**

At the end of the `unsafe` block inside `style_menu`, after the `apply_to_item(ns_menu, layout.quit_idx, &quit);` line, add:

```rust
        for (idx, window) in &layout.window_items {
            if let Some(item) = ns_menu.itemAtIndex(*idx as isize) {
                let view = make_progress_row_view(window);
                item.setView(Some(&*view));
            }
        }
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo check
```

Expected: no errors. If `NSTextField::setAlignment` or `NSBox::setBoxType` enum variants don't match, fix the names — check the objc2-app-kit 0.3 generated bindings by searching:

```bash
grep -r "NSBoxCustom\|NSBoxType" ~/.cargo/registry/src/ 2>/dev/null | grep "app-kit" | head -10
grep -r "setAlignment\|NSTextAlignment" ~/.cargo/registry/src/ 2>/dev/null | grep "app-kit" | head -10
```

- [ ] **Step 4: Run full test suite**

```bash
cargo test -p aiusagebar
```

Expected: all tests pass (no new tests for the ObjC builder — visual only).

- [ ] **Step 5: Build and run**

```bash
make dev
```

Expected: menu opens, each `LimitWindow` row shows:
- Window name (gray) at top-left
- Percentage (colored) at top-right
- 4pt horizontal bar colored green/amber/red
- Reset time detail line at bottom

If bar fill color is wrong, adjust `bar_fill_color` thresholds. If layout overlaps, adjust frame origins/sizes. If `setView:` has no visible effect, confirm `NSMenuItem` was not `enabled=false` in a way that prevents view rendering — try setting `item.setEnabled(true)` if needed.

- [ ] **Step 6: Commit**

```bash
git add src/ui/styled.rs
git commit -m "feat(ui): custom NSView progress bar rows in menu"
```

---

## Self-Review

**Spec coverage:**
- ✓ Custom NSView rows (42pt high, 290pt wide)
- ✓ Label (name, 11.5pt gray), pct (11.5pt bold, threshold color), bar (4pt NSBox), detail (reset time, 10.5pt gray)
- ✓ Threshold colors: green < 60%, amber 60–80%, red > 80%
- ✓ Bar background: separatorColor
- ✓ `resets_at` formatting: relative (5h session), absolute date (7d weekly), raw otherwise
- ✓ NSBox Cargo feature added
- ✓ Affected files: Cargo.toml, styled.rs, mod.rs (claude.rs + copilot.rs unchanged — styling is post-build)

**Placeholder scan:** None found.

**Type consistency:** `bar_fill_color(pct: f32)` → called with `unwrap_or(0.0)` everywhere. `bar_fill_width(pct: Option<f32>)` → called with window field directly. `format_reset(window: &LimitWindow)` → called with same `&LimitWindow` reference throughout. `make_progress_row_view(window: &LimitWindow)` → called from `style_menu` which iterates `layout.window_items: Vec<(usize, LimitWindow)>`.
