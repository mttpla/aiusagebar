---
id: 25
status: backlog
priority: Normal
tags: [ui, ux, post-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Click row → copy to clipboard

Clicking a usage row (e.g. `Session 67%`) copies a formatted snippet: `Claude session: 67% used, resets 14:32` into pasteboard via `NSPasteboard.generalPasteboard`. Useful for status updates in Slack/standups.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Subtle UX feature — discoverable via tooltip "Click to copy". May conflict with card #9 progress-bar rows (custom `NSView` event handling). Post-1.0 — wait for #9 to land first.
