# Synchronized global backoff — one tick fetches all providers

Date: 2026-06-22

## Problem

The Claude provider silently disappears from the menu on automatic refresh, but
reappears on manual refresh. The diagnostic log is empty (no 429, no 5xx, no
parse error, no `Error`/`Stale` state), so the provider is **not** failing — its
fetches succeed every time they run.

## Root cause

Per-provider backoff (`HashMap<ProviderKind, BackoffState>`) combined with two
mechanics in `main.rs`:

1. `about_to_wait` triggers `refresh_all(false)` the instant **any** provider's
   backoff window is allowed (`any_provider_ready`) — it never waits for all.
2. `refresh_all` skips not-allowed providers with a bare `continue`, then rebuilds
   the whole menu from **only the fetched subset**. There is no retained state.

`on_success` sets `next_allowed_at = now + base` evaluated *after* each fetch
returns. In one cycle the two providers are fetched sequentially, so their windows
end up permanently offset by ~one fetch duration. From then on each wake fires for
the earlier provider only, fetches it, and drops the other from the rebuilt menu —
**ping-pong**, with no error involved. Manual refresh uses `force=true`, fetches
all, so both reappear briefly.

This regression was introduced by the per-provider backoff feature (commit
`a1a2110`); the pre-backoff `refresh()` fetched all providers every cycle and
could never drop one. `http.rs` was never touched by that commit and is not at
fault.

## Design

Return to a single synchronized tick — the pre-backoff loop behavior — with one
app-wide backoff timer layered on top.

### Global backoff timer

- `App.backoff: BackoffState` — a single instance, replacing
  `HashMap<ProviderKind, BackoffState>`.
- Constructed in `main()` as
  `BackoffState::new(settings.poll_interval, settings.backoff_factor, settings.backoff_cap)`.

### One tick fetches all

`refresh_all(force)`:

1. Gate once for the whole tick: `if !force && !self.backoff.is_allowed() { return; }`.
2. Loop over **all** providers, fetch each (no per-provider gate, no `continue`).
   Collect `(kind, state, http_err)` for every provider.
3. Aggregate the backoff decision:
   - If **any** provider returned `HttpError::RateLimited | ServerError(_)` →
     `self.backoff.on_error()`.
   - Otherwise → `self.backoff.on_success()`.
4. Rebuild the menu and icon from **all** providers' states (always the full set).
5. Set `last_refreshed_at = now`.

`force=true` (manual refresh) bypasses the gate, unchanged.

This preserves the lessons of cards #47 and #49: `on_success` advances
`next_allowed_at` (no tight loop after success), and a tick that returns
non-429/5xx errors (network/parse/`Unauthorized`/expired) still calls
`on_success`, so the timer always advances — no tight poll loop.

### Trade-off (accepted)

A 429/5xx on one provider now delays the *next* refresh of all providers (shared
timer). The other provider keeps showing its last good info; only refresh cadence
slows. This matches the documented intent in CLAUDE.md of a single global
≥180s poll interval — the per-provider split was the drift.

### `on_error` diagnostic

When the aggregate decision is `on_error`, write one diagnostic line recording the
backoff extension and the offending provider(s)/status, e.g.:

```
Backoff extended to {secs}s after {provider} {429|5xx}
```

Per-provider `Error`/`Stale` boundary logging via `state_diag_message`
(card #50) is unchanged.

### Timestamp ("Updated: HH:MM")

Keep the single existing "Updated: HH:MM" menu item. Because only a real full
tick bumps `last_refreshed_at`, the timestamp is now honest — the old desync bumped
it on single-provider refreshes. The menu shows the latest available info for every
provider (cached `last_ok` is already returned by `claude.rs` on 429). **No**
per-provider data-age annotation is added — keep it simple; the single timestamp is
consistent with the shown info.

### `about_to_wait` and `WaitUntil`

- Replace `any_provider_ready` with `if self.backoff.is_allowed()`.
- `WaitUntil` deadline becomes `self.backoff.next_allowed_at()` min the update-check
  deadline (no more `HashMap::values().map(..).min()`).

## Touch points

- `src/main.rs` — `App.backoff` field type; `refresh_all`; `about_to_wait`;
  `WaitUntil` computation; `main()` construction.
- `src/backoff.rs` — unchanged (already owns base/factor/cap).
- `src/settings.rs` — unchanged.
- UI — unchanged (full-set rebuild already supported; no new annotation).

## Out of scope

- Per-provider data-age annotation in the menu.
- Retained-state (`last_states`) caching — unnecessary once the tick fetches all.
- Falling back to `last_ok` for `Other`/network errors — no error is involved in
  this bug.
- Moving fetch off the event loop (card #16, separate).
- Independent per-provider backoff cadence (possible future change).

## Testing

- `BackoffState` unit tests unchanged (already cover advance/double/cap).
- New: `refresh_all` aggregate rule — any 429/5xx in the batch → `on_error`;
  all-clear → `on_success`. (Extract the aggregate decision into a pure helper if
  needed for testability.)
- Manual acceptance: with Claude + Copilot configured, neither provider vanishes
  across repeated automatic ticks; "Updated" advances only per full tick.

## Card conflict review

- **#19 api-error-backoff (done)** — superseded in granularity only: its
  `HashMap<ProviderKind, BackoffState>` / "per-provider state" decision is replaced
  by a single global `BackoffState`. The purpose (protect Claude's 180s floor on
  429/5xx) is preserved.
- **#47 fix-backoff-on-success-timer (done)** — preserved. `on_success` must still
  advance `next_allowed_at`; lives in `backoff.rs`, unaffected by going global.
- **#49 fix-backoff-tight-loop-on-error (done)** — preserved. The aggregate rule
  calls `on_success` for all non-429/5xx outcomes, so the global timer always
  advances. No tight loop.
- **#1 polling-mechanism-and-settings (done)** — compatible. Single "Updated" item
  and "manual resets countdown" both still hold.
- **#16 fetch-off-event-loop (backlog)** — neutral. The tick fetches all providers
  sequentially on the event loop, which is the original pre-backoff blocking
  profile #16 already accounts for. No conflict.
- **#23 network-reachability-refresh, #26 skip-refresh-display-asleep, sleep/wake
  (backlog)** — compatible; event-triggered refreshes drive the single global tick.
- **#50 provider-error-boundary-diag / #51 non-provider-diag-gaps (done)** —
  complementary. The new `on_error` diagnostic adds to boundary logging.

Only card **#19** is in opposition (granularity), and it is historical/done.
```
