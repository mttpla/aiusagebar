# AIUsageBar

macOS menu bar app that shows AI coding quota usage for **Claude** and **Copilot**.

Read-only monitor. Never sends prompts, never spends quota, never modifies credentials.

![Demo screenshot](assets/demo.png)

---

## Installation

Download `aiusagebar-macos-arm64-vX.Y.Z` from [GitHub Releases](https://github.com/mttpla/aiusagebar/releases), then:

```bash
chmod +x aiusagebar-macos-arm64-vX.Y.Z
mv aiusagebar-macos-arm64-vX.Y.Z /usr/local/bin/aiusagebar   # or any directory in $PATH
aiusagebar &
```

**First launch — Gatekeeper warning:** macOS blocks unsigned downloads. Two workarounds (pick one):

- Right-click the binary in Finder → **Open** → confirm in the dialog.
- Or from Terminal: `xattr -dr com.apple.quarantine /usr/local/bin/aiusagebar`

After the first launch the warning never appears again.

---

## Providers

| Provider | Limit windows                       |
|----------|-------------------------------------|
| Claude   | 5h session · 7d weekly              |
| Copilot  | Monthly premium quota (per account) |

Per-provider states: **Not configured** · **Stale** (renew via official client) · **OK** · **Error**.

---

## Icons

Icons by [Font Awesome](https://fontawesome.com) (CC BY 4.0).

| Tray icon | Meaning |
|---|---|
| Brain (white) | All AI usage under 80% |
| Brain + red dot | At least one provider at or above 80% usage |
| Brain (grey dot) | Data unavailable — not configured, stale, or fetch error |

---

## Keychain access

Claude token lives in the macOS Keychain (created by Claude Code). First read triggers a system dialog — click **Always Allow** once. Nothing is ever written back.

To avoid the Copilot Keychain prompt, set a fine-grained PAT before launching:

```bash
export COPILOT_GITHUB_TOKEN=github_pat_...
```

---

## Troubleshooting

If a provider row shows an error, open **Other ▶ Diagnostics ▶ Copy diagnostic log**
to copy the full diagnostic log to your clipboard. Paste it into a GitHub issue or email
when reporting a bug. The Diagnostics submenu is hidden when there is nothing to report.

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

### Releasing a new version

**Prerequisite:** `git-cliff` must be on PATH.

```bash
brew install git-cliff   # once
```

Then from the repo root:

```bash
./scripts/release.sh patch   # 0.1.0 → 0.1.1
./scripts/release.sh minor   # 0.1.0 → 0.2.0
./scripts/release.sh major   # 0.1.0 → 1.0.0
```

The script:
1. Prompts for confirmation
2. Bumps the version in `Cargo.toml`
3. Regenerates `CHANGELOG.md` via `git-cliff`
4. Commits both files (`chore(release): vX.Y.Z`)
5. Creates the git tag `vX.Y.Z`

Push manually after reviewing:

```bash
git push && git push --tags
```
