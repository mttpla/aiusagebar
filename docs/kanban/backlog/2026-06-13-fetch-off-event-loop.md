---
id: 16
status: backlog
priority: High
tags: [perf, event-loop, post-1.0]
created: 2026-06-13
updated: 2026-06-17
---
# Move provider fetch off winit event loop

If `fetch()` runs synchronously on the winit thread, slow API or network stall freezes the menu. Spawn a worker thread per refresh tick, post result back via `EventLoopProxy::send_event(UserEvent::ProviderResult)`.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Verify current threading model — if already async, drop this card. winit's `WaitUntil` plus a blocking HTTP call on the main thread = visible UI freeze, especially on a Claude 401/timeout. Critical UX bug for 1.0.
- 2026-06-17: Confirmed fetch() runs on winit thread (main.rs:56, inside about_to_wait). HTTP client is ureq with 15s global timeout. Risk reassessed as low: menu bar interaction is handled by the OS, not winit window events, so freeze only affects manual Refresh coinciding with auto-poll. Thread complexity (EventLoopProxy<UserEvent> + channel + panic handling) not justified pre-1.0. Moved to post-1.0.
