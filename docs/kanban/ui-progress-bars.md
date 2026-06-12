---
id: 9
status: todo
priority: Normal
tags: [ui, objc, aesthetics]
blocked_by: [8]
spec: superpowers/specs/2026-06-12-ui-progress-bars.md
created: 2026-06-12
updated: 2026-06-12
---
# UI progress bar rows

Replace plain-text window rows with custom `NSView` items: label + pct + 4pt NSBox bar colored by threshold + detail line. High ObjC2 complexity — isolated, can be deferred without breaking #7 or #8.

## Narrative
- 2026-06-12: Split from archived card #6. Green/amber/red thresholds at <60/60-80/>80%. NSBox for bar fill. resets_at formatting: relative for 5h session, absolute date for 7d weekly. Isolated by design so it can slip without blocking other work. Blocked by #8.
