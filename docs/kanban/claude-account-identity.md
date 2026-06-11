---
id: 4
status: todo
priority: Normal
tags: [claude, ui, auth]
spec: superpowers/specs/2026-06-11-claude-account-identity-design.md
created: 2026-06-11
updated: 2026-06-11
---
# Claude account identity in section header

Show the logged-in Anthropic account (email + plan) directly in the Claude section header: `Claude — mttpla@gmail.com (pro)`. On profile fetch failure: `Claude — account unavailable`.

## Narrative
- 2026-06-11: Captured from brainstorming. Access token is opaque (`sk-ant-oat01-`), not a JWT — JWT decode ruled out. Endpoint confirmed: `GET https://api.anthropic.com/api/oauth/profile`, scope `user:profile` already present in token. Email chosen over display name (unique, unambiguous). Plan derived from `has_claude_max` / `has_claude_pro` booleans. Profile merged into section header (not a separate disabled item). Section renamed "Anthropic" → "Claude". Fetch strategy: lazy one-time via `Mutex<Option<ProfileData>>`; reset on manual refresh or any `/usage` error; no retry on 401/403/429.
