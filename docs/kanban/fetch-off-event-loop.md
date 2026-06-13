---
id: 16
status: backlog
priority: High
tags: [perf, event-loop, pre-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Move provider fetch off winit event loop

If `fetch()` runs synchronously on the winit thread, slow API or network stall freezes the menu. Spawn a worker thread per refresh tick, post result back via `EventLoopProxy::send_event(UserEvent::ProviderResult)`.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Verify current threading model — if already async, drop this card. winit's `WaitUntil` plus a blocking HTTP call on the main thread = visible UI freeze, especially on a Claude 401/timeout. Critical UX bug for 1.0.
