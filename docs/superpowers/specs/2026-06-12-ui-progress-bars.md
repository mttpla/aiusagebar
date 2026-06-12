# UI Progress Bar Rows

**Date:** 2026-06-12
**Status:** Approved
**Parent spec:** `2026-06-11-ui-aesthetics-design.md` (§ Window Rows)
**Depends on:** card #8 (styled text must be in place)

## Goal

Replace plain-text window rows with custom `NSView` items containing a colored progress bar. High ObjC2 complexity — isolated from the rest of the UI so it can be deferred or simplified without breaking other cards.

## Scope

Each `LimitWindow` row becomes a 42pt-high `NSMenuItem` with `setView:` pointing to a custom `NSView`. Plain-text fallback remains on non-macOS.

Out of scope: provider headers, footer, About icon.

## Row Layout (per window, fixed width 290pt)

```
 8pt margin
 [label, 11.5pt gray]         [pct, 11.5pt bold, threshold color]   ← y=26pt
 [████████░░░░░░░░░░░░░░░]  4pt bar                                  ← y=18pt
 [detail, 10.5pt gray]                                               ← y=2pt
```

All frames are fixed (no Auto Layout). Total row height: 42pt.

## Threshold Colors (bar fill + pct text)

| Range | Color | Hex |
|---|---|---|
| `< 60%` | green | `#34C759` |
| `60–80%` | amber | `#FF9F0A` |
| `> 80%` | red | `#FF3B30` |

Bar background: `NSColor.separatorColor` (system adaptive gray).

## NSView Construction

Container: `NSView::initWithFrame(NSRect { x:0, y:0, w:290, h:42 })`.

Text fields: `NSTextField::labelWithString` + `setFont` + `setTextColor` + `setFrame`.

Progress bar: two `NSBox` instances (background + fill), `boxType = .custom`, `setFillColor`, `setBorderWidth(0)`.

```
bar_bg:   x=8,  y=18, w=270, h=4  — NSColor.separatorColor
bar_fill: x=8,  y=18, w=fill_w, h=4  — threshold color
fill_w = (pct / 100.0 * 270.0).clamp(0.0, 270.0)
```

## `resets_at` Formatting

| Window name contains | Format |
|---|---|
| `5h` / session | relative: `resets in 3h 12m` (diff from `Local::now()`, clamped ≥ 0) |
| `7d` / weekly | absolute: `resets Jun 18` (`%b %-d` strftime) |
| anything else | raw `resets_at` string, or omitted if `None` |

## New Cargo.toml Features (addition to card #8)

```toml
objc2-app-kit = { version = "0.3", features = [
    ...,   # all from card #8
    "NSBox",
] }
```

## Affected Files

- `Cargo.toml` — add `NSBox` feature
- `src/ui/claude.rs` — macOS: replace plain-text rows with `make_progress_row_item()`
- `src/ui/copilot.rs` — macOS: same
- `src/ui/styled.rs` — add `make_progress_row_item()`, `bar_fill_width()`, `format_reset()`
