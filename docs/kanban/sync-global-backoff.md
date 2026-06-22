---
id: 52
status: doing
priority: High
tags: [bug, backoff, polling, claude, pre-1.0]
spec: docs/superpowers/specs/2026-06-22-synchronized-global-backoff-design.md
plan: docs/superpowers/plans/2026-06-22-synchronized-global-backoff.md
created: 2026-06-22
updated: 2026-06-22
---
# Synchronized global backoff — one tick fetches all providers

Claude vanishes from the menu on automatic refresh but returns on manual refresh,
with an empty diagnostic log. Replace per-provider backoff with a single global
`BackoffState`: one tick fetches all providers, any 429/5xx backs off all, manual
still forces. Fixes the silent vanish and makes the "Updated" timestamp honest.

## Narrative
- 2026-06-22: Captured from debugging session. Root cause confirmed by code +
  git diff: per-provider `HashMap<ProviderKind, BackoffState>` plus
  `any_provider_ready` triggering `refresh_all(false)` for whichever provider is
  ready first, plus a menu rebuilt from only the fetched subset (bare `continue`
  skip, no retained state). `on_success` sets `next_allowed_at = now + base`
  *after* each fetch, so the two providers' windows drift apart by ~one fetch
  duration and stay offset forever — each wake fetches one provider and drops the
  other. Ping-pong, no error, empty diag. Manual refresh (`force=true`) fetches
  all, so both reappear. Regression introduced by backoff commit `a1a2110`;
  `http.rs` untouched and not at fault. The DNS error seen once is an unrelated
  environmental blip that self-heals.
- 2026-06-22: Design chosen — single global `BackoffState` (reverts the
  per-provider HashMap of card #19, granularity only; purpose preserved). Tick
  fetches all providers once and rebuilds the full menu (no vanish, no retained
  state needed). Aggregate backoff: any provider 429/5xx → `on_error` (back off
  all) else `on_success`. Manual refresh keeps `force=true` bypass. `on_error`
  writes a diagnostic line. Single honest "Updated: HH:MM" (only a full tick bumps
  it); show latest available info incl. cached `last_ok` on 429; no per-provider
  data-age annotation (kept simple).
- 2026-06-22: Rejected options — (a) `last_states` per-provider retention with
  per-provider gating: rejected, fetch-all makes it unnecessary; (b) per-provider
  "(data HH:MM)" annotation: rejected for simplicity; (c) `Other`/network error
  fallback to `last_ok`: out of scope, no error involved in this bug; (d) removing
  the `force` param: rejected — `force` correctly bypasses backoff for
  user-initiated refresh; removing it either breaks manual refresh or kills
  backoff's 429 protection (CLAUDE.md §3).
- 2026-06-22: Trade-off accepted — a 429/5xx on one provider delays the next
  refresh of all (shared timer); the other keeps its last good info, only cadence
  slows. Matches CLAUDE.md's documented single global ≥180s poll interval.
- 2026-06-22: Card conflict review — only #19 (api-error-backoff) is in opposition
  and only on granularity (HashMap → global); it is done/historical. #47
  (on_success advances timer) and #49 (advance on all non-429/5xx) are preserved
  by the aggregate rule. #1, #16, #23, #26, #50, #51 are compatible/complementary.
- 2026-06-22: Spec written and linked. Single card (no split). Next: writing-plans.
- 2026-06-22: Plan written + linked. Implemented on branch `fix/sync-global-backoff`:
  `App.backoff` is now a single `BackoffState`; `refresh_all` gates once, fetches
  all providers, aggregates via `should_back_off` (any 429/5xx → `on_error` + diag
  line `Backoff extended to {secs}s after {provider} {429|HTTP n}`, else
  `on_success`); `about_to_wait` and `WaitUntil` use the single timer; `HashMap`
  import removed; `BackoffState::current_interval()` accessor added. 5 new
  `should_back_off` unit tests. `cargo clippy -- -D warnings` clean, 196 tests pass.
  Pending: manual acceptance (`make dev`, GUI) before moving to done.
