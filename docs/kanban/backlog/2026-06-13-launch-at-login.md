---
id: 12
status: backlog
priority: Normal
tags: [ui, system, post-1.0, settings]
created: 2026-06-13
updated: 2026-06-17
---
# Launch at login

Toggle in menu (or settings) to register the app as a login item via `SMAppService.mainApp.register()` (macOS 13+). Table-stakes for a menu bar app — most users expect it to come back after reboot without manual action.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. `SMAppService` preferred over deprecated `SMLoginItemSetEnabled` and over raw LaunchAgent plist (no bundle install permission dialog). Requires `LSUIElement=true` already (verify). Single boolean state persisted by macOS, not by app.
- 2026-06-17: Backend already done — `launch_at_login::enable()` called at startup, writes `~/Library/LaunchAgents/com.mttpla.aiusagebar.plist` via launchctl. App already launches at login silently. What's missing is the menu toggle (enable/disable from UI). Moved post-1.0: belongs with settings UI work. `SMAppService` migration also deferred until .app bundle (#42) exists.
