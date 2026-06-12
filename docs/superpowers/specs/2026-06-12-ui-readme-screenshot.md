# UI README Screenshot

**Date:** 2026-06-12
**Status:** Approved
**Parent spec:** `2026-06-11-ui-aesthetics-design.md` (§ README Update)
**Depends on:** card #9 (progress bars must be done — screenshot should show final UI)

## Goal

Add a screenshot of the live menu to `README.md` so a visitor immediately understands what the app looks like.

## Scope

- Take screenshot of the running app (menu open, Claude pro + Copilot OK state)
- Commit as `assets/demo.png`
- Insert one line in `README.md`

No code changes.

## Screenshot Requirements

- Menu popup only — no desktop background, no window chrome
- Retina PNG (2× scale preferred)
- State shown: Claude `Ok` with pro identity + at least one window row; Copilot `Ok` with at least one quota row
- Crop tightly to the menu bounds

## README Insertion

Insert immediately after the one-line tagline, before the first `##` section:

```markdown
![AIUsageBar menu screenshot](assets/demo.png)
```

No caption needed.

## Affected Files

- `assets/demo.png` — new (create `assets/` dir)
- `README.md` — one-line insertion
