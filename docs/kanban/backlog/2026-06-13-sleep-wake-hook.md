---
id: 22
status: backlog
priority: Normal
tags: [perf, power, post-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Sleep/wake polling pause

Observe `NSWorkspace.sharedWorkspace.notificationCenter` `NSWorkspaceWillSleepNotification` → pause polling, `NSWorkspaceDidWakeNotification` → immediate refresh + restart timer. Saves battery and API quota across laptop lid-close, gives fresh data the instant the user reopens.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Without this, the 180s timer may fire seconds after wake (stale) or accumulate missed ticks. ObjC2 observer plus `EventLoopProxy` signal back to winit. Post-1.0 because most users keep machines awake during work hours; nice-to-have polish.
