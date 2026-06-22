# Idiomatic Rust cleanups in refresh loop, providers, and http

## Problem

Review of the codebase found seven non-idiomatic patterns. None is a bug; all are
readability / allocation / idiom issues. No observable behavior change in any item. Items
are independent and ordered by blast radius (smallest first).

Item 5 (the `UsageState::Ok` tuple → struct variant) was split out to card #55 during the
spec-split check — its 47-site blast radius dwarfs the rest. It is left as a stub pointer
below so item numbering matches the card narrative; this spec now covers the six one-file
local edits.

## Approach

Apply each fix in isolation, keeping every test green. Items 1–4 and 6 are mechanical and
local; item 7 is a judgment call resolved at implementation time.

## Scope

In scope: the seven items below.

Out of scope:

- Moving fetches off the event loop (🔴, separate concern).
- The Claude UA fallback string `"claude-code/2.1.153"` (`claude.rs:105`) — its rot risk is
  tied to constraint #3 (wrong UA = instant 429) and there is no clean live source; left
  alone deliberately.
- The HTTP status-table duplication and `open_url` / `details_kinds` cleanups — those are
  DRY, tracked in card #53.

## Design

### 1. Index loop → iterator (`main.rs:78`)

`fetch_with_http_error(&self)` and `kind(&self)` both take a shared ref, so the manual index
loop is unnecessary:

```rust
for p in &self.providers {
    let kind = p.kind();
    let (state, http_err) = p.fetch_with_http_error();
    if let Some(msg) = crate::provider::state_diag_message(kind.display_name(), &state) {
        crate::diag!(crate::diag::Level::Err, "{}", msg);
    }
    states.push((kind, state));
    http_errs.push(http_err);
}
```

### 2. `format!` into `push_str` (`copilot.rs:70`)

Allocating a throwaway `String` to immediately copy it into `raw_buf` is wasteful. Write
directly:

```rust
use std::fmt::Write;
// ...
let _ = write!(raw_buf, "--- @{account} ---\n{body}");
```

(`write!` into a `String` is infallible; `let _ =` discards the `Result` without `unwrap`.)

### 3. Redundant clone + clumsy chain (`http.rs:46`)

Current:

```rust
200 => raw.clone().map(Ok).unwrap_or_else(|| Err(HttpError::Other("body read error".into()))),
```

Replace with an explicit match that reads plainly:

```rust
200 => match &raw {
    Some(b) => Ok(b.clone()),
    None => Err(HttpError::Other("body read error".into())),
},
```

The clone stays (both `result` and the returned `raw` need an owned copy — `result` feeds the
parser, `raw` is cached as `last_raw_json`), but the intent is now explicit. Note: if card #53
lands first and rewrites this arm via `classify`, fold this readability fix into that change
rather than doing it twice.

### 4. Two-scope mutex dance (`claude.rs:289-303`)

Currently the `profile` mutex is locked to populate, dropped, then re-locked to read and
format. Collapse to one scope:

```rust
let profile_string = {
    let mut profile = self.profile.lock().unwrap();
    if profile.is_none() {
        if let CredLoad::Ok(ref c) = creds {
            *profile = fetch_profile(&c.access_token, ua);
        }
    }
    profile.as_ref().map(|p| format!("{} ({})", p.email, p.plan))
};
```

One lock acquisition; the format happens before the guard drops.

### 5. Opaque tuple variant — split out

Moved to card #55 (`docs/superpowers/specs/2026-06-22-usagestate-ok-struct-variant-design.md`).
Not implemented under this card. Left here as a numbering anchor only.

### 6. Hardcoded version string (`copilot.rs:179`)

Copilot's request User-Agent is the literal `"aiusagebar/0.1"`. Match what `get_public`
already does:

```rust
&[("User-Agent", concat!("aiusagebar/", env!("CARGO_PKG_VERSION")))],
```

`concat!`/`env!` are const, so this stays a `&'static str` — no allocation, and the version
tracks `Cargo.toml` automatically.

### 7. Low-value signature tests (`http.rs:99-108`)

`get_public_function_exists_and_compiles` and `get_returns_tuple` assert function signatures
the compiler already enforces — they cannot fail independently of compilation. Decision at
implementation time: delete as noise, or keep with a one-line comment declaring them
intentional API-shape guards. Default: delete.

## Error handling

No item introduces a new failure mode. All are refactors of existing control flow:
iteration shape (1), allocation (2, 6), expression form (3, 4), type shape (5), test
deletion (7).

## Testing

- Existing unit suites for `http`, `claude`, `copilot`, `provider` must stay green
  unchanged in meaning; these are equivalence-preserving edits.
- No new behavioral tests needed.
- Item 7 removes two tests by design.
- Gate: `cargo clippy -- -D warnings && cargo test` after each item.
