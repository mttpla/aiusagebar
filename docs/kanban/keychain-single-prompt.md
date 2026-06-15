---
id: 39
status: backlog
priority: High
tags: [keychain, ux]
spec: specs/2026-06-15-keychain-single-prompt-design.md
plan: plans/2026-06-15-keychain-single-prompt.md
created: 2026-06-15
updated: 2026-06-15
---
# Reduce keychain prompts to one per service

Currently `enumerate_generic_passwords` triggers N+1 macOS prompts per service (1 search + 1 per account). Fix by using `load_data(true)` in the search so password bytes are returned inline — single prompt regardless of account count.

## Narrative
- 2026-06-16: Reverted `load_data(true)` attempt — macOS silently returns no data for third-party ACL items (copilot-cli), causing the search to return empty via `unwrap_or_default()`. Back to 3 prompts (1 Claude + 1 Copilot search + 1 Copilot password read). Alternative to explore: cache account names on disk to skip the search and eliminate the extra prompt.
- 2026-06-15: Captured from investigation into why user sees 3 keychain prompts on startup (1 Claude + 2 Copilot for a single Copilot account). Root cause: `get_generic_password` called separately per account after the search. Fix: `load_data(true)` on `ItemSearchOptions`, read "v_Data" from result dict directly. Must ship before v1.0 — repeated prompts are confusing UX.
