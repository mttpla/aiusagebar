# Remove duplication across http, menu actions, and details lookup

## Problem

Three independent spots repeat logic that should live in one place. All pre-date this
review; none is a regression. Pure cleanup — no observable behavior change.

1. **HTTP status mapping is duplicated.** `get` (`src/http.rs:45`) and `get_public`
   (`src/http.rs:74`) each inline the same status table:
   `401 → Unauthorized`, `429 → RateLimited`, `500..=599 → ServerError(c)`,
   other → `Other(format!("HTTP {code}"))`. Two copies drift apart over time (e.g. a new
   status arm added to one and not the other).

2. **"open URL" menu action is repeated four times.** In `App::about_to_wait`
   (`src/main.rs`, ~176–189) the update, setup_claude, setup_copilot, and releases
   branches each spell out `std::process::Command::new("open").arg(url).spawn()` with a
   `diag!`-on-error tail. Same five lines, four times.

3. **`details_kinds` build is O(n²) with repeated locking.** `App::refresh_all`
   (`src/main.rs:111`) computes the set of provider kinds that have raw JSON by, for each
   ref, re-scanning `self.providers` with `.any(|p| p.kind() == *k && p.raw_json().is_some())`.
   That locks the `raw_json` mutex once per inner hit. With two providers it is harmless
   at runtime, but it is the wrong shape and re-locks needlessly.

## Approach

Extract each repeated fragment to a single definition; leave every call site behaviorally
identical.

1. A pure `fn classify(status: u16) -> Option<HttpError>` in `http.rs`, returning `None`
   for 200 and `Some(err)` for every error status. Both `get` and `get_public` call it and
   keep their own (different) body-reading paths.
2. A private `fn open_url(url: &str)` in `main.rs` that spawns `open` and logs failure via
   `diag!`. Each of the four branches calls it.
3. Replace the nested `.any()` scan with a single pass over `self.providers` that collects
   the kinds with raw JSON, locking each provider's `raw_json` at most once.

Rejected alternative: leaving items 2 and 3 as-is (they are tiny at n=2). Rejected because
the cost to fix is one helper each and the result is unambiguously clearer; the card
already groups them.

## Scope

In scope:

- `classify` helper + its two call sites in `http.rs`.
- `open_url` helper + its four call sites in `main.rs`.
- Single-pass `details_kinds` construction in `refresh_all`.

Out of scope:

- Any change to HTTP error semantics, status arms, or which errors trigger backoff.
- Any change to what URLs the menu opens or when.
- Moving fetches off the event loop (separate 🔴 concern, not tracked here).
- The redundant body clone in `get` (`http.rs:46`) — that is an idiom fix, tracked in the
  idiomatic-Rust card (#54), not here.

## Design

### 1. `classify` (http.rs)

```rust
/// Maps an HTTP status to its error, or `None` for 200 OK.
fn classify(status: u16) -> Option<HttpError> {
    match status {
        200 => None,
        401 => Some(HttpError::Unauthorized),
        429 => Some(HttpError::RateLimited),
        c @ 500..=599 => Some(HttpError::ServerError(c)),
        code => Some(HttpError::Other(format!("HTTP {code}"))),
    }
}
```

`get` becomes (body-read path preserved):

```rust
let status = resp.status().as_u16();
let raw = resp.into_body().read_to_string().ok();
let result = match classify(status) {
    None => raw.clone().ok_or_else(|| HttpError::Other("body read error".into())),
    Some(err) => Err(err),
};
```

`get_public` keeps its own 200-arm (it maps the read error to `Other`, `get` maps a missing
body to `Other("body read error")` — different, so the 200 arm stays inline). Only the error
arms route through `classify`:

```rust
let result = match classify(status) {
    None => resp.into_body().read_to_string().map_err(|e| {
        crate::diag!(crate::diag::Level::Err, "Reading body from {} failed: {}", url, e);
        HttpError::Other(e.to_string())
    }),
    Some(err) => Err(err),
};
```

The existing error-logging tails in both functions stay unchanged.

### 2. `open_url` (main.rs)

```rust
fn open_url(url: &str) {
    if let Err(e) = std::process::Command::new("open").arg(url).spawn() {
        crate::diag!(crate::diag::Level::Err, "Failed to open {}: {}", url, e);
    }
}
```

Each branch collapses to `open_url(CLAUDE_SETUP_URL);` etc. The releases branch uses the
inline literal it already has.

### 3. Single-pass `details_kinds` (refresh_all)

Build the set once from the providers (the authority on `raw_json`), then intersect with the
fetched refs:

```rust
let kinds_with_raw: Vec<ProviderKind> = self
    .providers
    .iter()
    .filter(|p| p.raw_json().is_some())
    .map(|p| p.kind())
    .collect();
let details_kinds: Vec<ProviderKind> = refs
    .iter()
    .map(|(k, _)| *k)
    .filter(|k| kinds_with_raw.contains(k))
    .collect();
```

Each provider's `raw_json` is locked exactly once. Output order matches `refs` as before.

## Error handling

None of the three changes introduces a new failure mode. `classify` is total over `u16`;
`open_url` preserves the existing log-and-continue behavior; the `details_kinds` rewrite is
a pure restructuring of a `Vec` build.

## Testing

- `classify`: unit tests for 200 → `None`, 401/429/503/418 → expected `Some(_)`.
- `get` / `get_public`: existing tests (`shared_agent_is_reused`, the structural ones) stay
  green; no network test added.
- `open_url`: not unit-tested — it spawns a process and only logs. Correctness is the
  trivial wiring; covered by manual smoke (clicking a setup link).
- `details_kinds`: no new test — it is internal to `refresh_all` (drives the tray). The
  observable output (which "Details" entries render) is unchanged; verified by the existing
  menu-build tests in `ui`.
- Gate: `cargo clippy -- -D warnings && cargo test`.
