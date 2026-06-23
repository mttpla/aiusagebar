---
id: 21
status: backlog
priority: Normal
tags: [ux, notifications, post-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# UserNotification on threshold crossing

When any window crosses `ALERT_THRESHOLD` (default 80%) upward, fire a macOS user notification: `Claude session at 82% — resets 14:32`. Suppress duplicate fires within the same window period. Dynamic icon already turns amber; this adds active push.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Use `objc2-user-notifications` or `mac-notification-sys`. Requires "Allow notifications" consent on first fire. Per-provider per-window debounce key. Post-1.0 because icon already covers passive signal.
