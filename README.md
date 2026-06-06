# AIUsageBar

macOS menu bar app that shows AI coding quota usage for **OpenAI**, **Anthropic**, and **GitHub** Copilot.

Read-only monitor. Never sends prompts, never spends quota, never modifies credentials.

---

## Providers

| Provider   | Limit windows                          |
|------------|----------------------------------------|
| OpenAI     | 5h session · 7d weekly                 |
| Anthropic  | 5h session · 7d weekly                 |
| GitHub     | Monthly premium quota (per account)    |

Per-provider states: **Not configured** · **Stale** (renew via official client) · **OK** · **Error**.

---

## Keychain access

Claude token lives in the macOS Keychain (created by Claude Code). First read triggers a system dialog — click **Always Allow** once. Nothing is ever written back.

To avoid the Copilot Keychain prompt, set a fine-grained PAT before launching:

```bash
export COPILOT_GITHUB_TOKEN=github_pat_...
```

---

## Requirements

- macOS 11+
- Rust 1.75+ (`rustup update`)
- At least one provider's CLI logged in, or a Copilot PAT

---

## Build & run

```bash
cargo run            # dev
cargo build --release
```

Place a 32×32 PNG at `icons/app_icon.png` — app runs with a placeholder if missing.
