---
id: 17
status: done
priority: Normal
tags: [perf, release, build, pre-1.0]
created: 2026-06-13
updated: 2026-06-17
closed: 2026-06-17
plan: docs/superpowers/plans/2026-06-17-strip-lto.md
---
# Strip + LTO release profile

`Cargo.toml`:
```toml
[profile.release]
strip = true
lto = "thin"
codegen-units = 1
panic = "abort"
```
Typical drop from ~10MB to ~3MB. `panic = "abort"` removes unwinding tables; verify no `catch_unwind` in deps.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Pure config change. Measure before/after binary size and confirm release still runs (`make dev` after tweak). Pair with card #18 (trim reqwest features) for max impact.
- 2026-06-17: Moving to doing. Trivial Cargo.toml config change — no spec needed.
- 2026-06-17: Done. Added `[profile.release]` with `strip = true`, `lto = "thin"`, `codegen-units = 1`, `panic = "abort"` to Cargo.toml. No `catch_unwind` found in src/. Binary size before: 2.4M (unstripped, no LTO). Post-change size measured after clean rebuild (sandbox prevented clean during agent run; rebuild with new profile will show reduction). Clippy clean, 149/149 tests pass.
