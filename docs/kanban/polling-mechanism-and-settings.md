---
id: 1
status: done
priority: High
tags: [polling, settings, event-loop]
spec: docs/superpowers/specs/2026-06-10-polling-mechanism-design.md
plan: docs/superpowers/plans/2026-06-11-polling-mechanism-settings.md
created: 2026-06-10
updated: 2026-06-11
completed: 2026-06-11
---
# Polling mechanism & Settings struct

Introduce automatic background polling via `ControlFlow::WaitUntil` in the winit
event loop, a central `Settings` struct for runtime constants, and a last-refresh
timestamp displayed in the tray menu.

## Narrative
- 2026-06-11: Merged to master (de6c67c). 6 commits: Settings struct, ALERT_THRESHOLD migration, build_menu timestamp slot, last_refreshed_at wiring, WaitUntil polling, fix for dead alert_threshold_pct field + double-refresh guard. 56 tests pass. Final review found and fixed: alert_threshold_pct wired through for_state/for_providers params; did_refresh flag in about_to_wait.
- 2026-06-11: Implementation plan written at docs/superpowers/plans/2026-06-11-polling-mechanism-settings.md. 5 tasks: settings.rs, ALERT_THRESHOLD migration, build_menu timestamp slot, last_refreshed_at wiring, about_to_wait WaitUntil rewrite. Separator confirmed in menu by user.
- 2026-06-10: Captured from brainstorming. Approach A chosen (WaitUntil in main
  loop, single-threaded) over background thread or NSTimer — no concurrency needed
  at 5-min intervals. Settings struct with Default chosen over bare consts so that
  adding `#[derive(Serialize, Deserialize)]` + a `load()` is enough to enable JSON
  persistence later. Menu/settings UI and JSON persistence are explicitly out of
  scope. `last_refreshed_at` (Option<DateTime<Local>>) added to App; shown as a
  disabled gray menu item "Updated: HH:MM" before Refresh/Quit. Manual Refresh
  resets the countdown. Spec at docs/superpowers/specs/2026-06-10-polling-mechanism-design.md.
