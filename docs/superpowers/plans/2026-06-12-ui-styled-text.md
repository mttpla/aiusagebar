# UI Styled Text Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply `NSAttributedString` styling to provider headers (brand color + bold), the Refresh item (blue + right-aligned timestamp), and the Quit item (red), via a macOS-only style pass after `build_menu` constructs the muda `Menu`.

**Architecture:** `build_menu` tracks item indices via a running counter as sections are appended. After construction, a `#[cfg(target_os = "macos")]` call to `styled::style_menu` casts the muda `Menu` to `*mut NSMenu` and calls `setAttributedTitle:` on targeted items. The separate "Updated HH:MM" label row is removed from the footer — the timestamp moves into the Refresh item's attributed string via a right-aligned tab stop at 290pt.

**Tech Stack:** Rust, objc2 0.6, objc2-app-kit 0.3, objc2-foundation 0.3, tray-icon 0.19 / muda `ContextMenu` trait.

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `Cargo.toml` | Modify | Add NSColor, NSFont, NSMenu, NSMenuItem, NSParagraphStyle features |
| `src/provider/mod.rs` | Modify | Add `Default` derive to `LimitWindow` (needed for tests) |
| `src/ui/claude.rs` | Modify | `append_claude_section` returns `usize` (items appended) |
| `src/ui/copilot.rs` | Modify | `append_copilot_section` returns `usize` (items appended) |
| `src/ui/base.rs` | Modify | Remove `refresh_label` + Updated label row; `append_footer` takes no `updated` arg |
| `src/ui/styled.rs` | Create | `srgb()`, attributed string builders, `style_menu()` (all macOS-gated) |
| `src/ui/mod.rs` | Modify | `MenuLayout`, `ProviderKind`, call `styled::style_menu`; update `build_menu` index tracking |

---

## Task 1: Cargo.toml — add objc2 features

**Files:**
- Modify: `Cargo.toml` lines 14–20

- [ ] **Step 1: Add features to the macos dependencies block**

Replace the existing `[target.'cfg(target_os = "macos")'.dependencies]` block with:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
security-framework = "3"
core-foundation = "0.9"
objc2 = "0.6"
objc2-app-kit = { version = "0.3", features = [
    "NSAlert", "NSTextField", "NSControl", "NSView", "NSText",
    "NSColor", "NSFont", "NSMenu", "NSMenuItem",
    "NSParagraphStyle",
] }
objc2-foundation = { version = "0.3", features = [
    "NSAttributedString",
] }
```

- [ ] **Step 2: Verify it compiles**

```
cargo check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: add objc2 NSColor/NSFont/NSMenu/NSParagraphStyle features"
```

---

## Task 2: Add `Default` to `LimitWindow`

**Files:**
- Modify: `src/provider/mod.rs`

`LimitWindow` currently derives `Debug, Clone, PartialEq`. Tests in later tasks use `..Default::default()` for concise construction.

- [ ] **Step 1: Write a failing test that uses Default**

Add to `src/provider/mod.rs` tests module (or any ui test module that imports LimitWindow):

```rust
#[test]
fn limit_window_default() {
    let w = crate::provider::LimitWindow::default();
    assert_eq!(w.name, "");
    assert!(w.percent_used.is_none());
}
```

- [ ] **Step 2: Run to see it fail**

```
cargo test limit_window_default 2>&1
```

Expected: compile error — `Default` not derived.

- [ ] **Step 3: Add Default derive**

In `src/provider/mod.rs`, change:

```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LimitWindow {
```

- [ ] **Step 4: Run tests**

```
cargo test
```

Expected: all pass including `limit_window_default`.

- [ ] **Step 5: Commit**

```bash
git add src/provider/mod.rs
git commit -m "refactor(provider): derive Default on LimitWindow"
```

---

## Task 3: `append_claude_section` and `append_copilot_section` return item count

`build_menu` needs to know how many NSMenu items each section appended so it can track the indices of Refresh and Quit.

**Files:**
- Modify: `src/ui/claude.rs`
- Modify: `src/ui/copilot.rs`

- [ ] **Step 1: Write failing tests in claude.rs**

Add to the `tests` module in `src/ui/claude.rs`:

```rust
#[test]
fn append_claude_section_count_ok_two_windows() {
    use crate::provider::{LimitWindow, UsageState};
    let menu = tray_icon::menu::Menu::new();
    let state = UsageState::Ok(
        vec![
            LimitWindow { name: "daily".into(), percent_used: Some(50.0), ..Default::default() },
            LimitWindow { name: "monthly".into(), percent_used: Some(20.0), ..Default::default() },
        ],
        Some("max".into()),
    );
    let count = append_claude_section(&menu, &state);
    assert_eq!(count, 3); // 1 header + 2 windows
}

#[test]
fn append_claude_section_count_not_configured() {
    let menu = tray_icon::menu::Menu::new();
    let count = append_claude_section(&menu, &UsageState::NotConfigured);
    assert_eq!(count, 1); // header only
}
```

- [ ] **Step 2: Run to see them fail**

```
cargo test append_claude_section_count 2>&1
```

Expected: compile error — function returns `()`.

- [ ] **Step 3: Update append_claude_section**

In `src/ui/claude.rs`, change the function signature and body:

```rust
pub(crate) fn append_claude_section(menu: &Menu, state: &UsageState) -> usize {
    super::append_label(menu, header_label("Claude", state));
    let mut count = 1usize;
    if let UsageState::Ok(windows, _) = state {
        for w in windows {
            let reset = w.resets_at.as_deref().unwrap_or("?");
            super::append_label(
                menu,
                format!("  {} — {}  resets {}", w.name, pct_label(w.percent_used), reset),
            );
            count += 1;
        }
    }
    count
}
```

- [ ] **Step 4: Write failing test in copilot.rs**

Add to `tests` module in `src/ui/copilot.rs`:

```rust
#[test]
fn append_copilot_section_count_ok_one_window() {
    use crate::provider::UsageState;
    let menu = tray_icon::menu::Menu::new();
    let state = UsageState::Ok(
        vec![make_window("monthly", Some(10.0), None)],
        None,
    );
    let count = append_copilot_section(&menu, &state);
    assert_eq!(count, 2); // 1 header + 1 window
}

#[test]
fn append_copilot_section_count_not_configured() {
    let menu = tray_icon::menu::Menu::new();
    let count = append_copilot_section(&menu, &UsageState::NotConfigured);
    assert_eq!(count, 1);
}
```

- [ ] **Step 5: Update append_copilot_section**

In `src/ui/copilot.rs`:

```rust
pub(crate) fn append_copilot_section(menu: &Menu, state: &UsageState) -> usize {
    super::append_label(menu, header_label("Copilot", state));
    let mut count = 1usize;
    if let UsageState::Ok(windows, _) = state {
        for w in windows {
            super::append_label(menu, row_label(w));
            count += 1;
        }
    }
    count
}
```

- [ ] **Step 6: Run all tests**

```
cargo test
```

Expected: all pass including the 4 new tests.

- [ ] **Step 7: Commit**

```bash
git add src/ui/claude.rs src/ui/copilot.rs
git commit -m "refactor(ui): append_*_section returns item count for index tracking"
```

---

## Task 4: Simplify `base.rs` — remove the separate Updated label

The Updated timestamp moves into the Refresh item's attributed string (built in Task 5). `append_footer` no longer needs `updated` and no longer appends the label row.

**Files:**
- Modify: `src/ui/base.rs`

- [ ] **Step 1: Update the tests**

In `src/ui/base.rs`, delete the tests `refresh_label_some` and `refresh_label_none`. They test a function that will be removed.

- [ ] **Step 2: Rewrite base.rs**

Replace the entire file content:

```rust
use tray_icon::menu::{Menu, MenuId, MenuItem};

pub(crate) struct FooterIds {
    pub refresh: MenuId,
    pub quit: MenuId,
}

/// Appends Refresh and Quit items. Always adds exactly 2 items.
pub(crate) fn append_footer(menu: &Menu) -> FooterIds {
    let item_refresh = MenuItem::new("↺ Refresh", true, None);
    let item_quit = MenuItem::new("Quit", true, None);
    menu.append(&item_refresh).expect("menu append failed");
    menu.append(&item_quit).expect("menu append failed");
    FooterIds {
        refresh: item_refresh.id().clone(),
        quit: item_quit.id().clone(),
    }
}
```

- [ ] **Step 3: Fix the call site — build_menu**

In `src/ui/mod.rs`, `build_menu` currently calls `base::append_footer(&menu, last_updated)`. Change to:

```rust
let footer = base::append_footer(&menu);
```

`cargo check` will tell you if there are other call sites.

- [ ] **Step 4: Run tests**

```
cargo test
```

Expected: all pass. Net test count decreases by 2 (the removed `refresh_label` tests).

- [ ] **Step 5: Commit**

```bash
git add src/ui/base.rs src/ui/mod.rs
git commit -m "refactor(ui): remove separate Updated label row, footer always 2 items"
```

---

## Task 5: Create `src/ui/styled.rs`

All code in this file is `#[cfg(target_os = "macos")]`. It casts the muda `Menu` to `*mut NSMenu` via `ContextMenu::ns_menu()`, then calls `setAttributedTitle:` on targeted items.

**Files:**
- Create: `src/ui/styled.rs`
- Modify: `src/ui/mod.rs` (declare module)

- [ ] **Step 1: Declare the module in mod.rs**

Add to `src/ui/mod.rs`:

```rust
#[cfg(target_os = "macos")]
pub(crate) mod styled;
```

- [ ] **Step 2: Create src/ui/styled.rs**

```rust
use objc2::msg_send;
use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSFont, NSMenu, NSMenuItem, NSMutableParagraphStyle, NSTextAlignment};
use objc2_foundation::{NSMutableAttributedString, NSRange, NSString};
use tray_icon::menu::{ContextMenu, Menu};

use super::MenuLayout;

// — Color helpers —

unsafe fn srgb(r: f64, g: f64, b: f64) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 1.0)
}

// — Low-level attribute helpers (msg_send for cross-version stability) —

/// Sets NSForegroundColorAttributeName on `range`.
unsafe fn set_color(mattr: &NSMutableAttributedString, color: &NSColor, range: NSRange) {
    let key = NSString::from_str("NSColor"); // NSForegroundColorAttributeName
    let _: () = msg_send![mattr, addAttribute: &*key value: color range: range];
}

/// Sets NSFontAttributeName on `range`.
unsafe fn set_font(mattr: &NSMutableAttributedString, font: &NSFont, range: NSRange) {
    let key = NSString::from_str("NSFont"); // NSFontAttributeName
    let _: () = msg_send![mattr, addAttribute: &*key value: font range: range];
}

/// Sets NSParagraphStyleAttributeName on the full string range.
unsafe fn set_para_style(mattr: &NSMutableAttributedString, style: &NSMutableParagraphStyle) {
    let key = NSString::from_str("NSParagraphStyle"); // NSParagraphStyleAttributeName
    let len = mattr.length();
    let range = NSRange { location: 0, length: len };
    let _: () = msg_send![mattr, addAttribute: &*key value: style range: range];
}

// — Paragraph style with right tab stop at 290pt —

unsafe fn refresh_para_style() -> Retained<NSMutableParagraphStyle> {
    let para = NSMutableParagraphStyle::new();
    // Build a right-aligned NSTextTab at 290pt.
    // NSTextTab initWithTextAlignment:location:options:
    let options: Retained<objc2_foundation::NSDictionary<NSString, objc2::runtime::AnyObject>> =
        objc2_foundation::NSDictionary::new();
    let tab: Retained<objc2::runtime::AnyObject> = {
        use objc2::{ClassType, msg_send_id};
        let cls = objc2::class!(NSTextTab);
        let alloc: *mut objc2::runtime::AnyObject = msg_send![cls, alloc];
        msg_send_id![
            alloc,
            initWithTextAlignment: NSTextAlignment::Right,
            location: 290.0_f64,
            options: &*options
        ]
    };
    let tabs: Retained<objc2_foundation::NSArray<objc2::runtime::AnyObject>> =
        objc2_foundation::NSArray::from_vec(vec![tab]);
    let _: () = msg_send![&*para, setTabStops: &*tabs];
    let _: () = msg_send![&*para, setDefaultTabInterval: 0.0_f64];
    para
}

// — Attributed string builders —

/// Provider header: brand color, bold 13pt.
pub(super) unsafe fn header_attr_str(
    text: &str,
    r: f64,
    g: f64,
    b: f64,
) -> Retained<NSMutableAttributedString> {
    let ns_text = NSString::from_str(text);
    let mattr = NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &ns_text);
    let range = NSRange { location: 0, length: ns_text.length() };
    set_color(&mattr, &srgb(r, g, b), range);
    set_font(&mattr, &NSFont::boldSystemFontOfSize_(13.0), range);
    mattr
}

/// Quit item: red #FF3B30, 13pt.
pub(super) unsafe fn quit_attr_str() -> Retained<NSMutableAttributedString> {
    let ns_text = NSString::from_str("Quit");
    let mattr = NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &ns_text);
    let range = NSRange { location: 0, length: ns_text.length() };
    set_color(&mattr, &srgb(1.0, 0.231, 0.188), range); // #FF3B30
    set_font(&mattr, &NSFont::systemFontOfSize_(13.0), range);
    mattr
}

/// Refresh item:
/// - Left "↺ Refresh": blue #147EFB, 13pt
/// - Right tab stop 290pt "Updated HH:MM": secondaryLabelColor, 11pt  (only when updated is Some)
pub(super) unsafe fn refresh_attr_str(updated: Option<&str>) -> Retained<NSMutableAttributedString> {
    let refresh_text = "↺ Refresh";
    let full_text = match updated {
        Some(ts) => format!("↺ Refresh\tUpdated {}", ts),
        None => refresh_text.to_owned(),
    };

    let ns_text = NSString::from_str(&full_text);
    let mattr = NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &ns_text);

    // Right tab stop paragraph style (only needed when there is a timestamp)
    if updated.is_some() {
        let para = refresh_para_style();
        set_para_style(&mattr, &para);
    }

    // "↺ Refresh" portion: blue, 13pt
    let refresh_ns = NSString::from_str(refresh_text);
    let refresh_len = refresh_ns.length(); // UTF-16 code units
    let refresh_range = NSRange { location: 0, length: refresh_len };
    set_color(&mattr, &srgb(0.078, 0.494, 0.984), refresh_range); // #147EFB
    set_font(&mattr, &NSFont::systemFontOfSize_(13.0), refresh_range);

    // "\tUpdated HH:MM" portion: secondaryLabelColor, 11pt
    if let Some(ts) = updated {
        let tab_text = format!("\tUpdated {}", ts);
        let tab_len = NSString::from_str(&tab_text).length();
        let ts_range = NSRange { location: refresh_len, length: tab_len };
        set_color(&mattr, &NSColor::secondaryLabelColor(), ts_range);
        set_font(&mattr, &NSFont::systemFontOfSize_(11.0), ts_range);
    }

    mattr
}

// — Style pass —

unsafe fn apply_to_item(ns_menu: *const NSMenu, idx: usize, attr_str: &NSMutableAttributedString) {
    if let Some(item) = (*ns_menu).itemAtIndex_(idx as isize) {
        let _: () = msg_send![&*item, setAttributedTitle: attr_str];
    }
}

pub(super) fn style_menu(menu: &Menu, layout: &MenuLayout) {
    use super::ProviderKind;
    let ns_menu = menu.ns_menu() as *const NSMenu;
    unsafe {
        for &(idx, ref kind) in &layout.header_indices {
            // Read current plain title so the header text (e.g. "Claude — max") is preserved.
            if let Some(item) = (*ns_menu).itemAtIndex_(idx as isize) {
                if let Some(title) = item.title() {
                    let text = title.to_string();
                    let (r, g, b) = match kind {
                        ProviderKind::Claude => (0.788, 0.333, 0.118),   // #C9551E
                        ProviderKind::Copilot => (0.431, 0.251, 0.788),  // #6E40C9
                    };
                    let attr = header_attr_str(&text, r, g, b);
                    let _: () = msg_send![&**item, setAttributedTitle: &*attr];
                }
            }
        }

        let refresh = refresh_attr_str(layout.last_updated.as_deref());
        apply_to_item(ns_menu, layout.refresh_idx, &refresh);

        let quit = quit_attr_str();
        apply_to_item(ns_menu, layout.quit_idx, &quit);
    }
}
```

- [ ] **Step 3: Verify compilation**

```
cargo check
```

If you get errors about missing methods (e.g. `boldSystemFontOfSize_` vs `boldSystemFont`), look up the exact name with:

```
cargo doc --open -p objc2-app-kit
```

and adjust the method name accordingly (objc2 uses selector colons → underscores, trailing colon → trailing underscore).

- [ ] **Step 4: Commit**

```bash
git add src/ui/styled.rs src/ui/mod.rs
git commit -m "feat(ui): add styled.rs — NSAttributedString helpers for menu styling"
```

---

## Task 6: Wire `MenuLayout` and `style_menu` into `build_menu`

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Write a test for index tracking**

Add to `src/ui/mod.rs` tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{LimitWindow, UsageState};

    #[test]
    fn menu_layout_indices_no_providers() {
        // No providers: About(0) + sep(1) → refresh at 2, quit at 3
        let layout = build_layout(&[], None);
        assert_eq!(layout.refresh_idx, 2);
        assert_eq!(layout.quit_idx, 3);
        assert!(layout.header_indices.is_empty());
    }

    #[test]
    fn menu_layout_indices_claude_two_windows() {
        // About(0) + sep(1) + Claude header(2) + win1(3) + win2(4) → refresh at 5, quit at 6
        let state = UsageState::Ok(
            vec![
                LimitWindow { name: "d".into(), ..Default::default() },
                LimitWindow { name: "m".into(), ..Default::default() },
            ],
            Some("max".into()),
        );
        let layout = build_layout(&[("Claude", &state)], None);
        assert_eq!(layout.header_indices[0].0, 2);
        assert_eq!(layout.refresh_idx, 5);
        assert_eq!(layout.quit_idx, 6);
    }
}
```

This test calls `build_layout`, a helper we'll extract from `build_menu`.

- [ ] **Step 2: Run to see the test fail**

```
cargo test menu_layout 2>&1
```

Expected: compile error — `build_layout` not defined.

- [ ] **Step 3: Rewrite mod.rs with MenuLayout, ProviderKind, build_layout, and updated build_menu**

```rust
use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
use crate::provider::UsageState;

pub mod base;
pub mod claude;
pub mod copilot;
#[cfg(target_os = "macos")]
pub(crate) mod styled;

pub struct MenuBuild {
    pub menu: Menu,
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
}

#[derive(Debug)]
pub(crate) enum ProviderKind {
    Claude,
    Copilot,
}

pub(crate) struct MenuLayout {
    pub header_indices: Vec<(usize, ProviderKind)>,
    pub refresh_idx: usize,
    pub quit_idx: usize,
    pub last_updated: Option<String>,
}

pub(crate) fn append_label(menu: &Menu, text: impl Into<String>) {
    menu.append(&MenuItem::new(text.into(), false, None))
        .expect("menu append failed");
}

/// Computes MenuLayout by counting items as sections are appended.
/// Separated so tests can inspect indices without building the full NSMenu.
pub(crate) fn build_layout(
    states: &[(&str, &UsageState)],
    last_updated: Option<&str>,
) -> MenuLayout {
    // About(0) + separator(1)
    let mut idx: usize = 2;
    let mut header_indices: Vec<(usize, ProviderKind)> = Vec::new();

    for (name, state) in states {
        match *name {
            "Claude" => {
                // claude::append_claude_section is not called here — we only count.
                // Count: 1 header + number of windows when Ok.
                let n = match state {
                    UsageState::Ok(windows, _) => 1 + windows.len(),
                    _ => 1,
                };
                header_indices.push((idx, ProviderKind::Claude));
                idx += n;
            }
            "Copilot" => {
                let n = match state {
                    UsageState::Ok(windows, _) => 1 + windows.len(),
                    _ => 1,
                };
                header_indices.push((idx, ProviderKind::Copilot));
                idx += n;
            }
            _ => idx += 1,
        }
    }

    MenuLayout {
        header_indices,
        refresh_idx: idx,
        quit_idx: idx + 1,
        last_updated: last_updated.map(str::to_owned),
    }
}

pub fn build_menu(states: &[(&str, &UsageState)], last_updated: Option<&str>) -> MenuBuild {
    let menu = Menu::new();
    let item_about = MenuItem::new("About AIUsageBar", true, None);
    menu.append(&item_about).expect("menu append failed");
    menu.append(&PredefinedMenuItem::separator())
        .expect("menu append failed");

    for (name, state) in states {
        match *name {
            "Claude" => { claude::append_claude_section(&menu, state); }
            "Copilot" => { copilot::append_copilot_section(&menu, state); }
            _ => append_label(&menu, format!("{}: unknown provider", name)),
        }
    }

    let footer = base::append_footer(&menu);
    let layout = build_layout(states, last_updated);

    #[cfg(target_os = "macos")]
    styled::style_menu(&menu, &layout);

    MenuBuild {
        menu,
        about: item_about.id().clone(),
        refresh: footer.refresh,
        quit: footer.quit,
    }
}
```

- [ ] **Step 4: Run all tests**

```
cargo test
```

Expected: all pass including the 2 new `menu_layout_*` tests.

- [ ] **Step 5: Build and run the app**

```
make dev
```

Open the menu bar icon. Verify:
- Claude header shows in orange (#C9551E), bold
- Copilot header (if configured) shows in purple (#6E40C9), bold
- Refresh item shows "↺ Refresh" in blue, with "Updated HH:MM" right-aligned if a timestamp is present
- Quit item shows in red (#FF3B30)

- [ ] **Step 6: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat(ui): wire MenuLayout + style_menu into build_menu"
```

---

## Self-Review

**Spec coverage:**

| Spec requirement | Task |
|---|---|
| Provider headers: brand color + bold 13pt | Task 5 `header_attr_str`, Task 6 `ProviderKind` colors |
| Claude `#C9551E`, Copilot `#6E40C9` | Task 6 `build_layout` match arm |
| Quit: red `#FF3B30`, 13pt | Task 5 `quit_attr_str` |
| Refresh: blue `#147EFB`, 13pt, left | Task 5 `refresh_attr_str` |
| Refresh: `Updated HH:MM` right-aligned at 290pt, secondary label, 11pt | Task 5 `refresh_para_style` + `refresh_attr_str` |
| `NSMutableParagraphStyle` + `NSTextTab` | Task 5 `refresh_para_style` |
| `Cargo.toml` new features | Task 1 |
| `src/ui/styled.rs` new file | Task 5 |
| `src/ui/mod.rs` `MenuLayout`, `style_menu` call | Task 6 |

**Placeholder scan:** No TBD/TODO/placeholder in code steps. The `cargo check` call in Task 5 Step 3 handles any minor API naming differences at build time.

**Type consistency:**
- `MenuLayout` defined in Task 6, referenced in Task 5 `style_menu` signature — consistent.
- `ProviderKind` defined in Task 6 `mod.rs`, used in Task 5 `style_menu` via `super::ProviderKind` — consistent.
- `build_layout` extracted in Task 6, tested in Task 6 before `build_menu` is updated — consistent.
- `append_footer` signature change (remove `updated` arg) applied in Task 4 and call site fixed in Task 4 Step 3 — consistent.
