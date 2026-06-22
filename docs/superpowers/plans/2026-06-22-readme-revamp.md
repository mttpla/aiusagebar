# README Revamp for 1.0.0 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring `README.md` to a 1.0-ready, end-user-first state: MIT license, badges, clear binary install with Gatekeeper guidance, deeper Keychain explainer, a Configuration section for Copilot tokens, contributor content under Development, and a Releasing section that matches `scripts/release.sh`.

**Architecture:** Documentation-only change plus one `Cargo.toml` `license` field and one new `LICENSE` file. No Rust source changes, no behavior change. Two manually-captured screenshots are committed under `assets/`.

**Tech Stack:** Markdown, shields.io badges, MIT license text, macOS screenshot capture.

> **Progress — 2026-06-22:** Tasks 2, 3, 6, 7, 8 done and pushed to `master` (LICENSE + license metadata; badges; Configuration section; Rust→Development; Releasing rewrite; License section). The `COPILOT_GITHUB_TOKEN` block was moved out of Keychain into Configuration as part of this pass. **Remaining:** Task 1 (human captures `gatekeeper-prompt.png` + `keychain-prompt.png` — agent cannot screenshot system dialogs), then Tasks 4 (Install + Gatekeeper screenshot) and 5 (Keychain screenshot embed). Card stays in `doing` until those land.

## Global Constraints

- Provider names are **Claude** and **Copilot** only. First mention may say "Claude (Anthropic)" / "Copilot (GitHub)"; subsequent mentions use the primary name. No "OpenAI" / "Codex" / "Anthropic" / "GitHub" as a primary provider name.
- `rg -i "openai|codex" README.md` must return nothing.
- Distribution is a **raw `aiusagebar-macos-arm64-vX.Y.Z` binary** from GitHub Releases. Do **not** describe a DMG or `.app` bundle (that is separate card #42).
- GitHub coordinates: `mttpla/aiusagebar`.
- No CI workflow exists (`.github/workflows/` absent) → do **not** add a build-status badge.
- All README / code / comment text in English.
- `cargo clippy -- -D warnings && cargo test` must pass before any commit that touches `Cargo.toml` (the lock/metadata change).

---

### Task 1: Capture and commit the two dialog screenshots

**Manual task — a human must capture the macOS dialogs; an agent cannot screenshot system dialogs.** Embedding tasks (4, 5) depend on these files existing so the images render.

**Files:**
- Create: `assets/gatekeeper-prompt.png`
- Create: `assets/keychain-prompt.png`

**Interfaces:**
- Produces: two committed PNGs at the exact paths above, referenced by Tasks 4 and 5.

- [ ] **Step 1: Capture the Gatekeeper dialog**

Trigger the "unidentified developer" / right-click→Open dialog for an unsigned downloaded binary:
1. Build a release binary: `cargo build --release`
2. Add the quarantine attribute to simulate a download: `xattr -w com.apple.quarantine "0081;00000000;Safari;" target/release/aiusagebar`
3. In Finder, right-click the binary → **Open**. macOS shows the "macOS cannot verify the developer" dialog.
4. Screenshot just that dialog: `Cmd-Shift-4`, then `Space`, click the dialog.

- [ ] **Step 2: Capture the Keychain dialog**

The "aiusagebar wants to access 'Claude Code-credentials'" dialog appears on first Keychain read. If already granted: open **Keychain Access**, find `Claude Code-credentials`, delete the access-control grant for `aiusagebar` (or run the unsigned binary so the prompt re-appears), then `Cmd-Shift-4` + `Space` to capture the dialog.

- [ ] **Step 3: Redact and place the files**

Cover any personal info (macOS username, email, account names) with a solid fill. Save as `assets/gatekeeper-prompt.png` and `assets/keychain-prompt.png`.

- [ ] **Step 4: Verify the files exist**

Run: `ls -1 assets/gatekeeper-prompt.png assets/keychain-prompt.png`
Expected: both paths listed, no error.

- [ ] **Step 5: Commit**

```bash
git add assets/gatekeeper-prompt.png assets/keychain-prompt.png
git commit -m "docs: add Gatekeeper and Keychain first-run screenshots"
```

---

### Task 2: Add the LICENSE file, Cargo.toml license field, and README License section

**Files:**
- Create: `LICENSE`
- Modify: `Cargo.toml:1-4` (`[package]` block)
- Modify: `README.md` (append a `## License` section at the end)

**Interfaces:**
- Produces: `License: MIT` referenced by the license badge in Task 3.

- [ ] **Step 1: Create the LICENSE file**

Create `LICENSE` with the standard MIT text:

```
MIT License

Copyright (c) 2026 mttpla

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- [ ] **Step 2: Add the license field to Cargo.toml**

In `Cargo.toml`, add the `license` line to `[package]`:

```toml
[package]
name = "aiusagebar"
version = "0.5.0"
edition = "2021"
license = "MIT"
```

- [ ] **Step 3: Add the License section to README**

Append to the end of `README.md`:

```markdown
---

## License

Released under the MIT License. See [LICENSE](LICENSE).
```

- [ ] **Step 4: Verify the license metadata**

Run: `cargo metadata --format-version 1 | jq '.packages[] | select(.name=="aiusagebar") | .license'`
Expected: `"MIT"`

Run: `test -f LICENSE && echo OK`
Expected: `OK`

- [ ] **Step 5: Quality gate (Cargo.toml / lock changed)**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: clippy clean, all tests pass.

- [ ] **Step 6: Commit**

```bash
git add LICENSE Cargo.toml Cargo.lock README.md
git commit -m "docs: add MIT LICENSE, license field, and README License section"
```

---

### Task 3: Add release and license badges under the title

**Files:**
- Modify: `README.md:1-9` (between tagline block and the `![Demo screenshot]` line)

**Interfaces:**
- Consumes: `License: MIT` from Task 2 (license badge links to `LICENSE`).

- [ ] **Step 1: Insert the badge line**

In `README.md`, insert a badges line immediately after the
"Read-only monitor..." paragraph and before `![Demo screenshot](assets/demo.png)`:

```markdown
[![Latest release](https://img.shields.io/github/v/release/mttpla/aiusagebar)](https://github.com/mttpla/aiusagebar/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
```

Do not add a build-status badge — no CI workflow exists.

- [ ] **Step 2: Verify the badges**

Run: `grep -c "img.shields.io" README.md`
Expected: `2`

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add release and license badges"
```

---

### Task 4: Rewrite the Install section with the Gatekeeper screenshot

**Files:**
- Modify: `README.md:11-26` (the `## Installation` section)

**Interfaces:**
- Consumes: `assets/gatekeeper-prompt.png` from Task 1.

- [ ] **Step 1: Replace the Installation section**

Replace the current `## Installation` section (download + chmod + Gatekeeper text) with:

```markdown
## Install

Download `aiusagebar-macos-arm64-vX.Y.Z` from the
[Releases page](https://github.com/mttpla/aiusagebar/releases/latest), then:

```bash
chmod +x aiusagebar-macos-arm64-vX.Y.Z
mv aiusagebar-macos-arm64-vX.Y.Z /usr/local/bin/aiusagebar   # or any directory in $PATH
aiusagebar &
```

No Rust toolchain and no Xcode required.

### First launch — Gatekeeper

macOS blocks unsigned downloads. The first time you launch, you will see an
"unidentified developer" dialog. Pick one workaround:

- Right-click the binary in Finder → **Open** → confirm in the dialog.
- Or from Terminal: `xattr -dr com.apple.quarantine /usr/local/bin/aiusagebar`

![Gatekeeper dialog](assets/gatekeeper-prompt.png)

After the first launch the warning never appears again.
```

- [ ] **Step 2: Verify**

Run: `grep -q "assets/gatekeeper-prompt.png" README.md && grep -qi "^## Install$" README.md && echo OK`
Expected: `OK`

Run: `rg -i "openai|codex|\.dmg|\.app bundle" README.md || echo CLEAN`
Expected: `CLEAN`

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: rewrite Install section with Gatekeeper screenshot"
```

---

### Task 5: Deepen the Keychain section with the Keychain screenshot

**Files:**
- Modify: `README.md:53-62` (the `## Keychain access` section)

**Interfaces:**
- Consumes: `assets/keychain-prompt.png` from Task 1.
- Produces: removes the `COPILOT_GITHUB_TOKEN` export from this section — Task 6 (Configuration) owns it. Tasks 5 and 6 must not both keep that export block.

- [ ] **Step 1: Replace the Keychain access section**

Replace the current `## Keychain access` section with the why/what/where form (and remove the Copilot env-var block, which moves to Configuration in Task 6):

```markdown
## Keychain access

**Why:** macOS sandboxes Keychain items per creating app. AIUsageBar is not
Claude Code, so reading Claude's stored OAuth token requires your explicit
consent. The first read triggers a system dialog — click **Always Allow** once.

![Keychain dialog](assets/keychain-prompt.png)

**What:** only the Claude OAuth access token (Keychain item service
`Claude Code-credentials`, account = your macOS username, JSON value with
`claudeAiOauth.accessToken`). Fallback if the Keychain item is unavailable:
`~/.claude/.credentials.json` (created by Claude Code).

**Where it never goes:** the token is never written, never logged, and never
sent anywhere except `api.anthropic.com` for the documented usage/profile
endpoints.
```

- [ ] **Step 2: Verify**

Run: `grep -q "assets/keychain-prompt.png" README.md && echo OK`
Expected: `OK`

Run: `grep -c "COPILOT_GITHUB_TOKEN" README.md`
Expected: `0` (moves to Configuration in Task 6; this task removes it here)

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: deepen Keychain section (why/what/where) + screenshot"
```

---

### Task 6: Add a Configuration section with the full Copilot token-priority chain

**Files:**
- Modify: `README.md` (insert a `## Configuration` section after `## Providers`)

**Interfaces:**
- Consumes: removal of the `COPILOT_GITHUB_TOKEN` block from Task 5.

- [ ] **Step 1: Insert the Configuration section**

Insert after the `## Providers` section (and its `---`):

```markdown
## Configuration

Copilot needs a GitHub token. AIUsageBar looks for one in this priority order:

1. `COPILOT_GITHUB_TOKEN` (recommended — a fine-grained PAT)
2. `GH_TOKEN`
3. `GITHUB_TOKEN`
4. Keychain item `copilot-cli`
5. `~/.copilot/config.json`
6. `~/.config/gh/hosts.yml`

To avoid the Copilot Keychain prompt, export a PAT before launching:

```bash
export COPILOT_GITHUB_TOKEN=github_pat_...
```

Claude needs no configuration beyond the `claude` CLI being logged in.

---
```

- [ ] **Step 2: Verify**

Run: `grep -q "^## Configuration$" README.md && grep -c "COPILOT_GITHUB_TOKEN" README.md`
Expected: section present; count `1` (only here now).

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add Configuration section with Copilot token priority"
```

---

### Task 7: Move the Rust toolchain requirement out of Requirements into Development

**Files:**
- Modify: `README.md:73-79` (`## Requirements`) and `README.md:81+` (`## Development`)

**Interfaces:**
- None.

- [ ] **Step 1: Trim the Requirements section**

Replace the `## Requirements` section with end-user-only requirements (drop `Rust 1.75+`):

```markdown
## Requirements

- macOS 11+
- At least one provider configured: the `claude` CLI logged in, or a Copilot PAT / `gh` login
```

- [ ] **Step 2: Add the toolchain prerequisite under Development**

At the top of the `## Development` section, before `### One-time setup (per machine)`, add:

```markdown
> Skip this section if you installed the binary from Releases.

**Prerequisites:** Rust 1.75+ (`rustup update`).
```

- [ ] **Step 3: Verify**

Run: `awk '/^## Requirements$/,/^## Development$/' README.md | grep -c "Rust 1.75"`
Expected: `0` (no longer under Requirements)

Run: `awk '/^## Development$/,0' README.md | grep -c "Rust 1.75"`
Expected: `1` (now under Development)

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: move Rust toolchain requirement into Development"
```

---

### Task 8: Rewrite the Releasing section to match scripts/release.sh

**Files:**
- Modify: `README.md:119-146` (`### Releasing a new version`)

**Interfaces:**
- None.

**Ground truth — what `scripts/release.sh <major|minor|patch>` actually does:**
preflight (git-cliff present, repo root, on `master`, clean tree, in sync with `origin/master`, target tag unused) → quality gate (`cargo clippy -- -D warnings` + `cargo test`) → confirm prompt → bump `Cargo.toml` + `cargo check` (syncs `Cargo.lock`) → regenerate `CHANGELOG.md` via git-cliff → commit `chore(release): vX.Y.Z` + annotated tag → push prompt; on **yes**: push `master` + tag, `cargo build --release`, ad-hoc `codesign`, package `dist/aiusagebar-macos-arm64-vX.Y.Z`, extract notes from CHANGELOG, `gh release create` with the binary attached.

- [ ] **Step 1: Replace the Releasing subsection**

Replace `### Releasing a new version` (through the manual `git push` block) with:

```markdown
### Releasing a new version

**Prerequisites:** `git-cliff` and the GitHub CLI on PATH.

```bash
brew install git-cliff gh   # once
```

From the repo root, on a clean `master` in sync with `origin`:

```bash
./scripts/release.sh patch   # 0.1.0 → 0.1.1
./scripts/release.sh minor   # 0.1.0 → 0.2.0
./scripts/release.sh major   # 0.1.0 → 1.0.0
```

The script:
1. Runs preflight checks (branch `master`, clean tree, synced with origin, tag unused)
2. Runs the quality gate (`cargo clippy -- -D warnings` and `cargo test`)
3. Bumps the version in `Cargo.toml`, syncs `Cargo.lock`, and regenerates `CHANGELOG.md`
4. Commits (`chore(release): vX.Y.Z`) and creates the annotated tag
5. Prompts to push — on confirm: pushes `master` + tag, builds the release binary, ad-hoc-signs it, packages `dist/aiusagebar-macos-arm64-vX.Y.Z`, and creates the GitHub release with the binary attached

If you decline the push prompt, the script prints the exact build / sign / `gh release create` commands to run manually later.
```

- [ ] **Step 2: Verify**

Run: `grep -q "gh release create\|GitHub release" README.md && grep -qi "quality gate\|clippy" README.md && echo OK`
Expected: `OK`

Run: `grep -c "Push manually after reviewing" README.md`
Expected: `0` (stale line removed)

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: rewrite Releasing section to match release.sh"
```

---

## Final verification (run after all tasks)

- [ ] `rg -i "openai|codex" README.md` → nothing
- [ ] `rg -i "\.dmg|\.app bundle" README.md` → nothing (binary distribution only)
- [ ] `test -f LICENSE && echo OK` → `OK`
- [ ] `cargo metadata --format-version 1 | jq '.packages[] | select(.name=="aiusagebar") | .license'` → `"MIT"`
- [ ] `grep -c "img.shields.io" README.md` → `2`
- [ ] All four image references resolve: `for f in assets/demo.png assets/gatekeeper-prompt.png assets/keychain-prompt.png; do test -f "$f" && echo "$f OK"; done`
- [ ] `cargo clippy -- -D warnings && cargo test` → clean
- [ ] Open README in GitHub preview: no table-formatting glitches, all Providers-table links resolve.

## Self-Review notes

- Spec coverage: License (T2), badges (T3), Install/binary (T4), Keychain depth (T5), Configuration chain (T6), Rust→Development (T7), Releasing (T8), screenshots (T1). Already-shipped items (Codex removal, naming, demo.png, Troubleshooting) intentionally not re-done.
- Descoped per spec addendum: DMG install, full `.app` first-run walkthrough, `tray-icon.png`, static Troubleshooting table.
- `COPILOT_GITHUB_TOKEN` ownership: removed in T5, re-added once in T6 — verified by the count checks in both tasks.
</content>
</invoke>
