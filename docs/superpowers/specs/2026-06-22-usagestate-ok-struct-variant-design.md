# Convert UsageState::Ok tuple variant to a struct variant

## Problem

`UsageState::Ok(Vec<LimitWindow>, Option<String>)` (`src/provider/mod.rs:18`) is a tuple
variant whose second field — the profile string, e.g. `"a@b.com (pro)"` — is unlabeled. At
every match site the reader must know positionally that the `Option<String>` means "profile".
Constructions like `UsageState::Ok(windows, profile_string)` and matches like
`UsageState::Ok(ref w, ref p)` carry no field names. There are 47 references across 7 files
(`icon.rs`, `ui/claude.rs`, `ui/copilot.rs`, `ui/mod.rs`, `provider/claude.rs`,
`provider/copilot.rs`, `provider/mod.rs`).

No bug — pure readability. Split out of card #54 because its blast radius (every `Ok`
construction and match) dwarfs the one-file edits in that card.

## Approach

Convert to a struct variant with named fields. Mechanically update every construction and
pattern. Equivalence-preserving: the compiled behavior is identical; only the syntax at each
site changes.

Rejected alternative: introduce a dedicated `struct OkPayload { windows, profile }` and keep
`Ok(OkPayload)`. Rejected — adds a named type for no gain over an inline struct variant, and
forces an extra construction step at every site.

## Scope

In scope:

- The variant definition in `provider/mod.rs`.
- All 47 construction/match sites across the 7 files above, including test modules.

Out of scope:

- The other six idiomatic cleanups (card #54).
- Any change to the meaning of `windows` or `profile`, or to when `Ok` is produced.

## Design

### Variant definition (`provider/mod.rs`)

```rust
pub(crate) enum UsageState {
    NotConfigured,
    Stale(String),
    Ok {
        windows: Vec<LimitWindow>,
        profile: Option<String>,
    },
    Error(String),
}
```

### Construction sites

Tuple form → struct form:

```rust
// before
UsageState::Ok(windows, profile_string)
// after
UsageState::Ok { windows, profile: profile_string }
```

Where the local is already named `profile`, field-init shorthand applies:
`UsageState::Ok { windows, profile }`.

### Match sites

```rust
// before
UsageState::Ok(w, p) => ...
UsageState::Ok(..)    => ...
// after
UsageState::Ok { windows, profile } => ...   // bind what is used
UsageState::Ok { .. }                => ...   // wildcard
```

Sites that only read `windows` and ignore the profile use
`UsageState::Ok { windows, .. }`; sites needing both bind both. The `state_diag_message`
wildcard arm (`provider/mod.rs:44`) becomes `UsageState::Ok { .. }`.

### Mechanical safety

The compiler enforces exhaustiveness and field naming — a missed site is a build error, not a
silent bug. Work file-by-file; `cargo check` after each catches stragglers.

## Error handling

No new failure modes. The conversion is syntactic; control flow and produced states are
unchanged.

## Testing

- Every existing test that constructs or matches `UsageState::Ok` (in `provider/mod.rs`,
  `provider/claude.rs`, `provider/copilot.rs`, and any `ui`/`icon` tests) is updated to the
  struct form. Their assertions are unchanged in meaning.
- No new behavioral test — equivalence-preserving.
- Gate: `cargo clippy -- -D warnings && cargo test`. Because the edit spans 7 files, run
  `cargo check` incrementally while converting.
