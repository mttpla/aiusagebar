---
id: 13
status: backlog
priority: High
tags: [ui, ux, pre-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Onboarding / NotConfigured empty state

First-run with no Claude token currently renders a silent `NotConfigured`. Replace with a clear actionable row: `Claude — not signed in · Setup…` opening README/docs URL. Same pattern for future providers.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Without this, a fresh install looks broken. UI module already renders per-provider sections (card #7), so the empty state lives in `src/ui/claude.rs` (and per-provider files). Click action: `open` the setup section of the GitHub README anchor. No new ObjC2.
