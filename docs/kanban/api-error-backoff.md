---
id: 19
status: backlog
priority: High
tags: [provider, safety, claude, pre-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Exponential backoff on 429 / 5xx

On `HttpError::Status(429|5xx)` double the next poll interval (180s → 360s → 720s, cap 3600s). Reset to base on first success. Protects Claude's documented "180s floor or persistent ban" rule (CLAUDE.md §3) under transient API outage or accidental burst.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Without backoff, a 429 storm keeps hitting the endpoint at the floor rate and risks the documented persistent ban. Per-provider state: next_allowed_at + current_interval. Manual refresh ignores backoff (user-initiated).
