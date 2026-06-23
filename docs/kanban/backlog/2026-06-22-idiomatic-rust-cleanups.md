---
id: 54
status: backlog
priority: Normal
tags: [refactor, idiomatic, cleanup]
spec: docs/superpowers/specs/2026-06-22-idiomatic-rust-cleanups-design.md
created: 2026-06-22
updated: 2026-06-22
---
# Idiomatic Rust cleanups in refresh loop, providers, and http

Non-idiomatic patterns found in review. Cosmetic/readability — no behavior change. Each item stands alone.

## Scope

1. **Index loop → iterator** (`main.rs:78`) — `for i in 0..count { let kind = self.providers[i].kind(); let (state, err) = self.providers[i].fetch_with_http_error(); }` double-indexes. `fetch_with_http_error(&self)` takes a shared ref, so `for p in &self.providers { let kind = p.kind(); ... }` compiles and reads as Rust, not C.

2. **`format!` into `push_str`** (`copilot.rs:70`) — `raw_buf.push_str(&format!("--- @{} ---\n{}", account, body))` allocates a throwaway `String`. Replace with `use std::fmt::Write; let _ = write!(raw_buf, "--- @{account} ---\n{body}");`.

3. **Redundant clone + clumsy chain** (`http.rs:46`) — `raw.clone().map(Ok).unwrap_or_else(|| Err(HttpError::Other("body read error".into())))`. Rewrite as an explicit `match raw { Some(b) => Ok(b), None => Err(...) }`; review whether both `result` and the returned `raw` truly need an independent owned copy.

4. **Two-scope mutex dance** (`claude.rs:289-303`) — profile mutex is locked, dropped, then re-locked to read the formatted string. Collapse to a single lock scope (lock once, populate if empty, format the string before releasing).

5. **Opaque tuple variant** — moved to card #55 (split out: widest blast radius, 47 sites across 7 files).

6. **Hardcoded version strings** — Copilot UA `"aiusagebar/0.1"` (`copilot.rs:179`) should use `concat!("aiusagebar/", env!("CARGO_PKG_VERSION"))` like `get_public` already does. (Claude UA fallback `"claude-code/2.1.153"` is a separate rot/risk concern tied to constraint #3 — leave it out of this card.)

7. **Low-value signature tests** (`http.rs:99-108`) — `get_public_function_exists_and_compiles` and `get_returns_tuple` assert what the compiler already enforces. Delete as noise, or keep if used as intentional API-shape guards (decide during implementation).

## Constraints

- No behavior change — tests stay green.
- `cargo clippy -- -D warnings && cargo test` before commit.
- All string literals English; `pub(crate)`/private only.

## Narrative
- 2026-06-22: Captured from full-codebase review. Items 1–4 + 6 are mechanical and low-risk. Item 5 (tuple → struct variant) has the widest blast radius (every `UsageState::Ok` match site) — flagged to do last or split if it grows. Item 7 is a judgment call left to implementation time. Kept separate from the DRY card (#53) since these touch readability/idiom rather than duplication, though both can be tackled in one sweep if desired.
- 2026-06-22: Spec written and linked. Item ordering set by blast radius (1–4,6 local; 5 last). Item 3 overlaps #53's `classify` rewrite of the `http.rs:46` 200-arm — noted in both specs.
- 2026-06-22: Spec-split check → item 5 (UsageState::Ok struct variant, 47 sites) split out to card #55. This card now holds only the six one-file local edits. Item numbering kept (5 is a stub pointer) to preserve spec/narrative alignment.
