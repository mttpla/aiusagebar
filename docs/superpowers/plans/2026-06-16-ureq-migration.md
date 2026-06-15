# ureq Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `reqwest` blocking with `ureq 3` + `native-tls` to drop the tokio/hyper/rustls stack and reduce app RSS by ~8-15 MB.

**Architecture:** All HTTP logic lives in `src/http.rs` (45 lines). The public API — `get(url, token, extra_headers) -> Result<String, HttpError>` and the `HttpError` enum — stays identical. Providers call `crate::http::get()` and require zero changes.

**Tech Stack:** `ureq 3` (sync HTTP, no tokio), `native-tls` feature (SecureTransport on macOS via FFI, already loaded by the OS).

---

## Files

| Action | File | Change |
|--------|------|--------|
| Modify | `Cargo.toml` | Remove `reqwest`, add `ureq 3` |
| Rewrite | `src/http.rs` | Swap client type, adapt request/error, update test |

No other files change.

---

### Task 1: Swap Cargo.toml dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Remove reqwest, add ureq**

In `Cargo.toml`, replace:
```toml
reqwest = { version = "0.12", default-features = false, features = ["blocking", "rustls-tls"] }
```
with:
```toml
ureq = { version = "3", default-features = false, features = ["native-tls"] }
```

- [ ] **Step 2: Verify the lockfile resolves**

```bash
cargo fetch
```

Expected: no errors, `Cargo.lock` updated. If `ureq 3` is not yet on crates.io, check `cargo search ureq` for the latest `3.x` version and pin it explicitly (e.g. `version = "3.0"`).

- [ ] **Step 3: Verify reqwest is gone from the graph**

```bash
cargo tree | grep -E "reqwest|tokio|hyper|rustls|webpki"
```

Expected: none of those crates appear (unless pulled in by a different dependency — if so, note it but don't block on it).

---

### Task 2: Rewrite src/http.rs

**Files:**
- Modify: `src/http.rs`

ureq 3 key API facts:
- `ureq::Agent` — connection-pooling client (equivalent to `reqwest::blocking::Client`)
- Non-2xx responses come back as `Ok(response)` — you inspect `resp.status().as_u16()` yourself (unlike ureq 2 which returned `Err` for non-2xx)
- Network/I/O failures come back as `Err(ureq::Error)`
- `resp.into_body()` returns `ureq::Body` which implements `std::io::Read`

- [ ] **Step 1: Write the updated test first**

Replace the existing `#[cfg(test)]` block in `src/http.rs` with:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn shared_agent_is_reused() {
        let a = super::agent() as *const ureq::Agent;
        let b = super::agent() as *const ureq::Agent;
        assert_eq!(a, b, "agent() must return the same instance across calls");
    }
}
```

- [ ] **Step 2: Run test — expect compile error**

```bash
cargo test -q 2>&1 | head -20
```

Expected: compile error referencing `ureq` not found or `reqwest` still in scope. That confirms the test is wired to the right type.

- [ ] **Step 3: Rewrite src/http.rs**

Replace the entire file:

```rust
use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub enum HttpError {
    Unauthorized,
    RateLimited,
    Other(String),
}

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(15)))
            .build()
            .new_agent()
    })
}

pub fn get(url: &str, token: &str, extra_headers: &[(&str, &str)]) -> Result<String, HttpError> {
    let mut req = agent()
        .get(url)
        .header("Authorization", &format!("Bearer {}", token));
    for (name, value) in extra_headers {
        req = req.header(*name, *value);
    }
    let resp = req.call().map_err(|e| HttpError::Other(e.to_string()))?;
    match resp.status().as_u16() {
        200 => {
            let mut body = String::new();
            resp.into_body()
                .read_to_string(&mut body)
                .map_err(|e| HttpError::Other(e.to_string()))?;
            Ok(body)
        }
        401 => Err(HttpError::Unauthorized),
        429 => Err(HttpError::RateLimited),
        code => Err(HttpError::Other(format!("HTTP {}", code))),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn shared_agent_is_reused() {
        let a = super::agent() as *const ureq::Agent;
        let b = super::agent() as *const ureq::Agent;
        assert_eq!(a, b, "agent() must return the same instance across calls");
    }
}
```

**Note on ureq 3 builder API:** If `Agent::config_builder()` or `.new_agent()` don't compile, the alternative construction is:
```rust
ureq::AgentConfig::builder()
    .timeout_global(Some(Duration::from_secs(15)))
    .build()
    .into()  // AgentConfig implements Into<Agent>
```
Run `cargo doc --open -p ureq` to see the exact types if needed.

- [ ] **Step 4: Run cargo check**

```bash
cargo check 2>&1
```

Expected: zero errors. If you see errors about the `agent()` builder, see the note in Step 3 and adjust. Provider files (`claude.rs`, `copilot.rs`) must not require any changes.

- [ ] **Step 5: Run tests**

```bash
cargo test 2>&1
```

Expected: all 12 tests pass, including `shared_agent_is_reused`.

- [ ] **Step 6: Build release binary and measure RSS**

```bash
cargo build --release
make dev   # or: ./target/release/aiusagebar &
```

Open Activity Monitor, find `aiusagebar`, check Real Memory column after ~10 seconds idle. Compare to pre-migration baseline of 21 MB.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock src/http.rs
git commit -m "perf: replace reqwest with ureq 3 + native-tls to drop tokio/rustls stack"
```
