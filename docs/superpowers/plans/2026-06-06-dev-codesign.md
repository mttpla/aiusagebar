# Dev Code Signing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Eliminate repeated Keychain permission prompts during development by signing debug builds with a stable self-signed certificate.

**Architecture:** `cargo run` produces unsigned binaries whose identity is cdhash-based (changes every build). Signing with a self-signed cert produces a stable Designated Requirement (DR) based on the certificate — macOS can then permanently trust the app after one "Always Allow" click. A `Makefile` wraps `cargo build + codesign + run` so the workflow stays ergonomic.

**Tech Stack:** macOS Keychain Access (GUI, one-time), `codesign` CLI, `make`

---

## Why not ad-hoc signing (`--sign -`)?

Ad-hoc signing (`codesign --sign -`) creates a DR of the form `cdhash H"<hash>"`. The hash changes every build → "Always Allow" breaks on every recompile. A real certificate (even self-signed) creates `identifier "aiusagebar" and certificate leaf[subject.CN] = "..."` — stable across all builds signed with the same cert.

---

## One-Time Setup (manual, done before running any task)

Create a self-signed code-signing certificate in Keychain Access. This is a one-time step per machine.

1. Open **Keychain Access.app** (Spotlight: `Keychain Access`)
2. Menu bar: **Keychain Access → Certificate Assistant → Create a Certificate...**
3. Fill in:
   - **Name:** `AiUsageBar Dev`  ← exact string, case-sensitive
   - **Identity Type:** Self Signed Root
   - **Certificate Type:** Code Signing
   - Leave "Let me override defaults" unchecked
4. Click **Create** → **Done**
5. Verify: in Keychain Access, under "login" keychain, find "AiUsageBar Dev" with kind "certificate"

> The cert lives in your login keychain only. It is never committed to the repo.

---

## Files

| Action | Path | Responsibility |
|---|---|---|
| Create | `Makefile` | `dev` target: build + sign + run |
| Modify | `CLAUDE.md` | Update commands section, add one-time setup note |

---

### Task 1: Create Makefile with `dev` target

**Files:**
- Create: `Makefile`

- [x] **Step 1: Create `Makefile`**

```makefile
CERT_NAME ?= AiUsageBar Dev
BINARY     = target/debug/aiusagebar

.PHONY: dev

dev:
	cargo build && codesign --force --sign "$(CERT_NAME)" $(BINARY) && $(BINARY)
```

> Indentation in Makefile rules MUST be a tab character, not spaces. Editors often auto-convert — verify with `cat -A Makefile` (tab shows as `^I`).

- [x] **Step 2: Verify Makefile syntax**

```bash
make --dry-run dev
```

Expected output (no errors):
```
cargo build && codesign --force --sign "AiUsageBar Dev" target/debug/aiusagebar && target/debug/aiusagebar
```

- [x] **Step 3: Run `make dev` once**

```bash
make dev
```

macOS will show a Keychain permission dialog: **"codesign wants to access key 'AiUsageBar Dev' in your keychain"** — enter your macOS password and click **Allow**.

Then the app will launch. macOS shows: **"aiusagebar wants to access the 'Claude Code-credentials' item in the login keychain"** — click **Always Allow**.

- [x] **Step 4: Verify prompt does not reappear**

Stop the app (Ctrl+C), run again:

```bash
make dev
```

Expected: app launches with **no Keychain dialog**.

- [x] **Step 5: Verify signing**

```bash
codesign -d -r - target/debug/aiusagebar
```

Expected output contains:
```
designated => identifier "aiusagebar" and certificate leaf[subject.CN] = "AiUsageBar Dev"
```

This confirms the DR is cert-based (stable), not cdhash-based.

- [x] **Step 6: Commit**

```bash
git add Makefile
git commit -m "build: add Makefile dev target with ad-hoc code signing"
```

---

### Task 2: Update CLAUDE.md commands section

**Files:**
- Modify: `CLAUDE.md` (lines 7–12, the Commands block)

- [x] **Step 1: Replace the commands block**

Current content to replace:
```
```bash
cargo run                 # run in dev (icon loads from icons/app_icon.png relative to CWD)
cargo build --release     # release binary
cargo check               # fast type-check without linking
cargo clippy              # lint
```
```

New content:
```
### One-time dev setup (per machine)

Create a self-signed code-signing certificate to avoid repeated Keychain prompts:

1. Open Keychain Access → Certificate Assistant → Create a Certificate
2. Name: `AiUsageBar Dev`, Identity Type: Self Signed Root, Certificate Type: Code Signing
3. Click Create → Done

This is required once per machine. Not committed to the repo.

### Commands

```bash
make dev                  # build + sign + run in dev (no repeated Keychain prompts after first run)
cargo build --release     # release binary
cargo check               # fast type-check without linking
cargo clippy              # lint
```
```

- [x] **Step 2: Verify CLAUDE.md renders correctly**

```bash
cat CLAUDE.md | head -30
```

Confirm the one-time setup section appears before Commands, and `make dev` is the primary run command.

- [x] **Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: document dev codesign setup, replace cargo run with make dev"
```

---

## Self-Review

**Spec coverage:**
- [x] No Keychain prompt after first "Always Allow" → covered by stable DR via cert
- [x] Dev workflow documented → CLAUDE.md updated
- [x] One-time setup documented → Task 1 Step 3 + CLAUDE.md section
- [x] Makefile created → Task 1

**Placeholder scan:** None found.

**Type consistency:** N/A (no code types).

**Edge cases documented:**
- Makefile tab vs space requirement → noted in Task 1 Step 1
- codesign accessing the cert itself requires password once → noted in Task 1 Step 3
- DR output format to verify → shown in Task 1 Step 5
