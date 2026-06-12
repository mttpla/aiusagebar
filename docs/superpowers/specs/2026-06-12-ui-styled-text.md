# UI Styled Text

**Date:** 2026-06-12
**Status:** Approved
**Parent spec:** `2026-06-11-ui-aesthetics-design.md` (§ Provider Header, § Footer)
**Depends on:** card #7 (`src/ui/` module must exist)

## Goal

Apply `NSAttributedString` styling to provider headers and the footer. No custom `NSView` — only `setAttributedTitle:` on existing `NSMenuItem`s. Medium ObjC2 complexity.

## Scope

- Provider headers: brand color + bold 13pt via `NSAttributedString`
- Quit item: red `#FF3B30`, 13pt
- Refresh item: `NSAttributedString` with right-aligned tab stop at 290pt for the "Updated HH:MM" suffix

Out of scope: progress bar rows (card #9), About icon (card #5).

## Technical Approach

After `build_menu` constructs the muda `Menu`, a macOS-only style pass walks the `NSMenu` by index and calls `setAttributedTitle:` on targeted items. Item indices are tracked during construction and passed to the style function.

```rust
// src/ui/mod.rs (macOS addition)
#[cfg(target_os = "macos")]
fn style_menu(menu: &Menu, layout: &MenuLayout);

struct MenuLayout {
    header_indices: Vec<(usize, ProviderKind)>,  // index in NSMenu + which brand color
    refresh_idx: usize,
    quit_idx: usize,
}

enum ProviderKind { Claude, Copilot }
```

`menu.ns_menu()` (from `muda::ContextMenu` trait) returns `*mut c_void`; cast to `*mut NSMenu` via `objc2`.

## Brand Colors (sRGB)

| Target | Hex | r / g / b |
|---|---|---|
| Claude header | `#C9551E` | 0.788 / 0.333 / 0.118 |
| Copilot header | `#6E40C9` | 0.431 / 0.251 / 0.788 |
| Quit | `#FF3B30` | 1.000 / 0.231 / 0.188 |
| Refresh | `#147EFB` | 0.078 / 0.494 / 0.984 |

## Refresh Row Layout

Single `NSMenuItem`. `NSAttributedString` with two segments:
- Left: `↺ Refresh` in blue `#147EFB`, 13pt
- Right tab stop at 290pt: `Updated HH:MM` in `NSColor.secondaryLabelColor`, 11pt

Implementation: `NSMutableParagraphStyle` + `NSTextTab(alignment: .right, location: 290)`.

## New Cargo.toml Features

```toml
objc2-app-kit = { version = "0.3", features = [
    "NSAlert", "NSTextField", "NSControl", "NSView", "NSText",
    "NSColor", "NSFont", "NSMenu", "NSMenuItem",
    "NSAttributedString", "NSParagraphStyle",
] }
objc2-foundation = { version = "0.3", features = ["NSAttributedString"] }
```

## Affected Files

- `Cargo.toml` — add features above
- `src/ui/mod.rs` — add `MenuLayout`, `style_menu` (macOS-gated), call from `build_menu`
- `src/ui/styled.rs` — new: `srgb()`, `attr_str_colored()`, `make_refresh_attr_str()`
