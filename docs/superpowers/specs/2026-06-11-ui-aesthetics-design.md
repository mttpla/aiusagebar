# UI Aesthetics Redesign

**Date:** 2026-06-11
**Status:** Approved

## Problem

The current menu uses plain `MenuItem` text with no visual hierarchy, no brand identity, and no usage visualization. Progress toward limits is invisible until the tray icon changes color. About and Quit are indistinguishable from data rows.

## Goals

1. Brand-colored provider headers with inline account identity
2. Per-window progress bars colored by threshold
3. Clear footer with Refresh + Updated on one row, About + Quit below
4. `src/ui/` module folder separating rendering from data logic
5. About dialog gains the version icon from card #5 (no layout change)

---

## File Structure

New module `src/ui/` replaces the inline rendering in `main.rs`:

```
src/ui/
  mod.rs     — shared render types, pub fn build_menu(...)
  base.rs    — footer section: Refresh/Updated row + About + Quit items
  claude.rs  — Claude section: header + 5h/7d window rows
  copilot.rs — Copilot section: header + per-quota rows
```

`main.rs` calls `ui::build_menu(states, last_updated)` and gets back a `MenuBuild`. All `append_label`, `build_menu`, `MenuBuild` code moves out of `main.rs`.

Each provider UI module exposes one function:

```rust
// claude.rs
pub fn append_claude_section(menu: &Menu, state: &UsageState);

// copilot.rs
pub fn append_copilot_section(menu: &Menu, state: &UsageState);

// base.rs
pub fn append_footer(menu: &Menu) -> FooterIds;  // returns about/refresh/quit MenuIds
```

Future providers (Codex, etc.) add a new file under `src/ui/` with no changes to existing files.

---

## Menu Visual Design

### Provider Header

One `NSMenuItem` per provider, disabled (not selectable), rendered via `setAttributedTitle:` with `NSAttributedString`:

- `●` (U+25CF) in brand color + space + provider name in brand color, bold 13pt
- Identity suffix in gray 11pt: `— mttpla@gmail.com (pro)`
- On `Ok(_, None)`: suffix is `— account unavailable` in gray italic
- On `Stale` / `Error` / `NotConfigured`: no suffix, badge appended (`⚠ stale` amber, `✕ error` red)

Brand colors:
- Claude: `#C9551E` (Anthropic orange)
- Copilot: `#6E40C9` (GitHub Copilot purple)

### Window Rows (Claude — 5h session, 7d weekly)

Custom `NSView` per row set as `NSMenuItem.view`. Layout:

```
  window-label (11.5pt gray)          XX%  (11.5pt bold, threshold color)
  [████████░░░░░░░░░░░░] 4pt bar
  resets in 3h 12m  (10.5pt gray)
```

Bar fill color by `percent_used`:
- `< 60%` → `#34C759` (green)
- `60–80%` → `#FF9F0A` (amber)
- `> 80%` → `#FF3B30` (red)

`resets_at` formatting:
- 5h session: relative — `resets in 3h 12m` (diff from now, clamped to 0)
- 7d weekly: absolute date — `resets Jun 18`

### Window Rows (Copilot — per-quota)

Same custom `NSView` layout. Label is the raw `LimitWindow.name` (`mttpla / premium_interactions`). Detail line:

```
  6,604 / 7,000 left · resets Jul 1
```

If `limit` or `remaining` is `None`, omit the count and show only `resets Jul 1`.

### Separator

`PredefinedMenuItem::separator()` between providers.

### Footer

**Row 1 — Refresh:** single `NSMenuItem`, enabled. `NSAttributedString` with tab stop at menu width:
- Left: `↺ Refresh` in `#147EFB` (system blue), 13pt
- Right: `Updated 14:32` in gray 11pt (decorative, part of same item)
- Hover highlights full row blue (acceptable — Updated is not an action)

**Separator**

**Row 2 — About:** standard `NSMenuItem`, enabled. Left-aligned, 13pt:
`ℹ About AIUsageBar`

**Row 3 — Quit:** standard `NSMenuItem`, enabled. Left-aligned, 13pt, `#FF3B30`:
`✕ Quit`

---

## About Dialog

No layout change from current `about::show()`. Single addition from card #5:

- Load the compile-time icon (`include_bytes!` from `$OUT_DIR`) as `NSData` → `NSImage`
- Pass to `NSAlert.setIcon()` before `runModal()`
- Graceful fallback: if `NSImage` init fails, call `setIcon(None)` (current behavior)

Body text unchanged — all existing content including disclaimer retained:

```
© YEAR Matteo Paoli · MIT License
https://github.com/mttpla/aiusagebar

A read-only monitor. Never sends prompts, never spends quota, never modifies credentials.

This software is provided "as is", without warranty of any kind.
The author is not liable for any damages arising from its use.
```

---

## State Rendering Matrix

| Provider state | Header | Rows |
|---|---|---|
| `Ok(windows, Some(identity))` | `● Claude — email (plan)` | progress bar rows |
| `Ok(windows, None)` | `● Claude — account unavailable` (italic suffix) | progress bar rows |
| `Stale(msg)` | `● Claude ⚠ stale` | one disabled text row with msg |
| `Error(msg)` | `● Claude ✕ error` | one disabled text row with msg |
| `NotConfigured` | `● Claude` | one disabled row: `not configured` |

---

## Technical Notes

- Custom `NSView` rows require `unsafe` ObjC via `objc2` (already used in `about.rs`)
- `NSAttributedString` color on provider name requires `objc2-app-kit` `NSColor`
- Tab stop on Refresh row: `NSMutableParagraphStyle` with `addTabStop(NSTextTab(.right, location: menuWidth))`
- Menu width is not known at build time — use a fixed 290pt tab stop (matches auto-width of widest item)
- Progress bar `NSView` height: ~42pt per row (label+bar+detail)
- All `NSView` rendering is macOS-only, gated behind `#[cfg(target_os = "macos")]`

## README Update

The first section of `README.md` (immediately after the title/tagline, before any install/usage text) gains a screenshot of the live menu so a visitor understands the app at a glance.

**Source:** take a screenshot of the "stato normale" panel from the visual companion mockup `menu-with-identity.html` (left column: Claude pro + Copilot OK). Crop to the menu popup only, no desktop background.

**Asset:** commit as `assets/demo.png` (create `assets/` dir if absent). Retina PNG, 2× scale preferred.

**README insertion:**

```markdown
![AIUsageBar menu screenshot](assets/demo.png)
```

Placed between the one-line tagline and the first `##` section. No caption needed — the screenshot is self-explanatory.

---

## Affected Files

- `src/main.rs` — remove `build_menu`, `append_label`, `MenuBuild`; call `ui::build_menu`
- `src/ui/mod.rs` — new: `build_menu`, `MenuBuild`, `FooterIds`
- `src/ui/base.rs` — new: footer rendering
- `src/ui/claude.rs` — new: Claude section rendering
- `src/ui/copilot.rs` — new: Copilot section rendering
- `src/about.rs` — add icon loading from `$OUT_DIR` bytes
- `src/provider/mod.rs` — no change (types already defined)
