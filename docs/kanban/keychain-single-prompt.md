---
id: 39
status: backlog
priority: Normal
tags: [keychain, ux, post-1.0]
spec: specs/2026-06-15-keychain-single-prompt-design.md
plan: plans/2026-06-15-keychain-single-prompt.md
created: 2026-06-15
updated: 2026-06-18
---
# Cache Copilot account name to skip Keychain enumerate (3→2 prompts)

Copilot path calls `enumerate_generic_passwords` (1 search prompt) then `get_generic_password` per account (1 prompt each) = 2 prompts for 1 account. Claude path is 1 prompt (known account, direct lookup). Total on first run: 3.

`load_data(true)` approach failed: macOS silently omits `v_Data` for third-party ACL items (`copilot-cli`), so the enumerate returns empty. Cannot get below 2 total (Claude and Copilot are separate Keychain services — minimum 1 prompt each).

Cache approach: after first successful Copilot read, persist account name to disk (`~/.cache/aiusagebar/copilot-account`). On subsequent runs, skip enumerate and call `read_generic_password` directly → 2 prompts on first run, 2 on subsequent (but one disappears after "Always Allow"). Gain only materialises from second run onward.

## Narrative
- 2026-06-16: Reverted `load_data(true)` attempt — macOS silently returns no data for third-party ACL items (copilot-cli), causing the search to return empty via `unwrap_or_default()`. Back to 3 prompts (1 Claude + 1 Copilot search + 1 Copilot password read). Alternative to explore: cache account names on disk to skip the search and eliminate the extra prompt.
- 2026-06-15: Captured from investigation into why user sees 3 keychain prompts on startup (1 Claude + 2 Copilot for a single Copilot account). Root cause: `get_generic_password` called separately per account after the search. Fix attempted: `load_data(true)` on `ItemSearchOptions`, read "v_Data" from result dict directly.
- 2026-06-18: Reclassified post-1.0. `load_data(true)` confirmed dead end (ACL blocks data for third-party items). Real gain is 3→2, only from second run onward. After "Always Allow" all prompts vanish anyway — UX impact low. Renamed card to reflect actual scope.
