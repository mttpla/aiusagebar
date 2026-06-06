# AIUsageBar — Requirements

Business decisions and product choices. **No code, no implementation detail.** Update this file when a decision changes; plans must match what's here.

Items marked `❓` are open questions awaiting confirmation.

---

## 1. App identity

- **Name:** AIUsageBar
- **Platform:** macOS only (menu bar app)
- **Author:** Mttpla
- **License:** MIT
- **Repository:** https://github.com/mttpla/aiusagebar
- **Distribution:** open source public release

## 2. Core promise

- Read-only monitor of AI coding-tool usage quotas.
- **Never** sends prompts, **never** spends quota, **never** modifies credentials, **never** refreshes tokens.
- Failure of one provider must not break the others.

## 3. Supported providers

Three providers, displayed by **company name**, not tool name:

| Display label        | Source tool             | Credentials read from                                    |
|----------------------|-------------------------|----------------------------------------------------------|
| `OpenAI`             | Codex CLI               | `~/.codex/auth.json`                                     |
| `Anthropic`          | Claude Code             | macOS Keychain → fallback `~/.claude/.credentials.json`  |
| `GitHub (<login>)`   | GitHub Copilot          | macOS Keychain `copilot-cli` per logged-in user          |

Copilot supports multiple logged-in users → one menu section per user.

## 4. What each provider shows in the menu

### 4.1 OpenAI (Codex)
- Two rolling windows: **5h session** and **7d weekly**
- Per window: percent used, reset timestamp
- No absolute count (API doesn't expose limit/remaining)

### 4.2 Anthropic (Claude)
- Two rolling windows: **5h session** and **7d weekly**
- Per window: percent used, reset timestamp
- Pre-flight local expiry check: if access token expired → show `"Scaduto dal <date> — esegui: claude login"`, no HTTP call
- On HTTP 401 → show `"Token rifiutato — esegui: claude login"` (no auto-refresh)

### 4.3 GitHub (Copilot)
- One section per logged-in account
- Window: **monthly (premium)** — percent used, remaining/limit, reset date
- Optional windows: chat, completions — shown only if **not** unlimited
- On HTTP 401 → `"Token scaduto o revocato — ri-esegui login Copilot"`

### 4.4 Common states (all providers)
- `Not configured` — credentials file missing
- `Stale` — token rejected or expired; user action required
- `Error` — network/parse failure
- `Ok` — windows displayed

## 5. Tray icon

- Single static PNG asset
- Starting symbol: custom icon (SF Symbols excluded — licensing incompatible with open source distribution)
- Fallback if asset missing: blue placeholder square (dev only)
- Future: battery-style overlay reflecting worst `percent_used` across providers

## 6. Tooltip

- Single-line summary: `Provider: NN%` per provider, joined by `  ·  `
- For `Ok` state: use the **worst** percent across that provider's windows
- For other states: show a symbol (`–` not configured, `⚠` stale, `✕` error)

## 7. About window

Triggered by **"About AIUsageBar"** menu item. Native macOS alert.

Shows:
- App name
- Version (from `Cargo.toml` at build time)
- Author: Mttpla
- License: MIT
- Repository URL
- Short tagline: "A read-only monitor. Never sends prompts, never spends quota, never modifies credentials."

Version string: semantic version from `Cargo.toml` only — no git SHA.

## 8. Preferences / Settings

Submenu in the tray. Persists across restarts.

### 8.1 Per provider (OpenAI / Anthropic / GitHub)
- **Enabled / disabled** toggle — disabled providers hidden from menu, not polled
- **Polling interval** — user-selectable from a range
  - Minimum: **5 minutes** (300s)
  - Maximum: **60 minutes** (3600s)
  - Step: **1 minute** (60s)
  - All GitHub Copilot accounts share the same interval and enable flag

### 8.2 Global
- **Launch at Login** toggle (macOS launchd)
  - Only effective on a release build; dev builds show an error alert

### 8.3 Persistence
- Settings file in standard macOS config location (`~/Library/Application Support/aiusagebar/`)
- Path: `~/.config/aiusagebar/`

## 9. Quit

- Always-present "Quit" menu item exits cleanly.

## 10. Performance

- Idle CPU ~0% (event-driven, no busy loop)
- Provider polling capped by user interval (min 5 min)
- One provider's slow network call must not block the others — serial vs parallel fetch TBD (no performance concerns at 5-min cadence)

## 11. Security & privacy

- Credentials are **read-only**. Never write to Keychain or credential files.
- Never log token values to stdout/stderr.
- Never call refresh endpoints (Claude/Codex refresh tokens are single-use/rotating).
- Claude API requires a matching `User-Agent` header — wrong UA permanently blocks the token.
- Copilot token priority order:
  1. `COPILOT_GITHUB_TOKEN` env var (fine-grained PAT — preferred)
  2. `GH_TOKEN` env var
  3. `GITHUB_TOKEN` env var
  4. Keychain `copilot-cli`
  5. `~/.copilot/config.json`
  6. `~/.config/gh/hosts.yml`

## 12. Versioning & release

- Semver tags: `v0.1.0`, `v0.2.0`, …
- `Cargo.toml` version updated only by release script — never by hand
- `CHANGELOG.md` follows Keep a Changelog format
- Initial public version: `v1.0.0` — as soon as at least one provider shows live usage data

## 13. Internationalisation

- Multilingual: born Italian + English, adapts to the OS locale.
- All user-facing strings must be localised; no hardcoded language mix.
- Default locale: Italian (`it`). Supported at launch: `it`, `en`.
- Use the OS locale to select language; fall back to `en` if locale unsupported.

## 14. Out of scope (explicit non-goals)

- No Linux or Windows support
- No bundling of provider CLIs
- No telemetry, no analytics, no auto-update
- No prompt forwarding, no chat UI, no model selection
- No writing to credentials, no refresh flows
- No notification on threshold crossing (future enhancement, not in v1)

---

## Open questions summary

All previously open questions resolved. No pending decisions.
