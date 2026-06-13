---
id: 12
status: backlog
priority: High
tags: [ui, system, pre-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Launch at login

Toggle in menu (or settings) to register the app as a login item via `SMAppService.mainApp.register()` (macOS 13+). Table-stakes for a menu bar app — most users expect it to come back after reboot without manual action.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. `SMAppService` preferred over deprecated `SMLoginItemSetEnabled` and over raw LaunchAgent plist (no bundle install permission dialog). Requires `LSUIElement=true` already (verify). Single boolean state persisted by macOS, not by app.
