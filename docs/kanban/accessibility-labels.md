---
id: 30
status: backlog
priority: Normal
tags: [a11y, ui, post-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# VoiceOver accessibility labels

`NSAttributedString` brand colors (card #8) and custom `NSView` progress bars (card #9) can confuse VoiceOver — color is non-semantic and custom views announce as "image". Add `setAccessibilityLabel:` plaintext on each menu item: `"Claude session usage 67 percent, resets at 14:32"`.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Blocked by #9 (custom NSView rows). Audit pass after #9 lands. Post-1.0 — important but not blocking initial release.
