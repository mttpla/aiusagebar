# Provider error boundary diagnostics

## Problem

A provider can reach a non-happy state without leaving any entry in the in-memory
diagnostic log. The most visible instance: when the Claude OAuth token is expired,
`do_fetch` short-circuits at the `is_expired` check (`src/provider/claude.rs:215`)
and returns `UsageState::Stale` **before** making the HTTP request — so none of the
HTTP-layer `diag!` calls fire. The result is a provider visibly in error/stale in the
menu header, but an empty diagnostic log, which in turn hides the "Other ▶ Diagnostics"
submenu (it only renders when `crate::diag::is_empty()` is false, `src/ui/base.rs:17`).

The card #46 instrumentation sweep deliberately covered only the HTTP/parse error arms
of the providers and `http.rs`/`keychain.rs`; its own narrative deferred a full sweep to
post-1.0. So these silent paths are a known, deferred gap, not a regression.

The invariant we want: **every provider that ends a fetch in a non-happy state leaves a
trace in the diagnostic log.**

## Approach

Log at the boundary instead of at every leaf. After each provider fetch in
`App::refresh_all`, inspect the resulting `UsageState`: if it is `Error` or `Stale`,
push one diagnostic entry. This single choke-point catches every provider non-happy
state by construction — including the silent `is_expired` Stale path and any future
provider — without having to remember to instrument each new leaf error path.

Rejected alternative — per-leaf sweep (option A): add a `diag!` at each of the silent
sites individually. Rejected because it is fragile: every new code path must be
remembered and instrumented by hand, which is exactly how the current gap was created.

## Scope

In scope:

- A pure decision function that maps a provider name + `UsageState` to an optional
  diagnostic message.
- A single call site in `refresh_all` that pushes the message when present.

Out of scope (explicitly deferred):

- The non-provider silent paths (`launch_at_login` stderr-only failures, swallowed
  `open` command spawns, credential-file IO errors, profile-unavailable, per-item
  Keychain drops, update-check JSON parse). Tracked in a separate backlog card.
- Any fix to providers disappearing from the menu on partial auto-refresh rebuilds
  (`refresh_all` rebuilds from only the fetched subset). Separate bug, not a logging
  concern.
- Diagnostic log persistence across restarts (the log is in-memory by design).
- Any deduplication / throttling / log-on-transition machinery. Decision: keep it
  simple. The log may flood for a persistently failing provider (steady 401 or network
  outage poll every 180s). This is accepted: a full log is itself the signal that the
  underlying problem must be fixed first. Backoff already self-throttles 429/5xx, and
  the expired-token path makes no HTTP call, so the practical flood cases are limited.

## Design

### Decision function

In `src/provider/mod.rs`, next to `UsageState`:

```rust
/// Returns the diagnostic message to log for a provider that ended a fetch in a
/// non-happy state, or None for happy/neutral states (Ok, NotConfigured).
pub(crate) fn state_diag_message(name: &str, state: &UsageState) -> Option<String> {
    match state {
        UsageState::Error(msg) | UsageState::Stale(msg) => Some(format!("{}: {}", name, msg)),
        UsageState::Ok(..) | UsageState::NotConfigured => None,
    }
}
```

Both `Error` and `Stale` log at `Level::Err`: both are non-happy states the user wants
to see.

### Call site

In `App::refresh_all` (`src/main.rs`), inside the provider loop, immediately after the
state is obtained from `fetch_with_http_error` and before it is pushed into `states`:

```rust
if let Some(m) = crate::provider::state_diag_message(kind.display_name(), &state) {
    crate::diag!(crate::diag::Level::Err, "{}", m);
}
```

The `diag!` macro injects the call-site `file!():line!()` and the timestamp, so the
emitted line is e.g. `[14:03:11 ERR] [src/main.rs:NN] Claude: Token rejected — run: claude login`.

Providers skipped by backoff (`continue` on `!is_allowed`) are never fetched, so they
produce no state and are correctly not logged — nothing happened.

### What is left untouched

The existing leaf `diag!` calls in `http.rs`, `claude.rs`, `copilot.rs`, `keychain.rs`,
`clipboard.rs` stay as-is. They carry detail the boundary message lacks (URL, HTTP
status, response body, per-account context in multi-account Copilot). On HTTP errors
this means a leaf line and a boundary line per poll; that redundancy is the accepted
flood for steady 401 / network-down.

## Error handling

This feature only adds logging; it introduces no new failure modes. `state_diag_message`
is total over `UsageState` and allocates a `String` only for the two logged variants.

## Testing

Unit tests on `state_diag_message` (pure, no tray / event-loop dependency):

- `Error("boom")` → `Some("Claude: boom")`
- `Stale("expired")` → `Some("Claude: expired")`
- `Ok(_, _)` → `None`
- `NotConfigured` → `None`
- message format includes the provider name and the state message

The `refresh_all` call site itself is not unit-tested (it drives the tray); correctness
there is the trivial `if let Some` wiring around the tested function.
