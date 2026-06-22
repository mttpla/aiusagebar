# Synchronized Global Backoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace per-provider backoff with one global `BackoffState`; a single tick fetches all providers and rebuilds the full menu, so no provider silently vanishes.

**Architecture:** `App.backoff` becomes a single `BackoffState`. `refresh_all` gates once for the whole tick, fetches every provider in one pass (no per-provider skip), and aggregates the backoff decision: any 429/5xx → `on_error` (log to diag), otherwise `on_success`. Manual refresh keeps its `force=true` bypass.

**Tech Stack:** Rust, winit event loop, existing `BackoffState`, `HttpError`, `crate::diag!`.

## Global Constraints

- Claude endpoint: 180s minimum poll interval (`Settings::poll_interval` default 300s). Global timer respects it.
- Tokens are read-only; no token refresh. (Untouched here.)
- Every provider failure is a state, never a panic. One provider failing must not blank others — the full-set rebuild guarantees this.
- All `.rs` string literals in English.
- Default `pub(crate)`/private; no bare `pub`; no `#[allow(dead_code)]`.
- Run `cargo clippy -- -D warnings && cargo test` before the commit.
- Commit message: no `Co-Authored-By` trailer.

---

### Task 1: Collapse to one global BackoffState (single atomic change)

This is one task because changing `App.backoff`'s type breaks its construction and
all usages at once, and `clippy -D warnings` forbids landing an unused helper or
accessor in a separate commit. The testable logic (`should_back_off`) anchors the
TDD cycle; the wiring is verified by build + clippy + existing tests + manual
acceptance.

**Files:**
- Modify: `src/main.rs` — `App.backoff` field, imports, `refresh_all`, `about_to_wait`, `WaitUntil`, `main()` construction, add `should_back_off` + test module.
- Modify: `src/backoff.rs` — add `current_interval()` accessor.

**Interfaces:**
- Produces: `fn should_back_off(http_errs: &[Option<HttpError>]) -> bool` (private to `main.rs`).
- Produces: `BackoffState::current_interval(&self) -> std::time::Duration` (pub(crate)).
- Consumes: existing `BackoffState::{new, is_allowed, on_error, on_success, next_allowed_at}`, `HttpError::{RateLimited, ServerError}`, `crate::provider::state_diag_message`, `ui::build_menu`.

- [ ] **Step 1: Write the failing test for `should_back_off`**

Add to the bottom of `src/main.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_back_off_empty_is_false() {
        assert!(!should_back_off(&[]));
    }

    #[test]
    fn should_back_off_all_none_is_false() {
        assert!(!should_back_off(&[None, None]));
    }

    #[test]
    fn should_back_off_rate_limited_is_true() {
        assert!(should_back_off(&[None, Some(HttpError::RateLimited)]));
    }

    #[test]
    fn should_back_off_server_error_is_true() {
        assert!(should_back_off(&[Some(HttpError::ServerError(503))]));
    }

    #[test]
    fn should_back_off_unauthorized_and_other_is_false() {
        assert!(!should_back_off(&[
            Some(HttpError::Unauthorized),
            Some(HttpError::Other("dns".into())),
        ]));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test should_back_off`
Expected: FAIL — `cannot find function should_back_off in this scope`.

- [ ] **Step 3: Add the `should_back_off` helper**

Add near the top of `src/main.rs` (after the `use` block, before `struct App`):

```rust
/// True when any provider in the batch returned a 429/5xx — the only outcomes
/// that extend the global backoff. All other outcomes (network/parse error,
/// `Unauthorized`, `NotConfigured`) advance the timer normally via `on_success`.
fn should_back_off(http_errs: &[Option<HttpError>]) -> bool {
    http_errs
        .iter()
        .any(|e| matches!(e, Some(HttpError::RateLimited | HttpError::ServerError(_))))
}
```

- [ ] **Step 4: Add the `current_interval()` accessor to `src/backoff.rs`**

Insert after the existing `next_allowed_at()` accessor (around line 38):

```rust
    pub(crate) fn current_interval(&self) -> Duration {
        self.current_interval
    }
```

- [ ] **Step 5: Change the `App.backoff` field type**

In `src/main.rs`, change the field (currently line 57):

```rust
    backoff: BackoffState,
```

- [ ] **Step 6: Remove the now-unused `HashMap` import**

In `src/main.rs`, delete the line:

```rust
use std::collections::HashMap;
```

- [ ] **Step 7: Rewrite `refresh_all` gating, loop, and aggregate decision**

Replace the body of `refresh_all` from the start through the `states.push(...)` /
empty-guard region (currently lines 63-89) with:

```rust
    fn refresh_all(&mut self, force: bool) {
        if !force && !self.backoff.is_allowed() {
            return;
        }
        let count = self.providers.len();
        let mut states: Vec<(ProviderKind, UsageState)> = Vec::with_capacity(count);
        let mut http_errs: Vec<Option<HttpError>> = Vec::with_capacity(count);
        for i in 0..count {
            let kind = self.providers[i].kind();
            let (state, http_err) = self.providers[i].fetch_with_http_error();
            if let Some(msg) = crate::provider::state_diag_message(kind.display_name(), &state) {
                crate::diag!(crate::diag::Level::Err, "{}", msg);
            }
            states.push((kind, state));
            http_errs.push(http_err);
        }
        if should_back_off(&http_errs) {
            self.backoff.on_error();
            let reasons: Vec<String> = states
                .iter()
                .zip(&http_errs)
                .filter_map(|((kind, _), err)| match err {
                    Some(HttpError::RateLimited) => Some(format!("{} 429", kind.display_name())),
                    Some(HttpError::ServerError(c)) => Some(format!("{} HTTP {c}", kind.display_name())),
                    _ => None,
                })
                .collect();
            crate::diag!(
                crate::diag::Level::Err,
                "Backoff extended to {}s after {}",
                self.backoff.current_interval().as_secs(),
                reasons.join(", ")
            );
        } else {
            self.backoff.on_success();
        }
```

The remainder of `refresh_all` (building `state_refs`, `icon_kind`, `refs`,
`details_kinds`, the menu, `set_menu`, `set_icon`, `last_refreshed_at = Some(now)`)
stays exactly as it is — it already rebuilds from the full `states` vector.

- [ ] **Step 8: Update `about_to_wait` gating**

In `src/main.rs`, replace the `any_provider_ready` block (currently lines 136-141):

```rust
        let mut did_refresh = false;
        if self.backoff.is_allowed() {
            self.refresh_all(false);
            did_refresh = true;
        }
```

- [ ] **Step 9: Update the `WaitUntil` deadline**

In `src/main.rs`, replace the `next_provider` computation (currently lines 190-193):

```rust
        let next_provider = self.backoff.next_allowed_at();
```

(The `update_deadline` lines and `set_control_flow(... next_provider.min(update_deadline))` stay unchanged.)

- [ ] **Step 10: Update the `main()` construction**

In `src/main.rs`, replace the backoff construction (currently lines 237-240):

```rust
    let backoff = BackoffState::new(
        settings.poll_interval,
        settings.backoff_factor,
        settings.backoff_cap,
    );
```

The `App { ... backoff, ... }` initializer field name is unchanged.

- [ ] **Step 11: Run clippy and the full test suite**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: no warnings; all tests pass (existing suite + the 5 new `should_back_off` tests).

- [ ] **Step 12: Commit**

```bash
git add src/main.rs src/backoff.rs docs/superpowers/specs/2026-06-22-synchronized-global-backoff-design.md docs/superpowers/plans/2026-06-22-synchronized-global-backoff.md docs/kanban/sync-global-backoff.md
git commit -m "fix(backoff): one global timer, tick fetches all providers

Per-provider backoff plus any_provider_ready let one provider's poll
window drift from the other; each wake fetched only the ready provider
and rebuilt the menu from that subset, silently dropping the other.
Collapse to a single global BackoffState: one tick fetches all
providers, any 429/5xx backs off all (logged to diag), manual still
forces. Closes #52."
```

---

## Manual acceptance

With Claude + Copilot configured, run `make dev`:
- Neither provider vanishes across repeated automatic ticks (wait ≥2 poll intervals).
- Manual Refresh and automatic refresh both show the same full set.
- "Updated: HH:MM" advances once per full tick.
- Force a 429 path if feasible (or read the diag): on 429/5xx, the diag shows `Backoff extended to {secs}s after {provider} 429`.

## Self-review notes

- **Spec coverage:** global `BackoffState` (Steps 5,10); tick fetches all (Step 7); aggregate any-429/5xx → on_error else on_success (Steps 1-3,7); on_error diag (Steps 4,7); honest single timestamp / full rebuild (Step 7 keeps existing menu build); `about_to_wait`/`WaitUntil` (Steps 8,9); `force` bypass preserved (Step 7 gate). Lessons of #47/#49 preserved: `on_success` advances timer (in `backoff.rs`, untouched) and is called for all non-429/5xx outcomes (Step 7 `else`).
- **Placeholder scan:** none.
- **Type consistency:** `should_back_off(&[Option<HttpError>])`, `current_interval() -> Duration` used consistently in Steps 7 and tests.
