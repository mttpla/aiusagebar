---
id: 49
status: done
priority: High
tags: [bug, backoff, polling, pre-1.0]
created: 2026-06-21
updated: 2026-06-21
---
# Fix: backoff tight loop on Error/Stale without 429/5xx

After a successful poll `next_allowed_at = now + 300s`. Five minutes later the
auto-poll fires and returns any non-Ok, non-429, non-5xx outcome (network
timeout, parse error, `Unauthorized`, expired token, malformed credentials). The
`_ =>` branch in `refresh_all` only calls `on_success` when `state == Ok`, so
`next_allowed_at` is never advanced. On the next `about_to_wait` tick
`is_allowed()` is immediately true → tight poll loop → eventual persistent 429
→ `Error("Rate limited (no cache)")`. Manual refresh breaks the loop only if the
fetch succeeds.

**Fix:** drop the `Ok`-guard in `main.rs` — always call `on_success` when there
is no 429/5xx signal, regardless of state:

```rust
match http_err {
    Some(HttpError::RateLimited | HttpError::ServerError(_)) => {
        b.on_error(self.settings.backoff_factor, self.settings.backoff_cap);
    }
    _ => {
        b.on_success(self.settings.poll_interval);
    }
}
```

## Narrative
- 2026-06-21: Diagnosed while reviewing backoff system. Complements card #47
  (on_success fix). That fix prevented tight loops after success; this fixes
  tight loops after Error/Stale. Root: `main.rs` only called `on_success` inside
  `if matches!(state, Ok)`, leaving all error paths without a `next_allowed_at`
  advance. Confirmed by code reading — no test covered this path.
- 2026-06-21: Fixed. Dropped `if matches!(state, Ok)` guard — `on_success` now
  called for all non-429/5xx outcomes. 168/168 tests pass.
