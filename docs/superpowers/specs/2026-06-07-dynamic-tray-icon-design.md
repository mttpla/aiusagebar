# Dynamic Tray Icon Design

## Summary

Replace the static tray icon with a three-state dynamic icon that communicates AI usage status at a glance. Icon switches on every `refresh()` call (user-triggered, no background polling).

## Icon States

| State | Icon | Trigger |
|---|---|---|
| Normal | Brain, white/template | `Ok` + all provider windows < 80% |
| Alert | Brain + red dot badge | `Ok` + at least one window ≥ 80% |
| Unavailable | Brain, grey | `Error` / `Stale` / `NotConfigured` |

**Threshold**: 80% hardcoded. Future work: make configurable per-provider or per-window (Claude has both 5h and 7h windows — which to use needs separate design).

## Asset Pipeline

1. Download `fa-brain` SVG from Font Awesome Free (CC BY 4.0).
2. Run one-shot Python script (`scripts/gen_icons.py`) to produce:
   - `icons/brain_normal.png` — white brain, 32×32 px
   - `icons/brain_alert.png` — white brain + red circle (8px) bottom-right corner, 32×32 px
   - `icons/brain_grey.png` — grey brain, 32×32 px
3. Commit all three PNGs to the repo.
4. Script is committed but not run at build time — PNGs are the source of truth.

**Why 32×32**: macOS Retina displays @2x. tray-icon renders at correct logical size.

## Embedding

Icons embedded at compile time via `include_bytes!()` — no runtime file I/O, binary is self-contained:

```rust
static ICON_NORMAL: &[u8] = include_bytes!("../icons/brain_normal.png");
static ICON_ALERT:  &[u8] = include_bytes!("../icons/brain_alert.png");
static ICON_GREY:   &[u8] = include_bytes!("../icons/brain_grey.png");
```

`load_icon()` is replaced by `parse_icon(bytes: &[u8])` — no path logic, no fallback placeholder needed.

## Icon Selection Logic

After each `fetch()`, compute which icon to show:

```
fn icon_for_state(state: &UsageState) -> IconKind {
    match state {
        Ok(windows) => {
            if windows.iter().any(|w| w.percent_used.unwrap_or(0.0) >= 80.0) {
                Alert
            } else {
                Normal
            }
        }
        _ => Unavailable,
    }
}
```

`App` stores three pre-parsed `tray_icon::Icon` instances. `refresh()` calls `tray.set_icon()` with the result.

## README Updates

- Add "Icons by [Font Awesome](https://fontawesome.com)" attribution (CC BY 4.0 requirement).
- Add icon legend:

```
| Icon | Meaning |
|---|---|
| Brain (white) | All usage under 80% |
| Brain + red dot | At least one provider at or above 80% |
| Brain (grey) | Data unavailable (error, stale, or not configured) |
```

## Out of Scope

- Background polling (future plan)
- Configurable threshold (future plan)
- Per-provider icon (single global icon covers all providers)
- Codex / Copilot providers (separate plan)
