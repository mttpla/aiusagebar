---
id: 55
status: backlog
priority: Normal
tags: [refactor, idiomatic, types]
spec: docs/superpowers/specs/2026-06-22-usagestate-ok-struct-variant-design.md
created: 2026-06-22
updated: 2026-06-22
---
# Convert UsageState::Ok tuple variant to a struct variant

`UsageState::Ok(Vec<LimitWindow>, Option<String>)` carries an unlabeled `Option<String>` (the profile string) that is opaque at every match site. Convert to a named struct variant `Ok { windows, profile }`.

Split out of #54 (idiomatic Rust cleanups) because it has the widest blast radius of that set — it touches every `UsageState::Ok` construction and match across `claude.rs`, `copilot.rs`, `main.rs`, `ui/*`, and the test modules — whereas #54's remaining items are one-file local edits. Keeping it separate lets the mechanical edits ship without waiting on this wider sweep.

## Constraints

- Equivalence-preserving — no behavior change, all tests stay green (patterns updated mechanically to the named form).
- `cargo clippy -- -D warnings && cargo test` before commit.
- All code/comments English; `pub(crate)`/private only.

## Narrative
- 2026-06-22: Split out of #54 per spec-split check. Item 5 of the idiomatic-cleanups spec moved here verbatim; #54 spec updated to point at this card.
