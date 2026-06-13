---
id: 27
status: backlog
priority: Normal
tags: [perf, ui, post-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Cache tray icon bitmap per state class

`tray-icon::Icon::set_icon` currently re-decoded/re-uploaded each refresh. Memoize three pre-loaded `Icon` instances (normal, alert, error) and call `set_icon` only when the state class actually changes vs last call.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Verify whether current code already short-circuits — if yes, drop card. Otherwise small CPU + main-thread time saved every 180s. Pairs with dynamic-icon feature (see memory `project_dynamic_icon.md`). Post-1.0.
