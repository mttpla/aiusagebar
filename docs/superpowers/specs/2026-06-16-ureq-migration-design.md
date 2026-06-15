# Spec: Migrate HTTP client from reqwest to ureq 3 + native-tls

**Date:** 2026-06-16  
**Goal:** Reduce RSS by ~8-15 MB by eliminating the tokio/hyper/rustls stack.

## Problem

`reqwest` blocking creates an internal `tokio::runtime::Runtime` (single-threaded executor + blocking thread pool). Combined with `hyper`, `rustls`, and `webpki-roots`, this pulls in ~5-10 MB of code and allocates background threads (each with a 2 MB default stack) that exist solely to serve 2 periodic HTTP calls.

## Approach

Replace `reqwest` with `ureq 3` configured with the `native-tls` feature. `ureq` is a synchronous HTTP client with no async runtime dependency. On macOS, `native-tls` uses SecureTransport via FFI — already resident in memory by the OS, so TLS costs nothing additional.

## Dependency changes

**Remove:**
```
reqwest = { version = "0.12", default-features = false, features = ["blocking", "rustls-tls"] }
```

**Add:**
```
ureq = { version = "3", default-features = false, features = ["native-tls"] }
```

Crates that exit the dependency graph: `tokio`, `hyper`, `hyper-util`, `hyper-rustls`, `rustls`, `rustls-pki-types`, `webpki-roots`, `tokio-rustls`, `h2` and their transitive deps.

## Code changes

### `src/http.rs` (only file that touches the HTTP client)

- Static singleton: `OnceLock<reqwest::blocking::Client>` → `OnceLock<ureq::Agent>`
- Agent construction: `ureq::AgentConfig::builder().timeout_global(Some(Duration::from_secs(15))).build().into()`
- Request: `agent.get(url).header("Authorization", format!("Bearer {}", token)).header(name, value).call()`
- Response body: `response.into_body().read_to_string(&mut String::new())` (or equivalent ureq 3 idiom)
- Error mapping:
  - `ureq::Error::Status(401, _)` → `HttpError::Unauthorized`
  - `ureq::Error::Status(429, _)` → `HttpError::RateLimited`
  - `ureq::Error::Status(code, _)` → `HttpError::Other(format!("HTTP {}", code))`
  - transport/io errors → `HttpError::Other(e.to_string())`
- Public API (`get()` signature, `HttpError` enum) unchanged — providers require zero modifications.

### `src/provider/claude.rs`, `src/provider/copilot.rs`

No changes. Both only call `crate::http::get()`.

### Tests

`shared_client_is_reused` test: update pointer comparison to use `ureq::Agent` type. Functional behaviour identical.

## Out of scope

- Measurement: verify RSS reduction manually in Activity Monitor post-build.
- No changes to error types, polling logic, or provider behaviour.
- HTTP/2 not needed; ureq 3 is HTTP/1.1 only — acceptable since both API endpoints are plain REST.

## Risks

- ureq 3 API is a major breaking change from ureq 2; verify exact builder and error-matching syntax against crate docs during implementation.
- `native-tls` on macOS requires linking against `Security.framework` — already linked by `security-framework` crate, no new linker flags needed.
