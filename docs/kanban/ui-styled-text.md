---
id: 8
status: todo
priority: High
tags: [ui, objc, aesthetics]
blocked_by: [7]
spec: superpowers/specs/2026-06-12-ui-styled-text.md
created: 2026-06-12
updated: 2026-06-12
---
# UI styled text

Apply `NSAttributedString` to provider headers (brand colors, bold) and footer (Refresh tab-stop, Quit red). Medium ObjC2 — only `setAttributedTitle:`, no custom NSView.

## Narrative
- 2026-06-12: Split from archived card #6. Covers Claude orange `#C9551E`, Copilot purple `#6E40C9`, Refresh blue `#147EFB` with right-aligned tab stop at 290pt, Quit red `#FF3B30`. Blocked by #7 (src/ui/ must exist first).
