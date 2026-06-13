---
id: 17
status: backlog
priority: Normal
tags: [perf, release, build, pre-1.0]
created: 2026-06-13
updated: 2026-06-13
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
