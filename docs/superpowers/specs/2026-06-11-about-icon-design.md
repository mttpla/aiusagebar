# About Icon — Design Spec

**Date:** 2026-06-11
**Status:** Approved

---

## Overview

Generate a 128×128 PNG icon displaying `[{version}]` in Courier Prime Bold on a transparent background. The icon is produced at compile time by `build.rs`, embedded into the binary with `include_bytes!`, and passed to the NSAlert in `about::show()` as the app icon.

The PNG lives only in `$OUT_DIR` — it is never committed to the repo and regenerates automatically on every `cargo build`.

---

## Visual Design

| Property | Value |
|---|---|
| Canvas | 128×128 px RGBA |
| Background | Fully transparent (alpha = 0) |
| Text | `[{version}]` — e.g. `[0.1.0]` |
| Font | Courier Prime Bold |
| Color | Black (`#000000`, alpha = 255) |
| Alignment | Centered horizontally and vertically |
| Text width target | 80% of canvas width (~102 px) |
| Font size | Auto-scaled: measure at 1 pt, scale to hit target width |

Auto-scaling ensures the icon looks consistent across all version strings (from `[0.1.0]` to `[99.99.99]`).

---

## Font

**Courier Prime Bold** — free, SIL Open Font License.

- File: `assets/fonts/CourierPrime-Bold.ttf`
- Committed to the repo (~120 KB)
- Downloaded once from: https://quoteunquote.com/courierprime/

---

## Build Script (`build.rs`)

`build.rs` is the sole owner of icon generation. It:

1. Reads `CARGO_PKG_VERSION` from the build environment
2. Formats the text string: `format!("[{}]", version)`
3. Loads `assets/fonts/CourierPrime-Bold.ttf` via `ab_glyph::FontRef`
4. Measures the layout width of the string at scale 1.0
5. Computes `font_scale = (128.0 * 0.80) / measured_width`
6. Lays out glyphs centered on a 128×128 `image::RgbaImage` (transparent background)
7. Rasterizes each glyph pixel into the image (black, full alpha)
8. Writes the result to `$OUT_DIR/about-icon.png`
9. Emits `cargo:rerun-if-changed=assets/fonts/CourierPrime-Bold.ttf` and `cargo:rerun-if-env-changed=CARGO_PKG_VERSION`

**Build dependencies added to `Cargo.toml`:**

```toml
[build-dependencies]
ab_glyph = "0.7"
image = { version = "0.25", default-features = false, features = ["png"] }
```

Note: `image` is already a runtime dependency; it must also be declared as a build dependency since build scripts have an isolated dep graph.

---

## `src/about.rs` Integration

### Embed icon bytes

At the top of the `show()` function (macOS-only block):

```rust
const ABOUT_ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/about-icon.png"));
```

### Create NSImage and pass to NSAlert

Replace the current `unsafe { alert.setIcon(None) }` with:

```rust
use objc2_app_kit::NSImage;
use objc2_foundation::NSData;

let icon_data = NSData::with_bytes(ABOUT_ICON);
if let Some(icon) = NSImage::initWithData(NSImage::alloc(), &icon_data) {
    alert.setIcon(Some(&icon));
}
```

If `NSImage::initWithData` fails (should never happen with a valid PNG), the alert falls back to showing no icon — graceful degradation.

### `Cargo.toml` runtime additions

```toml
[target.'cfg(target_os = "macos")'.dependencies]
# add to existing list:
objc2-app-kit = { version = "0.3", features = ["NSAlert", "NSTextField", "NSControl", "NSView", "NSText", "NSImage"] }
```

`NSData` is already available via `objc2-foundation` without a feature flag.

---

## File Layout

```
assets/
  fonts/
    CourierPrime-Bold.ttf   ← committed, ~120 KB
build.rs                    ← modified: add icon generation
src/about.rs                ← modified: embed + pass icon
Cargo.toml                  ← modified: build-deps + NSImage feature
```

---

## Out of Scope

- Dark mode variant (NSAlert uses the icon as-is; black on transparent reads fine in both modes on macOS)
- Menu bar icon (separate feature; this icon is only for the NSAlert)
- Runtime regeneration (compile-time only)
