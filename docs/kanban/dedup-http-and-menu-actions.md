---
id: 53
status: backlog
priority: Normal
tags: [refactor, dry, cleanup]
spec: docs/superpowers/specs/2026-06-22-dedup-http-and-menu-actions-design.md
created: 2026-06-22
updated: 2026-06-22
---
# Remove duplication across http, menu actions, and details lookup

Three spots repeat logic that should live in one place. Pure cleanup — no behavior change. Each item is independently shippable.

## Scope

1. **HTTP status → `HttpError` mapping** — `get` (`src/http.rs:45`) and `get_public` (`src/http.rs:74`) both inline the same `match status { 401 / 429 / 500..=599 / other }` arms. Extract a single helper, e.g. `fn classify(status: u16) -> Option<HttpError>` (`None` = 200/OK), and call it from both. Keeps the two response-body code paths but removes the duplicated status table.

2. **"open URL" menu actions** — `main.rs` (lines ~176–189) repeats `std::process::Command::new("open").arg(url).spawn()` with diag-on-error four times (update, setup_claude, setup_copilot, releases). Factor `fn open_url(url: &str)` that spawns and logs failure via `crate::diag!`, then call it from each branch.

3. **`details_kinds` build** (`main.rs:111`) — currently O(n²): for each ref it re-scans `self.providers` with `.any()`, locking `raw_json()` per inner hit. Build the list in a single pass over providers (collect `(kind, has_raw)` once) instead of the nested scan.

## Constraints

- No behavior change — outputs identical, tests stay green.
- `cargo clippy -- -D warnings && cargo test` before commit.
- All code/comments in English; `pub(crate)`/private only, no bare `pub`.

## Narrative
- 2026-06-22: Captured from full-codebase review. Three independent DRY findings grouped into one card because each is a small, self-contained edit. Item 1 (http classify) is the highest-value — two near-identical status tables drift apart over time. Items 2 and 3 are ergonomics. Deliberately kept separate from the non-idiomatic-Rust card (#54) so each can ship without touching the other's files.
- 2026-06-22: Spec written and linked. Note cross-dependency with #54 item 3: the `http.rs:46` body-clone readability fix lives in #54 but overlaps the 200-arm that `classify` rewrites here — whichever lands first absorbs the other's change to that arm.
