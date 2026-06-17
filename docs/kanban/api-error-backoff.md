---
id: 19
status: doing
priority: High
tags: [provider, safety, claude, pre-1.0]
created: 2026-06-13
updated: 2026-06-17
spec: docs/superpowers/specs/2026-06-17-api-error-backoff.md
plan: docs/superpowers/plans/2026-06-17-api-error-backoff.md
---
# Exponential backoff on 429 / 5xx

On `HttpError::Status(429|5xx)` double the next poll interval (180s → 360s → 720s, cap 3600s). Reset to base on first success. Protects Claude's documented "180s floor or persistent ban" rule (CLAUDE.md §3) under transient API outage or accidental burst.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Without backoff, a 429 storm keeps hitting the endpoint at the floor rate and risks the documented persistent ban. Per-provider state: next_allowed_at + current_interval. Manual refresh ignores backoff (user-initiated).
- 2026-06-17: Moving to doing. Spec + plan written. Key decisions: add `ServerError(u16)` to `HttpError` to distinguish 5xx from network errors; backoff state in `HashMap<ProviderKind, BackoffState>` on `App`; providers stay stateless; `backoff_factor` and `backoff_cap` live in `Settings` for future configurability (defaults: factor=2, cap=3600s); `UsageProvider` gains `fetch_raw` + default `fetch` impl; manual refresh bypasses `is_allowed()` gate; UI unchanged.
