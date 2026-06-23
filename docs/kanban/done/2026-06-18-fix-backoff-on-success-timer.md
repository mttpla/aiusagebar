---
id: 47
status: done
priority: High
tags: [bug, backoff, polling, claude]
created: 2026-06-18
updated: 2026-06-21
---
# Fix: BackoffState::on_success does not advance next_allowed_at

`on_success` resets `current_interval` but leaves `next_allowed_at` at the startup instant (always in the past). After every successful poll `is_allowed()` immediately returns true → `about_to_wait` wakes up instantly → continuous rapid polling → Anthropic issues a persistent 429 on the token. The next app session starts with an empty `last_ok` cache, hits that persistent 429, and shows "Rate limited (no cache)".

**Fix:** one line in `src/backoff.rs` `on_success`:
```rust
self.next_allowed_at = Instant::now() + base;
```

## Narrative
- 2026-06-18: Diagnosed from live "Rate limited (no cache)" report on v0.4.0. Root cause: `on_success` (`src/backoff.rs:16`) only resets `current_interval`; `on_error` correctly updates both fields but `on_success` never did. Backoff card #19 introduced the struct but the success path was incomplete. Fix is trivial (one line) but the missing update also breaks the existing `on_success_resets_interval` test assertion about `current_interval` — that test should be extended to also assert `next_allowed_at` is ~`now + base`.
- 2026-06-21: Fix confirmed in code (commit e73a21e). `backoff.rs:18` sets `next_allowed_at = Instant::now() + base`. Card moved to done.
