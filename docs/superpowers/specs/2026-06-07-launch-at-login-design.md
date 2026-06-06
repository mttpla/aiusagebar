# Launch at Login — Design Spec

**Date:** 2026-06-07
**Status:** Approved

## Goal

AiUsageBar auto-starts on macOS login when installed via Homebrew. The implementation is designed to support a future enable/disable toggle from the tray menu without rework.

## Approach

App self-registers via a LaunchAgent plist written to `~/Library/LaunchAgents/`. Registration happens at every startup (idempotent). No new Cargo dependencies.

## Architecture

New file `src/launch_at_login.rs`. Called from `main()` before the event loop.

```
src/
  launch_at_login.rs   — new module
  main.rs              — calls launch_at_login::enable() at startup
```

### Public API (sized for future toggle)

```rust
pub fn enable() -> Result<(), String>   // write plist + launchctl bootstrap
pub fn disable() -> Result<(), String>  // launchctl bootout + remove plist
pub fn is_enabled() -> bool             // plist file exists
```

## Plist

Written to `~/Library/LaunchAgents/com.mttpla.aiusagebar.plist` on every `enable()` call (overwrites — handles binary path changes on Homebrew upgrades).

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.mttpla.aiusagebar</string>
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
```

`{binary_path}` resolved at runtime via `std::env::current_exe()`.

`KeepAlive false`: launchd launches once at login; does not restart if user quits from the tray menu.

## launchctl Commands

Uses the modern macOS bootstrap domain (macOS 12+):

- **enable**: `launchctl bootstrap gui/<uid> <plist_path>`
- **disable**: `launchctl bootout gui/<uid> com.mttpla.aiusagebar`

`uid` obtained via `Command::new("id").arg("-u")` — no `libc` dependency.

Exit code 36 (`EALREADY`) on bootstrap is treated as `Ok(())` — service already loaded is not an error.

## Dev Build Guard

```rust
#[cfg(debug_assertions)]
pub fn enable() -> Result<(), String> {
    eprintln!("[launch_at_login] skipped in debug build");
    Ok(())
}
```

Satisfies REQUIREMENTS §8.2: "Only effective on a release build."

## Error Handling

Every failure is logged to stderr and returns `Err(String)`. The caller in `main()` logs and continues — the app always starts regardless. Failure points:

| Step | Failure | Behaviour |
|------|---------|-----------|
| `current_exe()` | Path unresolvable | `Err`, logged |
| `home_dir()` | No home directory | `Err`, logged |
| `fs::write()` | Permission denied | `Err`, logged |
| `id -u` | Command not found | `Err`, logged |
| `launchctl bootstrap` | Unknown exit code | `Err(stderr)`, logged |
| `launchctl bootstrap` | Exit 36 (EALREADY) | `Ok(())` |

## Startup Flow

```
main()
  └─ launch_at_login::enable()
       ├─ [debug build] → log + return Ok(())
       ├─ current_exe()  → binary path
       ├─ home_dir()     → plist directory
       ├─ fs::write()    → write plist
       ├─ id -u          → uid
       └─ launchctl bootstrap gui/<uid> <plist>
```

## main.rs Integration

```rust
if let Err(e) = launch_at_login::enable() {
    eprintln!("[launch_at_login] {e}");
}
```

## Testing

No automated test for live launchd interaction. One unit test: `plist_content()` is a pure function returning the XML string — tested independently of filesystem and launchctl.

## Out of Scope (future)

- Toggle UI in tray settings menu
- Settings persistence (`~/.config/aiusagebar/`)
- Per-provider enable/disable toggles
