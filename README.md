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

## Development

### One-time setup (per machine)

**1. Create a self-signed code-signing certificate**

This prevents macOS from re-prompting for Keychain access on every recompile.

1. Open **Keychain Access** → menu bar: **Keychain Access → Certificate Assistant → Create a Certificate…**
2. Fill in:
   - **Name:** `AiUsageBar Dev` ← exact, case-sensitive
   - **Identity Type:** Self Signed Root
   - **Certificate Type:** Code Signing
3. Click **Create** → **Done**

The cert stays in your login keychain and is never committed to the repo.

**2. Run `make dev` for the first time**

```bash
make dev
```

macOS will prompt twice:
- *"codesign wants to access key 'AiUsageBar Dev'"* → enter your macOS password, click **Allow**
- *"aiusagebar wants to access 'Claude Code-credentials'"* → click **Always Allow**

From this point on, `make dev` starts the app with no dialogs.

### Daily workflow

```bash
make dev                 # build + sign + run (no Keychain prompts)
cargo build --release    # release binary
cargo check              # fast type-check
cargo clippy             # lint
```

Place a 32×32 PNG at `icons/app_icon.png` — app runs with a placeholder if missing.
