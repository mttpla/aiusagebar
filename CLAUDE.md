# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### One-time dev setup (per machine)

Create a self-signed code-signing certificate to avoid repeated Keychain prompts:

1. Open Keychain Access → Certificate Assistant → Create a Certificate
2. Name: `AiUsageBar Dev`, Identity Type: Self Signed Root, Certificate Type: Code Signing
3. Click Create → Done

This is required once per machine. Not committed to the repo.

### Daily commands

```bash
make dev                  # build + sign + run in dev (no repeated Keychain prompts after first run)
cargo build --release     # release binary
cargo check               # fast type-check without linking
cargo clippy              # lint
```

12 unit tests. Manual acceptance: idle CPU ~0% (`Activity Monitor`), Claude provider renders correct usage state.

## Architecture

macOS menu bar app (tray-icon + winit event loop). **Plan 1 complete** — Claude provider live, showing real usage data in the menu. Codex and Copilot providers not yet built. UI polish is a separate plan.

### Current module structure

```
src/
  main.rs              — event loop, tray setup, dynamic menu from providers
  http.rs              — generic GET helper, HttpError (reused by all providers)
  keychain.rs          — macOS Keychain generic-password reader
  provider/
    mod.rs             — UsageProvider trait, UsageState, LimitWindow types
    claude.rs          — reads Keychain / ~/.claude/.credentials.json, calls api.anthropic.com
```

### Crates

| Crate | Use |
|---|---|
| `tray-icon`, `winit` | menu bar + event loop |
| `image` | load PNG icon |
| `reqwest` (blocking, rustls-tls) | HTTP |
| `serde`, `serde_json` | parse JSON responses and auth files |
| `security-framework` | macOS Keychain |
| `chrono` | parse/format reset timestamps |
| `dirs` | resolve `~` paths |

### Core types (implemented)

```rust
pub struct LimitWindow {
    pub name: String,
    pub percent_used: Option<f32>,
    pub limit: Option<u32>,
    pub remaining: Option<u32>,
    pub resets_at: Option<String>,
    pub unlimited: bool,
}

pub enum UsageState {
    NotConfigured,
    Stale(String),
    Ok(Vec<LimitWindow>),
    Error(String),
}

pub trait UsageProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn fetch(&self) -> UsageState;
}
```

UI iterates `Vec<Box<dyn UsageProvider>>`, renders per-provider menu sections.

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
