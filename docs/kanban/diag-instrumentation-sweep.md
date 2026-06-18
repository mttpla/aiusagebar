---
id: 46
status: backlog
priority: Normal
blocked_by: [44]
tags: [robustness, logging, post-1.0]
created: 2026-06-18
updated: 2026-06-18
---
# Instrumentation sweep: add diag! call sites across all error paths

After card #44 ships the `diag!` infrastructure, sweep the codebase and add
`inspect_err(|e| diag!(...))` at every error site where diagnostic context is
useful. The macro auto-injects `file!():line!()` — callers only write the message.

## Scope

- `src/http.rs` — all `HttpError` variants (Status, RateLimited, Network, etc.)
- `src/keychain.rs` — token read failures, malformed JSON
- `src/provider/claude.rs` — any error paths not already covered by #44 hook points
- Future providers (Copilot, Codex) — instrument at the time they are written

## Out of scope

- Logging success paths (Info-level last-ok snapshots already handled in #44)
- Adding new error variants — this is logging only, no error type changes

## Rules

- Messages must be specific: include operation, URL/service, and error value.
  Bad: `"fetch failed"`. Good: `"Copilot usage fetch failed ({}): {}", url, e`.
- Use `inspect_err`, not methods on error types — error types lack call-site context.
- No new dependencies.

## Narrative

- 2026-06-18: Split from #44 at design review. #44 delivers infrastructure + 3 Claude
  hook points sufficient for pre-1.0 error reporting. Full sweep across all providers
  and modules is post-1.0 — scope grows as more providers are added. Blocked by #44.
  Architecture decision: `inspect_err` at call sites preferred over logging methods on
  error types (error types lack URL/provider context). `diag!` macro handles file+line
  automatically.
