---
id: 23
status: backlog
priority: Normal
tags: [perf, ux, post-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Refresh on network reachability change

`SCNetworkReachability` callback on `api.anthropic.com` reachable state transition → immediate refresh instead of waiting up to 180s after Wi-Fi reconnect.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Pairs with card #22 (sleep/wake) but distinct trigger — wifi flip without sleep is common (commute, café). `system-configuration` crate. Post-1.0 because acceptable UX without it; 180s worst case.
