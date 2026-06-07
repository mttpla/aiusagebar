# Dynamic Tray Icon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the static tray icon with a three-state dynamic icon (normal / alert / unavailable) that updates on every user-triggered refresh.

**Architecture:** Three PNGs generated once by an offline Python script and committed to the repo. Embedded at compile time via `include_bytes!()` — no runtime file I/O, binary is self-contained. `icon_for_state()` maps `UsageState` to one of three icon byte slices. `refresh()` calls `tray.set_icon()` after each `fetch()`.

**Tech Stack:** Rust, `tray-icon` crate, `image` crate (already in Cargo.toml), Python + `cairosvg` + `Pillow` for one-time asset generation.

---

### Task 1: Generate icon assets

**Files:**
- Create: `scripts/gen_icons.py`
- Create: `icons/brain_normal.png`
- Create: `icons/brain_alert.png`
- Create: `icons/brain_unavailable.png`

- [ ] **Step 1: Install Python dependencies**

```bash
pip install cairosvg pillow
```

- [ ] **Step 2: Download FA brain SVG**

Go to fontawesome.com → search "brain" → Free / Solid style → Download SVG. Save the file as `/tmp/brain.svg`.

- [ ] **Step 3: Create `icons/` directory and `scripts/` directory**

```bash
mkdir -p icons scripts
```

- [ ] **Step 4: Write `scripts/gen_icons.py`**

```python
#!/usr/bin/env python3
"""
Generate tray icon assets from Font Awesome brain SVG (CC BY 4.0).
Usage: python scripts/gen_icons.py <path-to-brain.svg>
Run once; commit the generated PNGs.
"""
import io
import sys

import cairosvg
from PIL import Image, ImageDraw

SIZE = 32


def svg_to_image(svg_path: str, tint: tuple) -> Image.Image:
    png = cairosvg.svg2png(url=svg_path, output_width=SIZE, output_height=SIZE)
    img = Image.open(io.BytesIO(png)).convert("RGBA")
    pixels = img.load()
    for y in range(img.height):
        for x in range(img.width):
            _, _, _, a = pixels[x, y]
            if a > 0:
                pixels[x, y] = (*tint, a)
    return img


def add_red_dot(img: Image.Image) -> Image.Image:
    out = img.copy()
    draw = ImageDraw.Draw(out)
    r = 5
    draw.ellipse([SIZE - r * 2, SIZE - r * 2, SIZE, SIZE], fill=(220, 50, 50, 255))
    return out


if len(sys.argv) != 2:
    print("Usage: python scripts/gen_icons.py <brain.svg>")
    sys.exit(1)

svg = sys.argv[1]

normal = svg_to_image(svg, (255, 255, 255))
normal.save("icons/brain_normal.png")

alert = add_red_dot(normal)
alert.save("icons/brain_alert.png")

unavailable = svg_to_image(svg, (160, 160, 160))
unavailable.save("icons/brain_unavailable.png")

print("Generated: icons/brain_normal.png  icons/brain_alert.png  icons/brain_unavailable.png")
```

- [ ] **Step 5: Run the script**

```bash
python scripts/gen_icons.py /tmp/brain.svg
```

Expected output:
```
Generated: icons/brain_normal.png  icons/brain_alert.png  icons/brain_unavailable.png
```

- [ ] **Step 6: Verify icons look correct**

Open each PNG (e.g., `open icons/brain_normal.png`) and confirm:
- `brain_normal.png`: white brain on transparent background, 32×32
- `brain_alert.png`: white brain + red circle in bottom-right corner, 32×32
- `brain_unavailable.png`: grey brain on transparent background, 32×32

- [ ] **Step 7: Commit**

```bash
git add scripts/gen_icons.py icons/brain_normal.png icons/brain_alert.png icons/brain_unavailable.png
git commit -m "feat: add tray icon assets (FA brain, CC BY 4.0)"
```

---

### Task 2: Icon state logic (TDD)

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add `IconKind` enum to `src/main.rs`**

After the existing `use` imports (around line 11, before `struct App`), add:

```rust
#[derive(Clone, Copy, PartialEq, Debug)]
enum IconKind {
    Normal,
    Alert,
    Unavailable,
}
```

- [ ] **Step 2: Add failing tests to the bottom of `src/main.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use provider::{LimitWindow, UsageState};

    fn window(pct: Option<f32>) -> LimitWindow {
        LimitWindow {
            name: "test".into(),
            percent_used: pct,
            limit: None,
            remaining: None,
            resets_at: None,
            unlimited: false,
        }
    }

    #[test]
    fn icon_normal_when_all_under_threshold() {
        let state = UsageState::Ok(vec![window(Some(50.0)), window(Some(79.9))]);
        assert_eq!(icon_for_state(&state), IconKind::Normal);
    }

    #[test]
    fn icon_alert_when_any_at_threshold() {
        let state = UsageState::Ok(vec![window(Some(50.0)), window(Some(80.0))]);
        assert_eq!(icon_for_state(&state), IconKind::Alert);
    }

    #[test]
    fn icon_alert_when_any_over_threshold() {
        let state = UsageState::Ok(vec![window(Some(95.0))]);
        assert_eq!(icon_for_state(&state), IconKind::Alert);
    }

    #[test]
    fn icon_unavailable_on_error() {
        assert_eq!(icon_for_state(&UsageState::Error("e".into())), IconKind::Unavailable);
    }

    #[test]
    fn icon_unavailable_on_stale() {
        assert_eq!(icon_for_state(&UsageState::Stale("s".into())), IconKind::Unavailable);
    }

    #[test]
    fn icon_unavailable_on_not_configured() {
        assert_eq!(icon_for_state(&UsageState::NotConfigured), IconKind::Unavailable);
    }

    #[test]
    fn icon_normal_when_percent_unknown() {
        // None percent_used (unlimited window) → treated as 0.0 → Normal
        let state = UsageState::Ok(vec![window(None)]);
        assert_eq!(icon_for_state(&state), IconKind::Normal);
    }
}
```

- [ ] **Step 3: Run tests — expect compile error**

```bash
cargo test 2>&1 | head -10
```

Expected: `error[E0425]: cannot find function 'icon_for_state'`

- [ ] **Step 4: Implement `icon_for_state()`**

Add after the `IconKind` enum in `src/main.rs`:

```rust
fn icon_for_state(state: &UsageState) -> IconKind {
    match state {
        UsageState::Ok(windows) => {
            if windows.iter().any(|w| w.percent_used.unwrap_or(0.0) >= 80.0) {
                IconKind::Alert
            } else {
                IconKind::Normal
            }
        }
        _ => IconKind::Unavailable,
    }
}
```

- [ ] **Step 5: Run tests — all must pass**

```bash
cargo test
```

Expected:
```
test tests::icon_alert_when_any_at_threshold ... ok
test tests::icon_alert_when_any_over_threshold ... ok
test tests::icon_normal_when_all_under_threshold ... ok
test tests::icon_normal_when_percent_unknown ... ok
test tests::icon_unavailable_on_error ... ok
test tests::icon_unavailable_on_not_configured ... ok
test tests::icon_unavailable_on_stale ... ok
test result: ok. 7 passed; 0 failed
```

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: add icon_for_state() with hardcoded 80% threshold"
```

---

### Task 3: Embed icons and update icon loading

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Replace `load_icon()` with embedded statics and `parse_icon()`**

Delete the entire `fn load_icon()` block at the bottom of `src/main.rs` (lines 131–149) and replace with:

```rust
static ICON_NORMAL: &[u8] = include_bytes!("../icons/brain_normal.png");
static ICON_ALERT: &[u8] = include_bytes!("../icons/brain_alert.png");
static ICON_UNAVAILABLE: &[u8] = include_bytes!("../icons/brain_unavailable.png");

fn parse_icon(bytes: &[u8]) -> tray_icon::Icon {
    let img = image::load_from_memory(bytes)
        .expect("failed to decode icon")
        .into_rgba8();
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).expect("failed to create icon")
}
```

- [ ] **Step 2: Update `App::refresh()` to set the icon**

Replace the current `refresh()` method with:

```rust
fn refresh(&mut self) {
    let state = self.claude.fetch();
    let (menu, id_refresh, id_quit) = Self::build_menu(self.claude.name(), &state);
    self.id_refresh = id_refresh;
    self.id_quit = id_quit;
    self.tray.set_menu(Some(Box::new(menu)));
    let icon_bytes = match icon_for_state(&state) {
        IconKind::Normal => ICON_NORMAL,
        IconKind::Alert => ICON_ALERT,
        IconKind::Unavailable => ICON_UNAVAILABLE,
    };
    self.tray.set_icon(Some(parse_icon(icon_bytes))).ok();
}
```

- [ ] **Step 3: Update `main()` to use embedded icon**

In `fn main()`, replace:

```rust
let icon = load_icon();
```

with:

```rust
let icon = parse_icon(ICON_UNAVAILABLE);
```

The app starts grey (unavailable) until the first `refresh()` sets the correct icon.

- [ ] **Step 4: Verify compile + tests pass**

```bash
cargo test
```

Expected: all 7 icon tests pass, 0 compile errors.

- [ ] **Step 5: Build release and smoke-test**

```bash
make dev
```

Expected: app launches, tray icon shows brain (grey initially, then correct state after clicking Refresh).

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: embed icons via include_bytes!(), switch icon on refresh"
```

---

### Task 4: Update README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add FA attribution and icon legend**

Add this section to `README.md` (after the existing content):

```markdown
## Icons

Icons by [Font Awesome](https://fontawesome.com) (CC BY 4.0).

| Tray icon | Meaning |
|---|---|
| Brain (white) | All AI usage under 80% |
| Brain + red dot | At least one provider at or above 80% usage |
| Brain (grey) | Data unavailable — not configured, stale, or fetch error |
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add FA attribution and tray icon legend to README"
```
