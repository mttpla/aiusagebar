# Binary Release Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `scripts/release.sh` so a single command runs quality checks, builds and ad-hoc signs the arm64 binary, publishes it to GitHub Releases with changelog notes, and adds an Installation section to README.

**Architecture:** All steps run locally in `scripts/release.sh`. Quality gate (clippy + tests) runs before any state mutation. Build + sign + GitHub upload run after tag is pushed. No CI secrets needed.

**Tech Stack:** Bash, `cargo clippy`, `cargo test`, `cargo build --release`, `codesign` (macOS built-in), `gh` CLI, `awk` for changelog extraction.

---

## File Map

| File | Change |
|---|---|
| `scripts/release.sh` | Add quality gate, build, sign, copy, gh release create |
| `.gitignore` | Add `dist/` |
| `README.md` | Add `## Installation` section before `## Providers` |

---

### Task 1: Add clippy + test quality gate

**Files:**
- Modify: `scripts/release.sh`

The gate must run **before** any state mutation (version bump, commit, tag). Insert after the "tag must not exist" block and before the confirm prompt. `set -euo pipefail` already aborts on failure — no extra handling needed.

- [ ] **Step 1: Locate insertion point**

Open `scripts/release.sh`. Find these two consecutive blocks (around line 56–67):

```bash
if git ls-remote --tags --exit-code origin "v$NEW" >/dev/null 2>&1; then
    echo "Error: tag v$NEW already exists on origin." >&2
    exit 1
fi

echo "Bumping $CURRENT → $NEW"
read -r -p "Continue? [y/N] " CONFIRM
```

- [ ] **Step 2: Insert quality gate between the two blocks**

```bash
if git ls-remote --tags --exit-code origin "v$NEW" >/dev/null 2>&1; then
    echo "Error: tag v$NEW already exists on origin." >&2
    exit 1
fi

echo "Running quality gate (clippy + tests)..."
cargo clippy -- -D warnings
cargo test
echo "Quality gate passed."

echo "Bumping $CURRENT → $NEW"
read -r -p "Continue? [y/N] " CONFIRM
```

- [ ] **Step 3: Verify manually**

Run with a bad test to confirm early exit:

```bash
# temporarily break a test, then:
bash scripts/release.sh patch
# Expected: "FAILED" from cargo test, script exits before confirm prompt
# No files modified
```

Revert the broken test afterward.

- [ ] **Step 4: Commit**

```bash
git add scripts/release.sh
git commit -m "feat(release): add clippy + test quality gate before version bump"
```

---

### Task 2: Build, sign, package, and publish to GitHub Releases

**Files:**
- Modify: `scripts/release.sh`

After the tag is pushed, reset the ERR trap (the rollback handler is meaningless post-push), then build → sign → copy with version suffix → extract changelog section → `gh release create`.

If the user skips the push, print manual recovery commands and exit cleanly.

- [ ] **Step 1: Locate the push section**

Find this block at the end of `scripts/release.sh`:

```bash
echo ""
read -r -p "Push to origin now? [y/N] " PUSH
if [[ "$PUSH" == "y" || "$PUSH" == "Y" ]]; then
    git push origin master
    git push origin "v$NEW"
    echo "Pushed v$NEW to origin."
else
    echo "Skipped push. To publish later: git push origin master && git push origin v$NEW"
fi
```

- [ ] **Step 2: Replace the entire push section with this**

```bash
echo ""
read -r -p "Push to origin now? [y/N] " PUSH
if [[ "$PUSH" == "y" || "$PUSH" == "Y" ]]; then
    git push origin master
    git push origin "v$NEW"
    echo "Pushed v$NEW to origin."
else
    echo "Skipped push. To publish later:"
    echo "  git push origin master && git push origin v$NEW"
    echo "  cargo build --release"
    echo "  codesign --force -s - target/release/aiusagebar"
    echo "  mkdir -p dist && cp target/release/aiusagebar dist/aiusagebar-macos-arm64-v$NEW"
    echo "  gh release create v$NEW --title v$NEW --notes-file <notes-file> dist/aiusagebar-macos-arm64-v$NEW"
    exit 0
fi

# Post-push: replace rollback trap with a simpler message
trap 'echo "" >&2; echo "Post-push step failed. Tag v$NEW is live on origin." >&2; echo "Run the build/sign/release steps manually (see output above)." >&2' ERR

echo "Building release binary..."
cargo build --release

echo "Signing binary (ad-hoc)..."
codesign --force -s - target/release/aiusagebar
codesign --verify --verbose target/release/aiusagebar

echo "Packaging..."
mkdir -p dist
cp target/release/aiusagebar "dist/aiusagebar-macos-arm64-v${NEW}"

echo "Extracting release notes from CHANGELOG.md..."
NOTES=$(mktemp)
awk -v tag="## [v${NEW}]" 'index($0,tag)==1{p=1;next} /^## \[/{p=0} p' CHANGELOG.md > "$NOTES"

echo "Creating GitHub release..."
gh release create "v${NEW}" \
    "dist/aiusagebar-macos-arm64-v${NEW}" \
    --title "v${NEW}" \
    --notes-file "$NOTES"
rm -f "$NOTES"

echo ""
echo "Released: https://github.com/mttpla/aiusagebar/releases/tag/v${NEW}"
```

- [ ] **Step 3: Verify awk changelog extraction locally**

With the real `CHANGELOG.md`, run:

```bash
NEW="0.2.0"  # use an existing version in your CHANGELOG
awk -v tag="## [v${NEW}]" 'index($0,tag)==1{p=1;next} /^## \[/{p=0} p' CHANGELOG.md
```

Expected: the bullet list for that version, no heading lines, stops before the next `## [`.

- [ ] **Step 4: Verify `gh` is authenticated**

```bash
gh auth status
```

Expected: `✓ Logged in to github.com account mttpla (keyring)` with `Active account: true`.

- [ ] **Step 5: Commit**

```bash
git add scripts/release.sh
git commit -m "feat(release): build, sign, and publish arm64 binary to GitHub Releases"
```

---

### Task 3: .gitignore and README Installation section

**Files:**
- Modify: `.gitignore`
- Modify: `README.md`

- [ ] **Step 1: Add `dist/` to .gitignore**

Append to `.gitignore`:

```
dist/
```

- [ ] **Step 2: Add Installation section to README**

Insert this block in `README.md` **between** the opening paragraph and `## Providers` (after the `---` separator on line 7, before line 9 `## Providers`):

```markdown
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
```

- [ ] **Step 3: Commit**

```bash
git add .gitignore README.md
git commit -m "chore: add dist/ to .gitignore and add Installation section to README"
```

---

### Task 4: Manual smoke test (full dry run)

No automated test for shell release scripts. Verify the entire flow without actually pushing.

- [ ] **Step 1: Verify quality gate fires correctly**

```bash
bash scripts/release.sh patch
```

Expected first output lines:
```
Running quality gate (clippy + tests)...
...
Quality gate passed.
Bumping X.Y.Z → X.Y.(Z+1)
Continue? [y/N]
```

Type `N` to abort before any mutation.

- [ ] **Step 2: Verify build + sign produce a valid binary**

Run the build steps manually to confirm they work on this machine:

```bash
cargo build --release
codesign --force -s - target/release/aiusagebar
codesign --verify --verbose target/release/aiusagebar
```

Expected from codesign verify:
```
target/release/aiusagebar: valid on disk
target/release/aiusagebar: satisfies its Designated Requirement
```

- [ ] **Step 3: Verify awk extraction on the real CHANGELOG**

```bash
awk -v tag="## [v$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')]" \
    'index($0,tag)==1{p=1;next} /^## \[/{p=0} p' CHANGELOG.md
```

Expected: non-empty release notes for the current version.

- [ ] **Step 4: Verify gh release create syntax (dry run)**

```bash
gh release create --help | grep -E "notes-file|title"
```

Confirms flags exist in installed gh version.

- [ ] **Step 5: Update card #11 on the kanban board**

Move card #11 to `done`. Append to Narrative: date, outcome, any deviations from plan.

---

## Self-Review

**Spec coverage:**
- ✅ clippy gate before version bump
- ✅ `cargo test` gate
- ✅ `cargo build --release`
- ✅ `codesign -s -` (ad-hoc)
- ✅ cp with version suffix (`aiusagebar-macos-arm64-vX.Y.Z`)
- ✅ `gh release create` (published immediately, no draft)
- ✅ changelog notes extracted from CHANGELOG.md
- ✅ `dist/` in .gitignore
- ✅ README Installation section with Gatekeeper workaround

**Placeholder scan:** No TBD / TODO / "handle edge cases" patterns present.

**Type consistency:** N/A — shell script, no types. Variable names (`$NEW`, `$NOTES`) are consistent across tasks.
