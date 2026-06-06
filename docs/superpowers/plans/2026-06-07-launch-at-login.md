# Launch at Login Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** AiUsageBar auto-starts on macOS login by self-registering a LaunchAgent plist at every startup.

**Architecture:** New `src/launch_at_login.rs` module writes `~/Library/LaunchAgents/com.mttpla.aiusagebar.plist` using `std::env::current_exe()` for the binary path, then calls `launchctl bootstrap`. The public API (`enable` / `disable` / `is_enabled`) is sized for the future settings toggle. `main()` calls `enable()` before the event loop. Dev builds skip registration via `#[cfg(debug_assertions)]`.

**Tech Stack:** Rust std only — `std::fs`, `std::process::Command`, `dirs` (already in Cargo.toml).

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/launch_at_login.rs` | All plist generation + launchctl integration |
| Modify | `src/main.rs` | Declare module, call `enable()` before event loop |

---

### Task 1: Stub module with failing test

**Files:**
- Create: `src/launch_at_login.rs`

- [ ] **Step 1: Create `src/launch_at_login.rs` with stub and test**

```rust
const LABEL: &str = "com.mttpla.aiusagebar";

fn plist_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| {
        h.join("Library/LaunchAgents")
            .join(format!("{LABEL}.plist"))
    })
}

fn plist_content(_binary_path: &str) -> String {
    todo!("implement plist_content")
}

#[cfg(debug_assertions)]
pub fn enable() -> Result<(), String> {
    eprintln!("[launch_at_login] skipped in debug build");
    Ok(())
}

#[cfg(not(debug_assertions))]
pub fn enable() -> Result<(), String> {
    todo!("implement enable")
}

pub fn disable() -> Result<(), String> {
    todo!("implement disable")
}

pub fn is_enabled() -> bool {
    plist_path().map(|p| p.exists()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_content_contains_label_and_binary() {
        let xml = plist_content("/opt/homebrew/bin/aiusagebar");
        assert!(xml.contains("<string>com.mttpla.aiusagebar</string>"));
        assert!(xml.contains("<string>/opt/homebrew/bin/aiusagebar</string>"));
        assert!(xml.contains("<true/>"));
        let keep_alive_pos = xml.find("KeepAlive").unwrap();
        assert!(xml[keep_alive_pos..].contains("<false/>"));
    }
}
```

- [ ] **Step 2: Run test — verify it fails**

```
cargo test launch_at_login
```

Expected: FAIL — `panicked at 'implement plist_content'`

---

### Task 2: Implement plist_content — test passes

**Files:**
- Modify: `src/launch_at_login.rs`

- [ ] **Step 1: Replace the `plist_content` stub with the real implementation**

Replace `fn plist_content(_binary_path: &str) -> String { todo!("implement plist_content") }` with:

```rust
fn plist_content(binary_path: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{binary_path}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#
    )
}
```

- [ ] **Step 2: Run test — verify it passes**

```
cargo test launch_at_login
```

Expected: `test launch_at_login::tests::plist_content_contains_label_and_binary ... ok`

- [ ] **Step 3: Commit**

```
git add src/launch_at_login.rs
git commit -m "feat: add launch_at_login module with plist_content"
```

---

### Task 3: Implement enable / disable / uid

**Files:**
- Modify: `src/launch_at_login.rs`

- [ ] **Step 1: Add `uid()` helper above the `enable` functions**

Insert after `fn plist_content`:

```rust
fn uid() -> Result<u32, String> {
    let out = std::process::Command::new("id")
        .arg("-u")
        .output()
        .map_err(|e| e.to_string())?;
    String::from_utf8(out.stdout)
        .map_err(|e| e.to_string())?
        .trim()
        .parse::<u32>()
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Replace the release `enable` stub with the real implementation**

Replace `#[cfg(not(debug_assertions))] pub fn enable() -> Result<(), String> { todo!("implement enable") }` with:

```rust
#[cfg(not(debug_assertions))]
pub fn enable() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let binary = exe.to_str().ok_or("non-UTF8 binary path")?;
    let plist = plist_path().ok_or("no home directory")?;
    if let Some(parent) = plist.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&plist, plist_content(binary)).map_err(|e| e.to_string())?;
    let uid = uid()?;
    let plist_str = plist.to_str().ok_or("non-UTF8 plist path")?;
    let out = std::process::Command::new("launchctl")
        .args(["bootstrap", &format!("gui/{uid}"), plist_str])
        .output()
        .map_err(|e| e.to_string())?;
    // 36 = EALREADY (already bootstrapped) — treat as success
    let code = out.status.code().unwrap_or(-1);
    if out.status.success() || code == 36 {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}
```

- [ ] **Step 3: Replace the `disable` stub with the real implementation**

Replace `pub fn disable() -> Result<(), String> { todo!("implement disable") }` with:

```rust
pub fn disable() -> Result<(), String> {
    let uid = uid()?;
    let out = std::process::Command::new("launchctl")
        .args(["bootout", &format!("gui/{uid}"), LABEL])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        if let Some(p) = plist_path() {
            let _ = std::fs::remove_file(p);
        }
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}
```

- [ ] **Step 4: Verify it compiles**

```
cargo check
```

Expected: no errors.

- [ ] **Step 5: Run tests — verify existing test still passes**

```
cargo test launch_at_login
```

Expected: `test launch_at_login::tests::plist_content_contains_label_and_binary ... ok`

- [ ] **Step 6: Commit**

```
git add src/launch_at_login.rs
git commit -m "feat: implement launch_at_login enable/disable"
```

---

### Task 4: Wire into main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add module declaration at the top of `src/main.rs`**

After `mod provider;` (line 3), add:

```rust
mod launch_at_login;
```

- [ ] **Step 2: Call `enable()` at the start of `main()`**

In `fn main()`, after `#[cfg(target_os = "macos")] set_accessory_policy();` and before `let event_loop = ...`, add:

```rust
if let Err(e) = launch_at_login::enable() {
    eprintln!("[launch_at_login] {e}");
}
```

- [ ] **Step 3: Verify it compiles**

```
cargo check
```

Expected: no errors.

- [ ] **Step 4: Run all tests**

```
cargo test
```

Expected: all 12 existing tests + `plist_content_contains_label_and_binary` pass.

- [ ] **Step 5: Commit**

```
git add src/main.rs
git commit -m "feat: call launch_at_login::enable at startup"
```

---

### Task 5: Manual acceptance test

**Files:** none

- [ ] **Step 1: Verify dev build skips registration**

```
make dev
```

Expected: stderr contains `[launch_at_login] skipped in debug build`

- [ ] **Step 2: Build release binary and verify plist is created**

```
cargo build --release
./target/release/aiusagebar &
sleep 1
cat ~/Library/LaunchAgents/com.mttpla.aiusagebar.plist
```

Expected: plist file exists, contains `<string>/path/to/target/release/aiusagebar</string>` and `<true/>` for RunAtLoad.

- [ ] **Step 3: Verify launchd registered the service**

```
launchctl list | grep aiusagebar
```

Expected: line containing `com.mttpla.aiusagebar`

- [ ] **Step 4: Kill the test process and verify it does NOT restart (KeepAlive false)**

```
pkill aiusagebar
sleep 2
launchctl list | grep aiusagebar
```

Expected: service still listed (registered) but not running (no PID column). KeepAlive false means launchd will not relaunch on quit.

- [ ] **Step 5: Log out and back in — verify the app auto-starts**

Log out of macOS session, log back in. Check menu bar for AIUsageBar icon.

- [ ] **Step 6: Clean up test binary registration before shipping**

```
launchctl bootout gui/$(id -u) com.mttpla.aiusagebar
rm ~/Library/LaunchAgents/com.mttpla.aiusagebar.plist
```

Use Homebrew-installed binary (not `target/release/`) for production — the plist will be re-written with the correct Homebrew path on first real launch.
