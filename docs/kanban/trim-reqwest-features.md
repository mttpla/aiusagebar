---
id: 18
status: backlog
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
