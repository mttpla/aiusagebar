---
id: 29
status: backlog
priority: Normal
tags: [auth, claude, post-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Detect Keychain token rotation

Cached `ProfileData` and access token currently stick until error (card #4 narrative). If Claude Code app rotates token mid-session (refresh by official client), our cached token may go stale silently. Re-read Keychain on each fetch and drop profile cache if access token differs.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Keychain read is cheap; first-read dialog already accepted. Hard constraint #2 (read-only) preserved — we only re-read, never write. Post-1.0 — current 401 → `Stale` path handles the symptom acceptably.
