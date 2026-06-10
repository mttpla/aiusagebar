---
id: 1
status: todo
priority: High
tags: [polling, settings, event-loop]
spec: docs/superpowers/specs/2026-06-10-polling-mechanism-design.md
created: 2026-06-10
updated: 2026-06-10
---
# Polling mechanism & Settings struct

Introduce automatic background polling via `ControlFlow::WaitUntil` in the winit
event loop, a central `Settings` struct for runtime constants, and a last-refresh
timestamp displayed in the tray menu.

## Narrative
- 2026-06-10: Captured from brainstorming. Approach A chosen (WaitUntil in main
  loop, single-threaded) over background thread or NSTimer — no concurrency needed
  at 5-min intervals. Settings struct with Default chosen over bare consts so that
  adding `#[derive(Serialize, Deserialize)]` + a `load()` is enough to enable JSON
  persistence later. Menu/settings UI and JSON persistence are explicitly out of
  scope. `last_refreshed_at` (Option<DateTime<Local>>) added to App; shown as a
  disabled gray menu item "Updated: HH:MM" before Refresh/Quit. Manual Refresh
  resets the countdown. Spec at docs/superpowers/specs/2026-06-10-polling-mechanism-design.md.
