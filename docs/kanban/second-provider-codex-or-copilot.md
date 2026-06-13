---
id: 14
status: backlog
priority: High
tags: [provider, scope, pre-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Ship second provider (Codex or Copilot) — or rename app

Name is "AiUsage**Bar**" plural — shipping 1.0.0 with only Claude looks like false advertising. Either implement one more provider (Codex easier, Copilot bigger user base) or rename to `ClaudeBar`/`ClaudeUsage`.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Architecture is already provider-agnostic (`UsageProvider` trait, dynamic menu). Copilot has token-priority chain documented in CLAUDE.md; Codex shares Claude's "no refresh" constraint. Decision pending: ship which, or rename. Spec/plan to be written when card moves to doing.
