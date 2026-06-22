---
id: 51
status: backlog
priority: Normal
tags: [robustness, logging, diag, pre-1.0]
created: 2026-06-22
updated: 2026-06-22
---
# Diag coverage for non-provider silent paths

Add `diag!` traces to the non-provider error paths that currently fail silently, so no
malfunction outside the provider fetch goes untraced. Found during the 2026-06-22 audit;
carved out of card #50, which covers only the provider boundary.

Silent paths to instrument:

- `launch_at_login.rs` + `main.rs:193` — all failures go to `eprintln!` (stderr,
  invisible in a GUI app): `enable()` error, bootstrap warning, debug-skip notice.
- `main.rs:155/159/161` + `about.rs:74` — `let _ = Command::new("open")...spawn()`:
  Setup / About / Release URL launches swallow spawn failures.
- `claude.rs:52` — `read_to_string(path).ok()`: credential-file IO error swallowed.
- `claude.rs:194` — `fetch_profile` `result.ok()?`: profile-unavailable leaves no
  provider-level trace (the HTTP error itself is logged by the http layer).
- `keychain.rs:93-94` — per-item `get_generic_password(..).ok()?` / `from_utf8().ok()?`
  in `enumerate_generic_passwords`: an unreadable / non-UTF8 Keychain item is dropped
  silently (a Copilot account vanishes from the list).
- `update_check.rs:10` — `parse_release` `.ok()?`: malformed releases JSON → silent
  None (the HTTP error is logged, the parse failure is not).
- `main.rs:118` — `set_icon(..).ok()`: tray icon update failure swallowed.

## Narrative
- 2026-06-22: Captured from the diag audit during card #50 brainstorming. User wants
  every non-happy path traced, but chose to keep card #50 minimal (provider boundary
  only) and defer these non-provider paths here. Per-leaf instrumentation is appropriate
  here since there is no single boundary that covers them. Lightweight card — promote to
  spec/plan only when picked up.
