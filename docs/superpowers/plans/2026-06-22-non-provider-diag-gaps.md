# Non-Provider Diagnostics Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `diag!(Err, …)` traces to the non-provider silent failure paths found in the 2026-06-22 audit, so no malfunction outside the provider fetch goes untraced.

**Architecture:** One pure predicate (`io_error_is_loggable`) gates the per-poll credential-file read; every other site is a mechanical `if let Err(e) { diag!(…) }` / match-and-log around an OS call. All genuine failures log at `Level::Err`; benign and already-traced paths are left alone.

**Tech Stack:** Rust, existing `crate::diag!` macro and `crate::diag::Level`, `cargo test` / `cargo clippy`.

## Global Constraints

- All `.rs` string literals must be English (runtime i18n is separate).
- No bare `pub` in this binary crate — default `pub(crate)` / private.
- Never add a `Co-Authored-By` trailer to commit messages.
- Run `cargo clippy -- -D warnings && cargo test` before every commit.
- No `#[allow(dead_code)]` — delete unused symbols instead.
- Tokens read-only; this feature adds logging only, touches no auth/network write path.
- All new failure logs use `crate::diag::Level::Err`.
- Skip `std::io::ErrorKind::NotFound` on the credential-file read (file absent = token in Keychain — expected, not a failure).

---

### Task 1: Credential-source read logging

**Files:**
- Modify: `src/provider/claude.rs` (add `io_error_is_loggable` before `load_credentials_json` ~line 46; rewrite the file fallback at line 52; add 2 tests to the `#[cfg(test)] mod tests`)
- Modify: `src/keychain.rs` (rewrite the per-item read in the `enumerate_generic_passwords` closure, lines 93-94)

**Interfaces:**
- Consumes: existing `crate::diag!` macro, `crate::diag::Level::Err`, `keychain_error_is_loggable` (`src/keychain.rs:8`).
- Produces: `fn io_error_is_loggable(e: &std::io::Error) -> bool` (private to `claude.rs`).

- [ ] **Step 1: Write the failing tests**

Add to the `mod tests` block in `src/provider/claude.rs`:

```rust
    #[test]
    fn io_error_not_found_is_not_loggable() {
        let e = std::io::Error::from(std::io::ErrorKind::NotFound);
        assert!(!super::io_error_is_loggable(&e));
    }

    #[test]
    fn io_error_permission_denied_is_loggable() {
        let e = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        assert!(super::io_error_is_loggable(&e));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib provider::claude::tests::io_error`
Expected: FAIL — `cannot find function io_error_is_loggable`.

- [ ] **Step 3: Add the helper and wire the credential-file site**

In `src/provider/claude.rs`, insert immediately before `fn load_credentials_json()` (before line 46):

```rust
/// True for credential-file IO errors worth logging — genuine read failures, not the
/// expected `NotFound` that simply means the file is absent (token lives in the Keychain).
fn io_error_is_loggable(e: &std::io::Error) -> bool {
    e.kind() != std::io::ErrorKind::NotFound
}
```

Then replace the final two lines of `load_credentials_json` (currently):

```rust
    let path = dirs::home_dir()?.join(".claude").join(".credentials.json");
    std::fs::read_to_string(path).ok()
```

with:

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

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib provider::claude::tests::io_error`
Expected: PASS (2 tests).

- [ ] **Step 5: Wire the Keychain enumerate site**

In `src/keychain.rs`, in the `enumerate_generic_passwords` closure, replace these two lines (currently 93-94):

```rust
            let password = get_generic_password(service, &account).ok()?;
            String::from_utf8(password).ok().map(|p| (account, p))
```

with:

```rust
            let password = match get_generic_password(service, &account) {
                Ok(p) => p,
                Err(e) => {
                    if keychain_error_is_loggable(e.code()) {
                        crate::diag!(
                            crate::diag::Level::Err,
                            "Keychain read failed for service {} account {} (status {}): {}",
                            service, account, e.code(), e
                        );
                    }
                    return None;
                }
            };
            match String::from_utf8(password) {
                Ok(p) => Some((account, p)),
                Err(e) => {
                    crate::diag!(
                        crate::diag::Level::Err,
                        "Keychain item for service {} account {} is not valid UTF-8: {}",
                        service, account, e
                    );
                    None
                }
            }
```

- [ ] **Step 6: Quality gate + commit**

```bash
cargo clippy -- -D warnings && cargo test
git add src/provider/claude.rs src/keychain.rs
git commit -m "feat(diag): log credential-file and Keychain enumerate read failures"
```

Expected: clippy clean, all tests pass (191 — 189 prior + 2 new).

---

### Task 2: Update-check, tray, and launch-command logging

**Files:**
- Modify: `src/update_check.rs` (rewrite the parse in `parse_release`, line 10)
- Modify: `src/main.rs` (set_icon line 121; three `open` spawns at lines 158, 162, 164; `enable()` error at line 196)
- Modify: `src/about.rs` (the `open` spawn at lines 74-76)

**Interfaces:**
- Consumes: existing `crate::diag!` macro, `crate::diag::Level::Err`, the existing `CLAUDE_SETUP_URL` / `COPILOT_SETUP_URL` consts (`src/main.rs:37-40`).
- Produces: no new public symbol — behavioral wiring only.

- [ ] **Step 1: Log malformed releases JSON in `parse_release`**

In `src/update_check.rs`, replace line 10:

```rust
    let release: GithubRelease = serde_json::from_str(json).ok()?;
```

with:

```rust
    let release: GithubRelease = match serde_json::from_str(json) {
        Ok(r) => r,
        Err(e) => {
            crate::diag!(crate::diag::Level::Err, "Update check: malformed releases JSON: {}", e);
            return None;
        }
    };
```

- [ ] **Step 2: Log tray `set_icon` failure**

In `src/main.rs`, replace line 121:

```rust
        self.tray.set_icon(Some(self.icons.get(icon_kind))).ok();
```

with:

```rust
        if let Err(e) = self.tray.set_icon(Some(self.icons.get(icon_kind))) {
            crate::diag!(crate::diag::Level::Err, "Tray set_icon failed: {}", e);
        }
```

- [ ] **Step 3: Log the three `open` spawn failures in the menu handler**

In `src/main.rs`, replace the releases-page spawn (currently lines 155-157):

```rust
                let _ = std::process::Command::new("open")
                    .arg("https://github.com/mttpla/aiusagebar/releases/latest")
                    .spawn();
```

with:

```rust
                if let Err(e) = std::process::Command::new("open")
                    .arg("https://github.com/mttpla/aiusagebar/releases/latest")
                    .spawn()
                {
                    crate::diag!(crate::diag::Level::Err, "Failed to open releases page: {}", e);
                }
```

Replace the Claude setup spawn (currently line 162):

```rust
                let _ = std::process::Command::new("open").arg(CLAUDE_SETUP_URL).spawn();
```

with:

```rust
                if let Err(e) = std::process::Command::new("open").arg(CLAUDE_SETUP_URL).spawn() {
                    crate::diag!(crate::diag::Level::Err, "Failed to open {}: {}", CLAUDE_SETUP_URL, e);
                }
```

Replace the Copilot setup spawn (currently line 164):

```rust
                let _ = std::process::Command::new("open").arg(COPILOT_SETUP_URL).spawn();
```

with:

```rust
                if let Err(e) = std::process::Command::new("open").arg(COPILOT_SETUP_URL).spawn() {
                    crate::diag!(crate::diag::Level::Err, "Failed to open {}: {}", COPILOT_SETUP_URL, e);
                }
```

- [ ] **Step 4: Log the `enable()` failure (replace stderr)**

In `src/main.rs`, replace line 196:

```rust
        eprintln!("[launch_at_login] {e}");
```

with:

```rust
        crate::diag!(crate::diag::Level::Err, "launch_at_login enable failed: {}", e);
```

- [ ] **Step 5: Log the About `open` spawn failure**

In `src/about.rs`, replace lines 74-76:

```rust
        let _ = std::process::Command::new("open")
            .arg("https://www.matteopaoli.it")
            .spawn();
```

with:

```rust
        if let Err(e) = std::process::Command::new("open")
            .arg("https://www.matteopaoli.it")
            .spawn()
        {
            crate::diag!(crate::diag::Level::Err, "Failed to open matteopaoli.it: {}", e);
        }
```

- [ ] **Step 6: Quality gate + commit**

```bash
cargo clippy -- -D warnings && cargo test
git add src/update_check.rs src/main.rs src/about.rs
git commit -m "feat(diag): log update-check parse, tray icon, and open-command failures"
```

Expected: clippy clean (no unused-`Result`/`#[must_use]` warnings remain), all tests pass.

---

## Self-Review

**Spec coverage:**
- Site 1 cred-file IO error + `io_error_is_loggable` + NotFound skip → Task 1 Steps 1-4. ✓
- Site 2 Keychain enumerate item drop (read + UTF-8) → Task 1 Step 5. ✓
- Site 3 update_check malformed JSON → Task 2 Step 1. ✓
- Site 4 set_icon → Task 2 Step 2. ✓
- Site 5 three `open` spawns in main + about → Task 2 Steps 3, 5. ✓
- Site 6 enable() eprintln → diag → Task 2 Step 4. ✓
- Decision "all Err" → every log uses `Level::Err`. ✓
- Dropped benign (bootstrap-warning, debug-skip) and already-traced (#3 profile) → no task touches them. ✓
- No dedup → no caching logic added. ✓

**Placeholder scan:** No TBD/TODO; every code step shows complete before/after code. ✓

**Type consistency:** `io_error_is_loggable(&std::io::Error) -> bool` defined and used in Task 1; `keychain_error_is_loggable(i32) -> bool` is the existing signature (`src/keychain.rs:8`), `e.code()` returns `i32`. `crate::diag!(Level, fmt, args…)` matches the existing macro. `CLAUDE_SETUP_URL` / `COPILOT_SETUP_URL` are existing `&str` consts. ✓
