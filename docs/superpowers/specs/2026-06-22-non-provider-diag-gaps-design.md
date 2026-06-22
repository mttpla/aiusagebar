# Diagnostics for non-provider silent paths

## Problem

Card #50 guaranteed that every provider fetch ending in a non-happy state leaves a
diagnostic trace. But the 2026-06-22 audit found non-happy paths **outside** the provider
fetch that still fail silently — no entry in the in-memory diagnostic log, and in some
cases output sent to `eprintln!` (stderr), which is invisible in a GUI menu-bar app. The
invariant we want: a malfunction anywhere in the app leaves a trace in the diagnostic log.

This card instruments those non-provider sites. It is the deferred remainder of the diag
sweep; card #50 covered the provider boundary, this covers everything else worth tracing.

## Decisions

- **Level:** all genuine failures log at `Level::Err`. Benign/expected conditions are not
  logged at all.
- **Dropped as benign** (stay on `eprintln!`, no diag): launch-at-login bootstrap warning
  (`launch_at_login.rs:88` — the plist is already registered, so the app starts at next
  login regardless) and the debug-build skip notice (`launch_at_login.rs:56`).
- **Dropped as already-traced:** the Claude profile-unavailable path
  (`fetch_profile`, `claude.rs:192-198`). Its only two failure causes are an HTTP error
  (already logged by `http.rs`) and a profile parse error (already logged at
  `claude.rs:196`). Re-logging would duplicate.
- **No deduplication** (consistent with #50). Almost every site here is one-shot — startup,
  a user menu click, or the 24-hour update check — so flooding is not a concern. The one
  per-poll site is the credential-file read; a genuine, persistent IO error there is rare
  and the resulting flood is accepted, exactly as in #50.
- **Skip expected "not found":** the credential-file read returns `None` legitimately when
  the file is absent (the token lives in the Keychain). Only genuine IO errors are logged,
  mirroring the existing `keychain_error_is_loggable` `errSecItemNotFound` skip.

## Sites instrumented

All line numbers are the post-#50-merge positions.

1. `src/provider/claude.rs:52` — `load_credentials_json` falls back to
   `std::fs::read_to_string(path).ok()`, swallowing IO errors. Log genuine errors; skip
   `ErrorKind::NotFound`.
2. `src/keychain.rs:93-94` — inside `enumerate_generic_passwords`, the per-item
   `get_generic_password(..).ok()?` and `String::from_utf8(..).ok()?` silently drop an
   account whose item is unreadable or not UTF-8. Log both, reusing
   `keychain_error_is_loggable` for the read error.
3. `src/update_check.rs:10` — `parse_release` swallows malformed releases JSON via
   `serde_json::from_str(json).ok()?`. Log the parse failure (only that branch — the
   no-assets and empty-tag `None` paths are legitimate, not failures).
4. `src/main.rs:121` — `self.tray.set_icon(..).ok()` swallows a tray-icon update failure.
   Log on `Err`.
5. `src/main.rs:158` (open releases), `:162` (open Claude setup), `:164` (open Copilot
   setup), and `src/about.rs:74` (open matteopaoli.it) — `let _ = Command::new("open")…
   .spawn()` swallows launch failures. Log each on `Err`.
6. `src/main.rs:196` — `enable()` failure is sent to `eprintln!` (stderr). Replace with a
   `diag!(Err, …)`. The benign bootstrap-warning and debug-skip `eprintln!`s inside
   `launch_at_login.rs` are left untouched (see Dropped as benign).

## Design

### Testable unit

In `src/provider/claude.rs`, a pure predicate mirroring `keychain_error_is_loggable`:

```rust
/// True for credential-file IO errors worth logging — genuine read failures, not the
/// expected `NotFound` that simply means the file is absent (token lives in the Keychain).
fn io_error_is_loggable(e: &std::io::Error) -> bool {
    e.kind() != std::io::ErrorKind::NotFound
}
```

Used by site 1:

```rust
let path = dirs::home_dir()?.join(".claude").join(".credentials.json");
match std::fs::read_to_string(&path) {
    Ok(json) => Some(json),
    Err(e) => {
        if io_error_is_loggable(&e) {
            crate::diag!(crate::diag::Level::Err, "Reading {} failed: {}", path.display(), e);
        }
        None
    }
}
```

### Wiring (no new logic)

The remaining sites are mechanical `if let Err(e) = … { diag!(Err, …) }` or
match-and-log around OS calls, each carrying the operation and the error value in the
message. Representative shapes:

- Open command (sites in main.rs / about.rs):
  ```rust
  if let Err(e) = std::process::Command::new("open").arg(URL).spawn() {
      crate::diag!(crate::diag::Level::Err, "Failed to open {}: {}", URL, e);
  }
  ```
- `set_icon`:
  ```rust
  if let Err(e) = self.tray.set_icon(Some(self.icons.get(icon_kind))) {
      crate::diag!(crate::diag::Level::Err, "Tray set_icon failed: {}", e);
  }
  ```
- `enable()` (main.rs:196): `eprintln!("[launch_at_login] {e}")` →
  `crate::diag!(crate::diag::Level::Err, "launch_at_login enable failed: {}", e)`.
- Keychain enumerate (keychain.rs): match the read; on `Err(e)` log via
  `keychain_error_is_loggable(e.code())` then `return None`; on UTF-8 `Err(e)` log then
  `None`. Messages include `service` and `account`.
- update_check parse: replace the `.ok()?` with a `match`; on `Err(e)` log
  `"Update check: malformed releases JSON: {}"` then `return None`.

## Error handling

Logging only. No new failure modes, no error-type changes, no new dependencies. `String`
allocation happens only on the logged (failure) branches.

## Testing

Unit tests on `io_error_is_loggable` (pure):

- `ErrorKind::NotFound` → `false`
- `ErrorKind::PermissionDenied` → `true`

The wiring sites drive OS calls / the tray and are not unit-tested; correctness there is
the trivial `if let Err` wiring around already-tested predicates, validated by the
`cargo clippy -- -D warnings && cargo test` gate staying green. `parse_release`'s existing
tests still assert `None` for malformed JSON (they now also emit a harmless diag entry).

## Out of scope

Provider boundary logging (#50, done). The menu-disappearance bug
(`refresh_all` partial rebuild) and diagnostic-log persistence across restarts — both
noted in #50's spec — remain separate.
