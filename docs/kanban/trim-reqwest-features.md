---
id: 18
status: done
priority: Normal
tags: [perf, deps, pre-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Trim reqwest default features

```toml
reqwest = { version = "x", default-features = false, features = ["rustls-tls", "blocking", "json"] }
```
Drops native-tls, cookies, gzip(?), brotli, charset detection — none used. Pairs with card #17.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Verify which features are actually used (`cargo tree -e features`). Expected -2-3MB binary, faster cold compile.
- 2026-06-17: Closed. reqwest removed entirely during ureq migration (#40/#41). ureq in Cargo.toml uses `default-features = false, features = ["native-tls"]` — minimal footprint already. Goal superseded.
