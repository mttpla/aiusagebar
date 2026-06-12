---
id: 8
status: done
priority: High
tags: [ui, objc, aesthetics]
spec: docs/superpowers/specs/2026-06-12-ui-styled-text.md
plan: docs/superpowers/plans/2026-06-12-ui-styled-text.md
created: 2026-06-12
updated: 2026-06-12
---
# UI styled text

Apply `NSAttributedString` to provider headers (brand colors, bold) and footer (Refresh tab-stop, Quit red). Medium ObjC2 — only `setAttributedTitle:`, no custom NSView.

## Narrative
- 2026-06-12: Split from archived card #6. Covers Claude orange `#C9551E`, Copilot purple `#6E40C9`, Refresh blue `#147EFB` with right-aligned tab stop at 290pt, Quit red `#FF3B30`. Blocked by #7 (src/ui/ must exist first).
- 2026-06-12: Plan written (docs/superpowers/plans/2026-06-12-ui-styled-text.md). Moved to doing. Key design: build_layout extracted from build_menu for testable index tracking; style_menu uses msg_send! for cross-version ObjC API stability.
- 2026-06-13: Implementation complete. Merged to master (02a630a). 89 tests pass. Notable: objc2 0.6 drops trailing underscores on single-arg selectors; typed bindings used throughout (no msg_send! needed); AnyThread + ContextMenu imports both load-bearing.
