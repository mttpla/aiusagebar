# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo run                 # run in dev (icon loads from icons/app_icon.png relative to CWD)
cargo build --release     # release binary
cargo check               # fast type-check without linking
cargo clippy              # lint
```

No test suite yet. Manual acceptance: idle CPU ~0% (`Activity Monitor`), all three providers render correct states.

## Architecture

macOS menu bar app (tray-icon + winit event loop). Currently at **Step 1** of the implementation plan — skeleton only (`src/main.rs`). Steps 2–6 from `IMPLEMENTATION.md` are not yet built.

### Planned module structure (IMPLEMENTATION.md)

```
src/
  main.rs          — event loop, tray setup, polling tick
  provider/
    mod.rs         — UsageProvider trait, UsageState, LimitWindow types
    codex.rs       — reads ~/.codex/auth.json, calls chatgpt.com endpoint
    claude.rs      — reads Keychain / ~/.claude/.credentials.json, calls api.anthropic.com
    copilot.rs     — token priority chain, calls api.github.com/copilot_internal/user
```

### Core types (not yet implemented)

- `LimitWindow` — one time-bounded usage window (name, percent_used, limit, remaining, resets_at, unlimited)
- `UsageState` — `NotConfigured | Stale(String) | Ok(Vec<LimitWindow>) | Error(String)`
- `UsageProvider` trait — `name() -> &'static str`, `fetch() -> UsageState`

UI iterates `Vec<Box<dyn UsageProvider>>`, renders per-provider menu sections, drives icon tint from worst `percent_used`.

### Hard constraints (read before touching auth or network code)

1. **Tokens are read-only.** Never write to Keychain or credential files.
2. **No token refresh for Claude or Codex.** Their refresh tokens are single-use/rotating — refreshing logs the user out of the official client. On 401: `Stale`, not refresh.
3. **Claude endpoint: 180s minimum poll interval** and must send a matching `User-Agent`. Wrong/missing UA = instant persistent HTTP 429.
4. **Copilot: prefer `COPILOT_GITHUB_TOKEN` env var** (fine-grained PAT) over Keychain. Token priority: `COPILOT_GITHUB_TOKEN` → `GH_TOKEN` → `GITHUB_TOKEN` → Keychain `copilot-cli` → `~/.copilot/config.json` → `~/.config/gh/hosts.yml`.
5. **Graceful degradation.** Every provider failure is a state (`NotConfigured`/`Stale`/`Error`), never a panic. One provider failing must not affect others.

### Keychain access

Claude token: service `Claude Code-credentials`, account = current macOS username, JSON value with `claudeAiOauth.accessToken`. Fallback: `~/.claude/.credentials.json`.

First Keychain read triggers a macOS dialog (item belongs to Claude Code app) — expected behavior, user clicks "Always Allow" once.

### Event loop pattern

`ControlFlow::Wait` (not `Poll`) for ~0% idle CPU. Background polling uses `ControlFlow::WaitUntil` once Step 6 is implemented. Global poll interval ≥ 180s (Claude's floor); each provider caches its own last state.
