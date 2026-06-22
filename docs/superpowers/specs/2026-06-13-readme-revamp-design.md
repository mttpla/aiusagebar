# README revamp for 1.0.0

## Refresh — 2026-06-22 (read this first; supersedes stale parts below)

The README evolved a lot since the original spec. Ground truth as of 2026-06-22:

**Already shipped (drop from scope):**
- OpenAI/Codex references removed (`rg -i "openai|codex" README.md` → nothing).
- Provider naming unified to **Claude** / **Copilot**.
- `assets/demo.png` embedded under the tagline.
- Troubleshooting section exists — but uses the **diagnostic-log** approach
  (Other ▸ Diagnostics ▸ Copy diagnostic log), not the static symptom/cause/fix
  table the original Design proposed. Keep the diagnostic-log approach.
- An Install section exists.

**Distribution model changed — the original DMG premise is dead:**
- The original Design (sections 5 "Install" and 6 "First run") assumed a signed
  `.app` inside a DMG from card #11. Reality: distribution is a **raw
  `aiusagebar-macos-arm64-vX.Y.Z` binary** + a Gatekeeper workaround
  (right-click Open, or `xattr -dr com.apple.quarantine`).
- DMG / `.app` bundling is now a **separate backlog card (#42
  `dmg-app-bundle-distribution`)**. When it lands, a follow-up updates the Install
  section. This card documents the **binary** install path only.
- Consequence: original sections 5 (DMG Install) and 6 (DMG first-run walkthrough)
  below are superseded by the binary install path. The full multi-step `.app`
  walkthrough and the `tray-icon.png` capture are **descoped** to a future card.
  **In scope for this card (decided 2026-06-22):** two dialog screenshots that
  apply to the binary path — `assets/gatekeeper-prompt.png` (the "unidentified
  developer" / right-click→Open dialog) and `assets/keychain-prompt.png` (the
  Keychain "Always Allow" dialog). These remove the two scariest first-run
  surprises. Embed `gatekeeper-prompt.png` in the Install section and
  `keychain-prompt.png` in the Keychain access section.

**Remaining work for this card (the actual scope now):**
1. `LICENSE` file (MIT) — absent. Copyright line `Copyright (c) 2026 mttpla`.
2. `license = "MIT"` in `Cargo.toml` `[package]` — absent.
3. **License** section in README — absent.
4. Badges (latest release + License: MIT) — absent.
5. **Keychain access** depth — current text is one short paragraph; expand to the
   why / what / where-it-never-goes form (original section 8 still valid).
6. **Configuration** — surface the full Copilot token priority chain
   (`COPILOT_GITHUB_TOKEN` → `GH_TOKEN` → `GITHUB_TOKEN` → Keychain `copilot-cli`
   → `~/.copilot/config.json` → `~/.config/gh/hosts.yml`); currently only
   `COPILOT_GITHUB_TOKEN` is shown (inside the Keychain section).
7. **Requirements / Development** — `Rust 1.75+` is still under top-level
   Requirements; move it under Development (end users don't need a Rust toolchain).
8. **Screenshots** — capture and commit `assets/gatekeeper-prompt.png` (Install
   section) and `assets/keychain-prompt.png` (Keychain section). Redact any
   personal info (email, account names) before committing.
9. **Releasing** section is **stale**: `scripts/release.sh` now bumps + changelog
   + commits + tags + **pushes** + `cargo build --release` + ad-hoc
   `codesign` + `gh release create` with the binary uploaded. README still says
   "Push manually after reviewing" and omits the build/sign/release steps. Rewrite
   to match the actual script.

Verification checklist (original "Tests" section) still applies for items 1–4.
Sections of the original Design below remain authoritative **except** Install (5)
and First run (6), which the binary-distribution model supersedes.

---

## Problem

`README.md` (118 lines) is contributor-oriented and contains stale information:

- Tagline and provider table mention **OpenAI / Codex**, which has no provider implementation in the codebase.
- Provider names are mixed: the table says "OpenAI / Anthropic / GitHub", the tray menu and code modules use "Claude / Copilot".
- There is no end-user install path. The only install instructions assume `cargo` and a self-signed code-signing cert — that workflow is for contributors, not users.
- The Keychain paragraph (line 35) tells the user a dialog will appear but does not explain *why*, *what* is read, or *where* the token lives.
- No Troubleshooting section. When a provider goes `Stale` or `Error`, the user has no documented recovery path.
- No License section. No `LICENSE` file in the repo. No `license` field in `Cargo.toml`.
- "Rust 1.75+" is under user-facing **Requirements** — a non-contributor downloading the DMG does not need a Rust toolchain.
- No first-run walkthrough. The Keychain dialog is the very first thing a user sees; they get one paragraph of warning.

## Goal

Rewrite the README so a non-contributor can:

1. Understand in 30 seconds what the app does and what it does *not* do (read-only, never sends prompts, never spends quota).
2. Install via DMG (downloaded from Releases).
3. Complete first-run without confusion: Keychain dialog appears, "Always Allow", and the tray icon's three states are explained.
4. Recover from common failures (provider stale, token expired, missing CLI login).

Contributor content (`make dev`, self-signed cert, releasing) stays — moved into a clearly-labeled **Development** section below the end-user content.

## Non-goals

- Implementing or re-adding a Codex provider. That is a separate effort; when it lands, a follow-up card re-introduces OpenAI/Codex content.
- Sparkle-style auto-update wording. Card #20 handles the auto-update-check feature; this card does not pre-announce it.
- Architecture documentation for contributors. `CLAUDE.md` already covers that.
- Italian translation or i18n of the README.
- A CHANGELOG link from the README. `CHANGELOG.md` is visible in the repo tree.
- A "1.0.0 in progress" badge. Pre-1.0 marker is the version number itself.

## Design

### New top-to-bottom section order

```
1. Title + tagline + screenshot
2. Badges (latest release, build status, license)
3. What it does + privacy/security one-liner
4. Providers (table — Claude, Copilot)
5. Install (end user, DMG)
6. First run (Keychain dialog, Always Allow, tray icon states)
7. Configuration (Copilot env vars, optional)
8. Keychain access (why, what, where, never written)
9. Troubleshooting
10. Development (cargo, make dev, self-signed cert, Rust 1.75+)
11. Releasing (existing content — moved here, otherwise unchanged)
12. License
```

### Section content rules

**1. Title + tagline + screenshot**
- Tagline drops "OpenAI". New form: "macOS menu bar app that shows AI coding quota usage for **Claude** and **GitHub Copilot**."
- Embed `assets/demo.png` (created by card #10) directly under tagline.

**2. Badges**
- Latest release version (`shields.io` GitHub release badge).
- Build status (GitHub Actions, if a workflow exists — otherwise omit, do not add a fake badge).
- License badge: `License: MIT`.

**3. What it does**
- One paragraph. Mention: tray-only, ≈0% idle CPU, local OS only (no telemetry, no network beyond provider APIs), read-only.

**4. Providers**

| Provider | CLI prerequisite                                  | Limit windows                       |
|----------|---------------------------------------------------|-------------------------------------|
| Claude   | [`claude` CLI](https://docs.anthropic.com/claude-code) logged in | 5h session · 7d weekly |
| Copilot  | [`gh` CLI](https://cli.github.com) with Copilot extension *or* `COPILOT_GITHUB_TOKEN` env var | Monthly premium quota (per account) |

Per-provider states: **Not configured** · **Stale** (renew via official client) · **OK** · **Error**.

**5. Install**

End-user path, assumes DMG from card #11:

1. Download `AIUsageBar-vX.Y.Z.dmg` from the [Releases page](https://github.com/<owner>/<repo>/releases/latest).
2. Open the DMG, drag **AIUsageBar.app** to **Applications**.
3. Launch from Launchpad or Spotlight.

No Rust toolchain required. No Xcode required.

**6. First run**

Step-by-step with screenshots. Three screenshots:

| File                        | Shows                                       |
|----------------------------|--------------------------------------------|
| `assets/keychain-prompt.png`| macOS Keychain dialog requesting access to `Claude Code-credentials` |
| `assets/tray-icon.png`     | Brain icon in the menu bar after launch    |
| `assets/menu-open.png`     | Open menu with Claude + Copilot rows (= card #10's `demo.png`, same image) |

To avoid duplication, the file is committed once as `assets/demo.png` (card #10's filename) and the README references the same file from both "Title" and "First run — menu open" callouts.

Walkthrough text:
1. First launch — macOS shows a Keychain dialog: "aiusagebar wants to access 'Claude Code-credentials'". Click **Always Allow**. (screenshot: `keychain-prompt.png`)
2. The brain icon appears in the menu bar. (screenshot: `tray-icon.png`)
3. Click the icon to see provider sections. (screenshot: `demo.png`)
4. Three tray icon states:
   - Brain (white): all providers under 80% usage
   - Brain + red dot: one or more providers at ≥80%
   - Brain + grey dot: data unavailable (not configured / stale / error)

**7. Configuration**

Single subsection: Copilot token priority.

- `COPILOT_GITHUB_TOKEN` (recommended for fine-grained PAT)
- `GH_TOKEN`
- `GITHUB_TOKEN`
- Keychain `copilot-cli`
- `~/.copilot/config.json`
- `~/.config/gh/hosts.yml`

Show the env-var export form from the current README.

**8. Keychain access**

Three short paragraphs:

- *Why:* macOS sandboxes Keychain items per creating app. AIUsageBar is not Claude Code, so reading Claude's stored OAuth token requires explicit user consent.
- *What:* the Claude OAuth access token (item service `Claude Code-credentials`, account = current macOS username, JSON value with `claudeAiOauth.accessToken`).
- *Where it never goes:* the token is never written, never logged, never sent anywhere except `api.anthropic.com` for the documented `/api/oauth/usage` and `/api/oauth/profile` endpoints. Fallback path if Keychain is unavailable: `~/.claude/.credentials.json` (created by Claude Code).

**9. Troubleshooting**

Table form:

| Symptom | Likely cause | Fix |
|--------|--------------|-----|
| "Claude: not configured" | Claude Code CLI not logged in | Run `claude` and complete OAuth in the browser |
| "Claude ⚠ stale" | OAuth token rotated or expired | Open Claude Code; it refreshes the token. AIUsageBar reads it on next poll. |
| "Copilot: not configured" | No GitHub token found | Set `COPILOT_GITHUB_TOKEN`, or `gh auth login` with Copilot scope |
| "Copilot ⚠ token expired, re-login" | PAT revoked or `gh` token invalid | Regenerate PAT or re-run `gh auth login` |
| All providers show ✕ Error | Network down or DNS | Check connectivity; AIUsageBar polls every 180s |
| App icon shows grey dot all the time | First-run Keychain dialog dismissed | Quit + relaunch; click **Always Allow** when prompted |

**10. Development**

This section is contributors-only and starts with a heading note: *"Skip this section if you installed via DMG."*

Then move the existing **Development** content (one-time setup, `make dev`, daily workflow) here. Two edits:

- Daily workflow lists only `make dev` and the bare `cargo` commands actually used. Do not show `cargo run` (it triggers Keychain re-prompts because the binary is unsigned).
- Move **Rust 1.75+** out of the top-level Requirements and into this section's prerequisites.

**11. Releasing**

Existing content (`./scripts/release.sh patch|minor|major`, git-cliff dependency) preserved unchanged.

**12. License**

One paragraph: "Released under the MIT License. See [LICENSE](LICENSE)."

### Provider naming convention

- Primary names: **Claude** and **Copilot**.
- First mention of each may say "Claude (Anthropic)" / "Copilot (GitHub)" for clarity.
- Subsequent mentions use the primary name only.
- Tray menu code already uses Claude / Copilot — README aligns.

### Files touched

| File | Change |
|---|---|
| `README.md` | Full rewrite per section order above. |
| `LICENSE` | New. MIT text with copyright line `Copyright (c) 2026 <owner>`. |
| `Cargo.toml` | Add `license = "MIT"` to `[package]`. |
| `assets/keychain-prompt.png` | New screenshot. |
| `assets/tray-icon.png` | New screenshot. |
| `assets/demo.png` | Provided by card #10. Reused for "First run — menu open". This card does not capture it. |

### Blockers

- **Card #10** (`ui-readme-screenshot`) — provides `assets/demo.png`. Section 6 ("First run — menu open") cannot be finalised without it.
- **Card #11** (`release-sign-notarize-dmg`) — provides the DMG artifact. Section 5 ("Install") cannot reference a real download URL or DMG filename pattern without it.

Both blockers are hard. The card itself can be drafted earlier on the unblocked sections (Codex removal, Configuration, Keychain, Troubleshooting, Development, Releasing, License) but is **not closable** until #10 and #11 land.

## Tests

No automated tests. Verification is a visual review checklist:

1. `rg -i "openai|codex" README.md` returns nothing.
2. All occurrences of `Anthropic` and `GitHub` (as primary provider names) replaced with `Claude` / `Copilot`.
3. `LICENSE` exists and the year is correct.
4. `cargo metadata --format-version 1 | jq '.packages[0].license'` returns `"MIT"`.
5. All three image links resolve (open README in GitHub preview).
6. Every link in the Providers table resolves.
7. README renders without table-formatting glitches in GitHub preview.

## Rollout

Single PR. No code changes outside `Cargo.toml`'s `license` field. Behavior change is documentation-only.

When blockers land, the PR for this card lands last (or rebases on top of them).
