---
id: 6
status: archive
priority: High
tags: [ui, aesthetics, objc]
spec: superpowers/specs/2026-06-11-ui-aesthetics-design.md
created: 2026-06-11
updated: 2026-06-12
---
# UI aesthetics redesign

Full visual overhaul of the menu and about dialog. Brand-colored provider headers, per-window progress bars, restructured footer, `src/ui/` module folder, README screenshot.

## Narrative
- 2026-06-12: Archived. Card was too large to implement safely. Split into card #7 (module restructure), #8 (styled text), #9 (progress bars), #10 (README screenshot). Original spec `2026-06-11-ui-aesthetics-design.md` kept as reference.
- 2026-06-11: Captured from brainstorming session. Design C chosen (custom NSView progress bars). Decisions: Claude orange `#C9551E`, Copilot purple `#6E40C9`, bars colored green/amber/red by threshold (<60/60-80/>80%). Provider header uses `●` Unicode + NSAttributedString brand color + inline identity suffix (gray, from card #4). Copilot rows: flat `login / quota_type` label + remaining count. Footer: ↺ Refresh + Updated 14:32 on one row (NSAttributedString tab stop), then ℹ About and ✕ Quit as left-aligned NSMenuItems. About dialog: NSAlert unchanged, adds icon from card #5. File structure: new `src/ui/` with `mod.rs`, `base.rs`, `claude.rs`, `copilot.rs`. README: screenshot from visual companion (stato normale panel) committed as `assets/demo.png`, inserted before first ## section. Rejected: NSPanel custom about (too much ObjC), right-aligned About/Quit row (requires NSView for two independent actions), account sub-header for Copilot (flat label preferred).
