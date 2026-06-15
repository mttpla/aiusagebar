---
id: 38
status: done
priority: Normal
tags: [copilot, ui, bug]
spec: specs/2026-06-15-copilot-multi-account-error-display-design.md
plan: plans/2026-06-15-copilot-multi-account-error-display.md
created: 2026-06-15
updated: 2026-06-15

---
# Fix Copilot multi-account error display

Two bugs in Copilot multi-account rendering.

## Narrative
- 2026-06-15: Captured from brainstorming. Two distinct bugs:
  1. Header always shows "account unavailable" even when all accounts work — `Ok(_, None)` in `header_label` (ui/copilot.rs:8) renders that suffix unconditionally, but for Copilot `None` is the normal state (no single profile, unlike Claude).
  2. Sentinel error rows don't show the username of the failing account — `load_copilot_tokens()` discards the keychain `account` field, so 401 rows say only "GitHub — token expired, re-login" with no identity.
  Fix approach: (1) `load_copilot_tokens` → `Vec<(String, String)>` (keychain_account, token); (2) `do_copilot_fetch` uses keychain_account in sentinel LimitWindow names; (3) `header_label` `Ok(_, None)` case shows just the provider name, no suffix. Files: src/provider/copilot.rs, src/ui/copilot.rs.
- 2026-06-15: Implemented. All 116 tests pass. Done.
