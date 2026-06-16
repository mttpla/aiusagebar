---
id: 15
status: done
priority: Normal
tags: [perf, http, pre-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Reuse single `reqwest::Client`

`src/http.rs` likely builds a fresh `Client` per request. Build once at startup, pass `&Client` to provider `fetch()`. Keepalive + connection pool → halves latency on subsequent calls + one TLS handshake instead of N.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Verify current implementation before estimating. Trivial change but compounds with multi-provider growth.
- 2026-06-17: Closed. Already implemented via ureq migration (#40/#41). `src/http.rs` uses `OnceLock<ureq::Agent>` — singleton, connection pool, single TLS handshake per host. Goal fully met.
