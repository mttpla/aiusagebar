---
id: 40
status: backlog
priority: Medium
tags: [performance, dependencies, memory]
spec: specs/2026-06-16-ureq-migration-design.md
plan: plans/2026-06-16-ureq-migration.md
created: 2026-06-16
updated: 2026-06-16
---
# Migrate HTTP client from reqwest to ureq 3 + native-tls

Replace `reqwest` (blocking) with `ureq 3` + `native-tls` to eliminate the tokio/hyper/rustls stack and reduce RSS by an estimated 8-15 MB.

## Narrative
- 2026-06-16: App measured at 21 MB RSS — high for a menu bar app doing 2 periodic HTTP calls. Root cause: reqwest blocking creates an internal tokio runtime (background threads, each 2 MB stack) plus embeds rustls/webpki-roots. ureq is sync-native with no async runtime. On macOS, native-tls delegates to SecureTransport (already loaded by the OS). Sole change surface: `src/http.rs` (45 lines) + `Cargo.toml`. Provider files untouched — they only call `crate::http::get()`. Rejected: reqwest + capped tokio threads (saves only 2-4 MB, tokio stays). Rejected: ureq + rustls (removes tokio but keeps embedded TLS stack).
