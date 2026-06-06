# AIUsageBar

A macOS menu bar app that shows, at a glance, how much of your AI coding
quota you have left — across **Codex**, **Claude Code**, and **GitHub Copilot**,
with room to add more providers in the future.

One icon in the menu bar. Each provider reports its own limit windows (rolling
5-hour, 7-day, monthly quota, etc.) so you see the right number for the right
constraint, not a normalised average.

---

## What it does

- Displays the current usage for each configured AI provider.
- Each provider exposes one or more **limit windows** with the metrics it
  actually tracks (percentage, absolute remaining, reset time).
- Surfaces a clear per-provider state instead of silently failing:
  - **Not configured** — no credentials found; you probably don't use that provider.
  - **Stale** — credentials found but expired or rejected; renew via the official client.
  - **OK** — live data retrieved; shows each window and its reset time.
- Polls in the background at a safe interval and updates the menu/tooltip.

It is a **read-only monitor**. It never sends prompts, never spends quota,
and never modifies your stored credentials.

---

## Why it asks for Keychain access

Each official CLI already authenticates you and stores a token locally:

| Provider | Where its token lives                                  |
|----------|--------------------------------------------------------|
| Codex    | plain file at `~/.codex/auth.json`                     |
| Claude   | **macOS Keychain**, service `Claude Code-credentials`  |
| Copilot  | **macOS Keychain**, service `copilot-cli` (or a PAT)   |

For Codex the token is a plain file — no dialog needed. For **Claude and
Copilot the token lives in the macOS Keychain**. Because the Keychain item was
created by another application, the first time AIUsageBar reads it macOS shows:

> *"AIUsageBar" wants to use confidential information stored in
> "Claude Code-credentials" in your keychain.*

This is expected. Click **Always Allow** once; subsequent reads are silent.

The app:

- reads the token **only** to call the provider's usage endpoint;
- **never writes** anything back to the Keychain;
- **never refreshes or rotates** the token — doing so with Claude or Codex would
  invalidate the official client's copy and force a re-login (their OAuth
  refresh tokens are single-use and rotating);
- does not transmit the token anywhere except the provider's own API.

Clicking **Deny** shows **Stale** for that provider. Nothing else breaks.

### Avoiding the Keychain prompt for Copilot

Set a fine-grained Personal Access Token with the **Copilot Requests**
permission in the environment before launching the app:

```bash
export COPILOT_GITHUB_TOKEN=github_pat_...
```

This is the recommended approach — PATs don't rotate, so there is no logout
hazard.

---

## How each provider is read

| Provider | Token source                           | Usage endpoint                                        |
|----------|----------------------------------------|-------------------------------------------------------|
| Codex    | `~/.codex/auth.json`                   | `GET https://chatgpt.com/backend-api/codex/usage`     |
| Claude   | Keychain `Claude Code-credentials`     | `GET https://api.anthropic.com/api/oauth/usage`       |
| Copilot  | PAT env var, or Keychain `copilot-cli` | `GET https://api.github.com/copilot_internal/user`    |

Each provider exposes the limit windows that make sense for its own model:

- **Codex / Claude** — rolling 5-hour session window + rolling 7-day window,
  each with utilization % and reset time.
- **Copilot** — monthly premium-request quota (entitlement, remaining, %,
  reset date); session and weekly limits also shown if present on the plan.

---

## Requirements

- macOS 11+
- Rust 1.75+ (`rustup update`)
- At least one provider's CLI logged in, or a Copilot PAT.

---

## Build & run

```bash
cargo run
cargo build --release
```

Place a 32×32 PNG at `icons/app_icon.png`. If the file is missing the app uses
a placeholder so it still runs.

---

## Design constraints (read before touching auth code)

1. **Tokens are read-only.** Never write to the Keychain or any credential file.
2. **No token refresh for Claude and Codex.** Their refresh tokens are
   single-use/rotating. Refreshing would log the user out of their coding tool.
   On a 401, show **Stale** and wait for the user to renew via the official client.
3. **Copilot via PAT** has no rotation hazard. Prefer it over the stored token.
4. **Claude endpoint requires a correct `User-Agent`** — a wrong one yields
   instant, persistent HTTP 429. Minimum polling interval: 180 seconds.
5. **Graceful degradation.** All three usage endpoints are unofficial and can
   disappear. On any failure, set a state; never panic.

---

## Limitations

- For Claude and Codex, access tokens expire in ~1 hour. The app shows
  **Stale** until the user runs the official client again.
- All usage endpoints are unofficial and may change without notice.
