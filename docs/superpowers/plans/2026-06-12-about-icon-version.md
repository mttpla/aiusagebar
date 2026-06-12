# About Icon with Version Number — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generate a `[0.1.0]`-style 128×128 PNG icon at compile time and display it as the icon in the About `NSAlert`.

**Architecture:** `build.rs` renders the icon using `ab_glyph` + `image`, writing `about-icon.png` to `$OUT_DIR`. `src/about.rs` embeds it via `include_bytes!` (compile-time, zero runtime cost) and passes it to `NSAlert.setIcon()` via `NSImage`/`NSData`. If `initWithData` fails, the alert falls back to no icon gracefully.

**Tech Stack:** `ab_glyph 0.7` (font rasterisation), `image 0.25` (PNG encoding, already a runtime dep), `objc2-app-kit 0.3` (`NSImage`), `objc2-foundation 0.3` (`NSData`), Courier Prime Bold TTF (OFL-licensed, committed to `assets/fonts/`).

---

### Task 1: Download and commit Courier Prime Bold font

**Files:**
- Create: `assets/fonts/CourierPrime-Bold.ttf`

- [ ] **Step 1: Create the directory and download the font**

```bash
mkdir -p assets/fonts
# Download from the official source listed in the spec
curl -L "https://quoteunquote.com/courierprime/CourierPrime-Bold.ttf" \
     -o assets/fonts/CourierPrime-Bold.ttf
```

If the direct URL doesn't work, download the ZIP from `https://quoteunquote.com/courierprime/` and extract `CourierPrime-Bold.ttf` into `assets/fonts/`.

- [ ] **Step 2: Verify the file is a valid TTF**

```bash
file assets/fonts/CourierPrime-Bold.ttf
# Expected: TrueType Font data or OpenType font data
ls -lh assets/fonts/CourierPrime-Bold.ttf
# Expected: ~100-130 KB
```

- [ ] **Step 3: Commit**

```bash
git add assets/fonts/CourierPrime-Bold.ttf
git commit -m "chore: add Courier Prime Bold font (OFL) for about icon generation"
```

---

### Task 2: Extend Cargo.toml

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add `ab_glyph` and `image` as build-dependencies**

`image` is already a runtime dep; it must also be declared separately in `[build-dependencies]` because build scripts have an isolated dependency graph.

Current `[build-dependencies]` block:
```toml
[build-dependencies]
vergen-git2 = "1"
```

Replace with:
```toml
[build-dependencies]
vergen-git2 = "1"
ab_glyph = "0.7"
image = { version = "0.25", default-features = false, features = ["png"] }
```

- [ ] **Step 2: Add `NSImage` to `objc2-app-kit` features**

Current macOS deps:
```toml
objc2-app-kit = { version = "0.3", features = ["NSAlert", "NSTextField", "NSControl", "NSView", "NSText"] }
```

Replace with:
```toml
objc2-app-kit = { version = "0.3", features = ["NSAlert", "NSTextField", "NSControl", "NSView", "NSText", "NSImage"] }
```

- [ ] **Step 3: Verify deps resolve**

```bash
cargo check
# Expected: compiles cleanly (no code uses the new items yet)
```

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: add ab_glyph + image build-deps, NSImage feature for about icon"
```

---

### Task 3: Write failing test — add `ABOUT_ICON` const

**Files:**
- Modify: `src/about.rs`

This is the TDD red step. We add the const and a PNG-validity test; `cargo test` will fail with a compile error because `build.rs` hasn't generated the file yet.

- [ ] **Step 1: Add `ABOUT_ICON` const at module level (after `START_YEAR`)**

```rust
#[cfg(target_os = "macos")]
const ABOUT_ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/about-icon.png"));
```

Insert after line 3 (`const START_YEAR: i32 = 2026;`).

- [ ] **Step 2: Add PNG-validity test to the existing `mod tests` block**

Append inside the `mod tests` block (after the last existing `#[test]` fn):

```rust
#[cfg(target_os = "macos")]
#[test]
fn about_icon_is_valid_png() {
    // PNG magic bytes: 0x89 P N G \r \n 0x1A \n
    const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";
    assert!(
        ABOUT_ICON.starts_with(PNG_MAGIC),
        "ABOUT_ICON must start with PNG magic bytes"
    );
    assert!(ABOUT_ICON.len() > 128, "ABOUT_ICON must have non-trivial content");
}
```

- [ ] **Step 3: Run — expect a compile error (red)**

```bash
cargo test 2>&1 | head -20
# Expected: error about missing file about-icon.png (OUT_DIR/.../about-icon.png not found)
# This confirms the test is properly wired — build.rs must generate the file.
```

---

### Task 4: Implement icon generation in `build.rs`

**Files:**
- Modify: `build.rs`

This is the TDD green step. After this task, `cargo test` must pass.

- [ ] **Step 1: Add `generate_about_icon()` function to `build.rs`**

Append the following function to `build.rs` (after `main()`):

```rust
fn generate_about_icon() {
    use ab_glyph::{Font, FontRef, Glyph, PxScale, ScaleFont, point};
    use image::{ImageBuffer, Rgba};

    const SIZE: u32 = 128;

    let version = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION");
    let text = format!("[{}]", version);

    let font_bytes = std::fs::read("assets/fonts/CourierPrime-Bold.ttf")
        .expect("failed to read assets/fonts/CourierPrime-Bold.ttf");
    let font = FontRef::try_from_slice(&font_bytes).expect("failed to parse font");

    // Measure total advance at scale 1.0, then compute scale to hit 80% canvas width
    let scaled_1 = font.as_scaled(PxScale::from(1.0));
    let width_at_1: f32 = text
        .chars()
        .map(|c| scaled_1.h_advance(font.glyph_id(c)))
        .sum();
    let font_scale_value = (SIZE as f32 * 0.80) / width_at_1;
    let font_scale = PxScale::from(font_scale_value);
    let scaled = font.as_scaled(font_scale);

    // Horizontal centering: total advance at final scale
    let total_advance: f32 = text
        .chars()
        .map(|c| scaled.h_advance(font.glyph_id(c)))
        .sum();
    let start_x = (SIZE as f32 - total_advance) / 2.0;

    // Vertical centering: baseline so the full glyph block is centred
    let ascent = scaled.ascent(); // positive: distance above baseline
    let descent = scaled.descent(); // negative: distance below baseline
    let text_height = ascent - descent;
    let baseline_y = (SIZE as f32 - text_height) / 2.0 + ascent;

    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(SIZE, SIZE);
    let mut caret_x = start_x;

    for c in text.chars() {
        let glyph_id = font.glyph_id(c);
        let glyph = Glyph {
            id: glyph_id,
            scale: font_scale,
            position: point(caret_x, baseline_y),
        };
        caret_x += scaled.h_advance(glyph_id);
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|x, y, coverage| {
                let px = bounds.min.x as i32 + x as i32;
                let py = bounds.min.y as i32 + y as i32;
                if px >= 0 && px < SIZE as i32 && py >= 0 && py < SIZE as i32 {
                    img.put_pixel(
                        px as u32,
                        py as u32,
                        Rgba([0, 0, 0, (coverage * 255.0) as u8]),
                    );
                }
            });
        }
    }

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    let icon_path = std::path::Path::new(&out_dir).join("about-icon.png");
    img.save(&icon_path).expect("failed to write about-icon.png");

    println!("cargo:rerun-if-changed=assets/fonts/CourierPrime-Bold.ttf");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
}
```

- [ ] **Step 2: Call `generate_about_icon()` in `main()`**

Current `main()` in `build.rs`:
```rust
fn main() {
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=11.0");

    vergen_git2::Emitter::default()
        ...
}
```

Insert `generate_about_icon();` before the `vergen_git2` block:
```rust
fn main() {
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=11.0");

    generate_about_icon();

    vergen_git2::Emitter::default()
        .add_instructions(
            &vergen_git2::Git2Builder::default()
                .describe(true, true, None)
                .build()
                .unwrap(),
        )
        .unwrap()
        .emit()
        .unwrap();
}
```

- [ ] **Step 3: Run tests — expect all green**

```bash
cargo test
# Expected: test result: ok. N passed; 0 failed
# The new test about_icon_is_valid_png must be among the passing tests.
```

- [ ] **Step 4: Verify the PNG is generated in OUT_DIR**

```bash
find target/debug/build/aiusagebar-*/out/about-icon.png 2>/dev/null
ls -lh "$(find target/debug/build/aiusagebar-*/out/about-icon.png 2>/dev/null | head -1)"
# Expected: a ~2-10 KB PNG file
```

- [ ] **Step 5: Commit**

```bash
git add build.rs src/about.rs
git commit -m "feat: generate about icon PNG at compile time via build.rs + ab_glyph"
```

---

### Task 5: Wire up icon in `src/about.rs` `show()`

**Files:**
- Modify: `src/about.rs`

- [ ] **Step 1: Add `NSData` to the `objc2_foundation` import and `NSImage` to the `objc2_app_kit` import inside `show()`**

Current imports at the top of `show()`:
```rust
use objc2_app_kit::{NSAlert, NSAlertSecondButtonReturn, NSTextField, NSTextAlignment};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
```

Replace with:
```rust
use objc2_app_kit::{NSAlert, NSAlertSecondButtonReturn, NSImage, NSTextField, NSTextAlignment};
use objc2_foundation::{NSData, NSPoint, NSRect, NSSize, NSString};
```

- [ ] **Step 2: Replace `setIcon(None)` with icon creation**

Current line:
```rust
unsafe { alert.setIcon(None) };
```

Replace with:
```rust
let icon_data = NSData::with_bytes(ABOUT_ICON);
let icon = unsafe { NSImage::initWithData(NSImage::alloc(), &icon_data) };
unsafe { alert.setIcon(icon.as_deref()) };
```

`icon.as_deref()` converts `Option<Retained<NSImage>>` to `Option<&NSImage>`. If `initWithData` returns `None` (should never happen with a valid PNG), `setIcon(None)` is called as a graceful fallback.

- [ ] **Step 3: Run tests to confirm nothing regressed**

```bash
cargo test
# Expected: test result: ok. N passed; 0 failed (same count as before, including about_icon_is_valid_png)
```

- [ ] **Step 4: Build and run manually**

```bash
make dev
```

Click the menu bar icon → "About AIUsageBar" → verify the About alert shows the `[0.1.0]` icon in the top-left corner of the NSAlert.

- [ ] **Step 5: Commit**

```bash
git add src/about.rs
git commit -m "feat: display compiled about icon in NSAlert via NSImage"
```

---

## Self-Review

### Spec coverage

| Spec requirement | Covered by |
|---|---|
| 128×128 RGBA canvas, transparent background | Task 4 `ImageBuffer::new(SIZE, SIZE)`, no background fill |
| Text `[{version}]` | Task 4 `format!("[{}]", version)` |
| Courier Prime Bold | Task 1 + Task 4 font load |
| Auto-scale to 80% canvas width | Task 4 `(SIZE * 0.80) / width_at_1` |
| Centered horizontally and vertically | Task 4 `start_x` + `baseline_y` |
| Black text, full alpha | Task 4 `Rgba([0, 0, 0, (coverage * 255.0) as u8])` |
| Write to `$OUT_DIR/about-icon.png` | Task 4 `icon_path` |
| `cargo:rerun-if-changed` declarations | Task 4 `println!("cargo:rerun-if-changed=...")` |
| Build deps: `ab_glyph`, `image` | Task 2 |
| `include_bytes!` embed in `about.rs` | Task 3 `ABOUT_ICON` const |
| NSImage + NSData integration | Task 5 |
| Graceful fallback if `initWithData` fails | Task 5 `icon.as_deref()` passes `None` through |
| `NSImage` feature in `objc2-app-kit` | Task 2 |
| Font file committed to `assets/fonts/` | Task 1 |
| PNG never committed | Not in `.gitignore` but lives in `$OUT_DIR` (never in working tree) |

No gaps found.

### Placeholder scan

No TBD, TODO, "implement later", or "add appropriate error handling" found. All code steps contain complete, runnable code.

### Type consistency

- `ABOUT_ICON: &[u8]` — defined Task 3, used in Task 5 (`NSData::with_bytes(ABOUT_ICON)`) ✓
- `NSImage::initWithData(NSImage::alloc(), &icon_data)` returns `Option<Retained<NSImage>>` — `.as_deref()` gives `Option<&NSImage>` — matches `setIcon(Option<&NSImage>)` signature ✓
- `ImageBuffer<Rgba<u8>, Vec<u8>>` — consistent with `img.put_pixel(_, _, Rgba([...]))` ✓
- `PxScale::from(f32)` used consistently throughout Task 4 ✓
