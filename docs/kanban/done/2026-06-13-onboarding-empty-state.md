---
id: 13
status: done
priority: High
tags: [ui, ux, pre-1.0]
created: 2026-06-13
updated: 2026-06-17
spec: superpowers/specs/2026-06-17-onboarding-empty-state-design.md
plan: superpowers/plans/2026-06-17-onboarding-empty-state.md
---
# Onboarding / NotConfigured empty state

First-run with no Claude token currently renders a silent `NotConfigured`. Replace with a clear actionable row: `Claude — not signed in · Setup…` opening README/docs URL. Same pattern for future providers.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Without this, a fresh install looks broken. UI module already renders per-provider sections (card #7), so the empty state lives in `src/ui/claude.rs` (and per-provider files). Click action: `open` the setup section of the GitHub README anchor. No new ObjC2.
- 2026-06-17: Expanded scope to include Copilot. Decided against README anchors — dedicated pages at repo root instead: `claude-setup.md` and `copilot-setup.md` (lowercase kebab; not in docs/ which is internal). Spec written.
- 2026-06-17: Implemented via SDD (4 tasks). Final review clean — ready to merge. Pre-existing minor: `display_name()` for Copilot returns "Copilot" not "GitHub Copilot"; tests hardcode the string so they pass but don't match runtime label. Not introduced by this card.
