# AIUsageBar — Implementation Brief

This document is the single source of truth for building the app. It is written
to be fed to an AI coding agent (Claude Code). Build **in the ordered steps
below**, one step at a time, verifying each before moving on.

> All code comments must be in **English**.

---

## 1. Goal

A macOS menu bar (system tray) app in Rust that shows AI coding quota usage
for Codex, Claude Code, and GitHub Copilot. Each provider exposes its own
limit windows — the trait does not force a single percentage, it lets each
provider return whatever windows it actually tracks.

Designed to accommodate additional providers in the future with no changes to
the core architecture.

---

## 2. Non-negotiable constraints

- **Tokens are read-only.** Never write to the Keychain or any credential file.
  Never call a refresh flow for Claude or Codex: their refresh tokens are
  single-use/rotating and refreshing them would log the user out of the
  official client.
- **Claude endpoint:** must send a correct `User-Agent` header (wrong or missing
  = instant, persistent HTTP 429). Minimum polling interval: 180 seconds.
- **Copilot:** prefer a user-supplied fine-grained PAT via `COPILOT_GITHUB_TOKEN`
  over the CLI's stored token. PATs don't rotate — no logout hazard.
- **Graceful degradation.** All three usage endpoints are unofficial. On any
  failure, return a state; never panic.

---

## 3. Architecture

### 3.1 State model

```rust
/// One time-bounded usage window reported by a provider.
/// Providers that track rolling windows (Codex, Claude) return multiple of
/// these. Providers with a single monthly quota (Copilot) return one or more.
pub struct LimitWindow {
    /// Human-readable label: "5h session", "7d weekly", "monthly", etc.
    pub name: String,
    /// Consumption as a percentage 0.0–100.0. None if the provider does not
    /// report a percentage for this window.
    pub percent_used: Option<f32>,
    /// Absolute quota for this window (e.g. 1500 for Copilot Pro+). None when
    /// the provider does not expose an entitlement figure.
    pub limit: Option<u32>,
    /// Absolute remaining amount. None when not reported.
    pub remaining: Option<u32>,
    /// When this window resets. None when unknown.
    pub resets_at: Option<DateTime<Utc>>,
    /// True when the provider reports this window as unlimited.
    pub unlimited: bool,
}

pub enum UsageState {
    /// No credentials found for this provider on this machine.
    NotConfigured,
    /// Credentials exist but the endpoint rejected them or the token expired.
    /// The string is a user-facing hint ("renew via Claude Code", etc.).
    Stale(String),
    /// Live data. One entry per window the provider tracks.
    Ok(Vec<LimitWindow>),
    /// Unexpected failure (network error, parse error, etc.).
    Error(String),
}

pub trait UsageProvider: Send + Sync {
    /// Short display name shown in the menu ("Codex", "Claude", "Copilot").
    fn name(&self) -> &'static str;
    /// Fetch current state. Implementations must enforce their own minimum
    /// polling interval and return a cached state if called too soon.
    fn fetch(&self) -> UsageState;
}
```

The UI layer iterates over a `Vec<Box<dyn UsageProvider>>`, calls `fetch()` on
each, and renders every window in `UsageState::Ok`. The worst
`percent_used` across all active providers drives the menu bar icon tint.

### 3.2 How each provider populates LimitWindow

**Codex** — `primary_window` (5h) and `secondary_window` (7d):
each becomes a `LimitWindow` with `percent_used` set to the endpoint's
utilization percentage and `resets_at` from the reset timestamp.
`limit` and `remaining` are `None` (endpoint does not expose them).

**Claude** — `five_hour` and `seven_day` windows:
each becomes a `LimitWindow`. `percent_used = Some(used_percentage)`,
`resets_at` from the ISO 8601 `resets_at` field.
`limit` and `remaining` are `None`.

**Copilot** — `premium_interactions` (always present) + `chat` and
`completions` if not `unlimited`:
- `percent_used = Some(100.0 - percent_remaining)`
- `limit = Some(entitlement)`, `remaining = Some(remaining)`
- `resets_at` from `quota_reset_date_utc`
- `unlimited = unlimited` (if `true`, skip the percentage in the display)
- If the plan also exposes session / weekly windows (April 2026+), add those
  as additional `LimitWindow` entries.

### 3.3 Suggested crates

| Crate | Use |
|---|---|
| `tray-icon`, `winit` | menu bar + event loop |
| `image` | load the PNG icon |
| `reqwest` (blocking feature) | HTTP |
| `serde`, `serde_json` | parse JSON responses and auth files |
| `security-framework` | read items from the macOS Keychain |
| `chrono` | parse and format reset timestamps |
| `dirs` | resolve `~` paths portably |

---

## 4. Provider details

### 4.1 Codex

**Token:**
Read and JSON-parse `~/.codex/auth.json`. Use the `accessToken` field.
No Keychain access required.

**Endpoint:**
```
GET https://chatgpt.com/backend-api/codex/usage
Authorization: Bearer <accessToken>
```

**Response shape (relevant fields):**
```json
{
  "primary_window":   { "utilization": 0.39, "resets_at": "..." },
  "secondary_window": { "utilization": 0.15, "resets_at": "..." }
}
```
`utilization` is a fraction 0–1; multiply by 100 for `percent_used`.

**Error handling:**
- File not found → `NotConfigured`
- HTTP 401 → `Stale("renew by running Codex CLI")`
- HTTP 429 → `Stale("rate limited; wait before retrying")`
- Network / parse error → `Error(message)`

---

### 4.2 Claude

**Token:**
1. Read Keychain generic-password:
   - service: `Claude Code-credentials`
   - account: current macOS username
   - Value is JSON: `{ "claudeAiOauth": { "accessToken": "sk-ant-oat01-...",
     "refreshToken": "sk-ant-ort01-...", "expiresAt": <epoch ms>,
     "scopes": [...] } }`
2. Fallback: read and parse `~/.claude/.credentials.json` (same schema).
3. Neither found → `NotConfigured`.

**First Keychain read triggers a macOS permission dialog** (the item belongs to
Claude Code). This is expected; the user clicks "Always Allow" once.

**Endpoint:**
```
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <accessToken>
User-Agent: <value matching the Claude Code CLI — reverse-engineer from binary
             or use the same string the official client sends>
```

**Minimum interval: 180 seconds.** Cache the last `UsageState` and return it
immediately on calls made before 180s have elapsed. Rate limiting is per
access token and aggressive; wrong or missing `User-Agent` yields instant
persistent 429s.

**Response shape (relevant fields):**
```json
{
  "five_hour": { "used_percentage": 39.0, "resets_at": "2026-06-06T14:00:00Z" },
  "seven_day":  { "used_percentage": 15.0, "resets_at": "2026-06-10T08:00:00Z" }
}
```

**Error handling:**
- HTTP 401 → `Stale("renew by running Claude Code (claude login)")`. Do NOT
  attempt refresh — the refresh token is single-use/rotating.
- HTTP 429 → respect back-off; return last cached state.
- Network / parse error → `Error(message)`

---

### 4.3 Copilot

**Token (check in this order — first hit wins):**
1. Env var `COPILOT_GITHUB_TOKEN` — recommended; use a fine-grained PAT with
   the "Copilot Requests" permission. No rotation, no Keychain prompt.
2. Env var `GH_TOKEN`
3. Env var `GITHUB_TOKEN`
4. Keychain generic-password, service `copilot-cli`
5. Plaintext file `~/.copilot/config.json`
6. gh CLI credentials at `~/.config/gh/hosts.yml`

None found → `NotConfigured`.

**Endpoint:**
```
GET https://api.github.com/copilot_internal/user
Authorization: Bearer <token>
```

**Response shape (relevant fields):**
```json
{
  "copilot_plan": "individual_pro",
  "quota_reset_date_utc": "2026-07-01T00:00:00.000Z",
  "quota_snapshots": {
    "premium_interactions": {
      "entitlement": 1500,
      "remaining": 1327,
      "percent_remaining": 88.5,
      "unlimited": false
    },
    "chat":        { "unlimited": true, ... },
    "completions": { "unlimited": true, ... }
  }
}
```

Build one `LimitWindow` per snapshot entry. For `unlimited: true`, set the
`unlimited` flag and skip `percent_used` / `limit` / `remaining`.

**Error handling:**
- HTTP 401 → `Stale("renew via Copilot CLI (copilot login) or set COPILOT_GITHUB_TOKEN")`
- Network / parse error → `Error(message)`

---

## 5. Build steps — do them in order

**Step 1 — Tray skeleton.** `cargo run` shows an icon in the menu bar (load
`icons/app_icon.png`, use a solid-colour fallback if missing). Menu has a
placeholder item and Quit. Clicking anything prints to the console. Event loop
uses `ControlFlow::Wait` — confirm ~0% CPU idle. No networking.

**Step 2 — State model + UI rendering.** Add `LimitWindow`, `UsageState`,
`UsageProvider`. Write a `StubProvider` that returns `Ok(vec![LimitWindow {
name: "5h session", percent_used: Some(39.0), ... }])`. Render per-provider
sections in the menu (name, each window's label + % + reset time) and a
one-line summary in the tooltip. Show distinct display for each state variant.

**Step 3 — Codex provider.** File read + endpoint + parse. Wire into the
provider list. Verify against a real `~/.codex/auth.json` if available.

**Step 4 — Claude provider.** Keychain read with `.credentials.json` fallback,
endpoint call with `User-Agent`, 180s minimum interval, and 401 → `Stale`.
Verify the Keychain prompt appears once and the windows render.

**Step 5 — Copilot provider.** Token-source priority chain, then the
`copilot_internal/user` call and `quota_snapshots` parsing. Test first with a
PAT in `COPILOT_GITHUB_TOKEN`.

**Step 6 — Background polling.** Drive all providers from the main event loop
using `ControlFlow::WaitUntil`. Global interval ≥ 180s (Claude's floor).
Each provider caches its own state and respects its own minimum interval
independently. Update menu and tooltip on each tick. All failures stay as
states — nothing crashes.

---

## 6. Acceptance checklist (run after each step)

- `cargo run` succeeds.
- Idle CPU ~0% (never `ControlFlow::Poll`).
- Token file / Keychain item absent → `NotConfigured`, not a panic.
- Expired / rejected token → `Stale`, not a panic, no refresh attempted.
- Claude never polled faster than 180s; always sends `User-Agent`.
- A provider returning `Error` does not affect the other providers.
