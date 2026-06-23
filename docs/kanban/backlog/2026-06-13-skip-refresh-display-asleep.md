---
id: 26
status: backlog
priority: Normal
tags: [perf, power, post-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Skip refresh when display is asleep

`CGDisplayIsAsleep(CGMainDisplayID())` check before each scheduled fetch. If user closed lid or display dimmed via Hot Corner, skip the tick — they cannot see the menu anyway. Critical for clamshell/battery use.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Overlaps with card #22 (sleep/wake) but display-sleep ≠ system-sleep. Combined effect: zero background work on a clamshelled laptop. Post-1.0.
