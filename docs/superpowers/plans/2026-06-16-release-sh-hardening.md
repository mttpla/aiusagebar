# release.sh Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `scripts/release.sh` safe to run without thinking: refuse bad repo state, rollback on failure, create an annotated tag, and prompt to push.

**Architecture:** Pure shell edits to `scripts/release.sh`. Pre-flight block runs first (before any mutation). ERR trap installed immediately before first `sed`. Annotated tag replaces lightweight tag. Interactive push prompt replaces the final hint line. Separate task fixes `cliff.toml` v-prefix and backfills `CHANGELOG.md` headings.

**Tech Stack:** bash, git, git-cliff (already in script)

---

## File Map

| File | Change |
|---|---|
| `cliff.toml` | Remove `trim_start_matches(pat="v")` from version heading template |
| `CHANGELOG.md` | Backfill 4 headings: prefix `0.2.0`, `0.3.0`, `0.3.1`, `0.3.2` with `v` |
| `scripts/release.sh` | Add pre-flight block, ERR trap, annotated tag, push prompt |

---

### Task 1: Fix cliff.toml and backfill CHANGELOG.md

**Files:**
- Modify: `cliff.toml:5`
- Modify: `CHANGELOG.md` (4 heading lines)

Current `cliff.toml` line 5:
```
## [{{ version | trim_start_matches(pat="v") }}] - {{ timestamp | date(format="%Y-%m-%d") }}
```

Future releases (via `release.sh`) pass `--tag "v$NEW"`, so `version` = `v0.4.0`. The `trim_start_matches` strips the prefix, producing `## [0.4.0]`. Card #37 (GH Action) extracts release-body sections by tag name, so heading must match tag exactly.

- [ ] **Step 1: Edit cliff.toml line 5** — remove the trim filter:

```toml
## [{{ version }}] - {{ timestamp | date(format="%Y-%m-%d") }}
```

Result: line 5 of `cliff.toml` becomes exactly that string.

- [ ] **Step 2: Backfill CHANGELOG.md** — rename all 4 bare-version headings:

```
## [0.2.0] → ## [v0.2.0]
## [0.3.0] → ## [v0.3.0]
## [0.3.1] → ## [v0.3.1]
## [0.3.2] → ## [v0.3.2]
```

Open `CHANGELOG.md`, find lines starting with `## [0.` and prefix each version with `v`. The lines currently read:
```
## [0.3.2] - 2026-06-16
## [0.3.1] - 2026-06-16
## [0.3.0] - 2026-06-15
## [0.2.0] - 2026-06-13
```
After edit:
```
## [v0.3.2] - 2026-06-16
## [v0.3.1] - 2026-06-16
## [v0.3.0] - 2026-06-15
## [v0.2.0] - 2026-06-13
```

- [ ] **Step 3: Verify no bare `## [0.` headings remain**

```bash
grep "^## \[0\." CHANGELOG.md
```

Expected: no output.

- [ ] **Step 4: Commit**

```bash
git add cliff.toml CHANGELOG.md
git commit -m "fix(release): use v-prefix in cliff.toml heading, backfill CHANGELOG"
```

---

### Task 2: Add pre-flight checks

**Files:**
- Modify: `scripts/release.sh`

Pre-flight runs after arg-parse and tool checks, before version parsing and any mutation. Checks 1–3 go right after the repo-root block (line 17). Check 4 (tag-not-exists) goes after `$NEW` is computed (after the `case $BUMP` block, line 29), before the user confirm prompt.

- [ ] **Step 1: Insert pre-flight checks 1–3 after line 17** (after the repo-root check, before `# Parse current version`):

```bash
# Pre-flight: branch
BRANCH="$(git rev-parse --abbrev-ref HEAD)"
[[ "$BRANCH" == "master" ]] || {
    echo "Error: must be on master (current: $BRANCH)" >&2
    exit 1
}

# Pre-flight: working tree clean
git diff --quiet && git diff --cached --quiet || {
    echo "Error: working tree not clean. Commit or stash first." >&2
    exit 1
}

# Pre-flight: sync with origin
git fetch --quiet origin master
LOCAL="$(git rev-parse HEAD)"
REMOTE="$(git rev-parse origin/master)"
[[ "$LOCAL" == "$REMOTE" ]] || {
    echo "Error: local master not in sync with origin/master." >&2
    echo "  local:  $LOCAL" >&2
    echo "  remote: $REMOTE" >&2
    exit 1
}
```

- [ ] **Step 2: Insert pre-flight check 4 after the `case $BUMP` block** (after line 29, before `echo "Bumping"`):

```bash
# Pre-flight: tag must not exist locally or on origin
if git rev-parse "v$NEW" >/dev/null 2>&1; then
    echo "Error: tag v$NEW already exists locally." >&2
    exit 1
fi
if git ls-remote --tags --exit-code origin "v$NEW" >/dev/null 2>&1; then
    echo "Error: tag v$NEW already exists on origin." >&2
    exit 1
fi
```

- [ ] **Step 3: Verify script still parses**

```bash
bash -n scripts/release.sh
```

Expected: no output (syntax OK).

- [ ] **Step 4: Smoke-test branch guard** — from a new branch:

```bash
git checkout -b test-preflight
bash scripts/release.sh patch
```

Expected: `Error: must be on master (current: test-preflight)`. Clean up:

```bash
git checkout master
git branch -d test-preflight
```

- [ ] **Step 5: Commit**

```bash
git add scripts/release.sh
git commit -m "feat(release): add pre-flight checks (branch, clean tree, sync, tag-exists)"
```

---

### Task 3: Add ERR trap for rollback

**Files:**
- Modify: `scripts/release.sh`

The trap must fire on any non-zero exit after the first mutation. Install it immediately before the `sed` line (currently line 39: `sed -i '' "s/^version ...`). At that point pre-flight has already confirmed the tree is clean, so `git checkout -- Cargo.toml CHANGELOG.md` reliably restores origin/master content.

- [ ] **Step 1: Insert trap before the `sed` line**:

```bash
# Rollback working tree if any step fails after this point
cleanup_on_error() {
    local rc=$?
    echo "" >&2
    echo "Error: release failed (exit $rc). Rolling back working tree..." >&2
    git checkout -- Cargo.toml CHANGELOG.md 2>/dev/null || true
    echo "  (if a commit was already made: git reset --hard origin/master)" >&2
    exit "$rc"
}
trap cleanup_on_error ERR
```

- [ ] **Step 2: Verify script still parses**

```bash
bash -n scripts/release.sh
```

Expected: no output.

- [ ] **Step 3: Commit**

```bash
git add scripts/release.sh
git commit -m "feat(release): add ERR trap — rollback Cargo.toml + CHANGELOG on failure"
```

---

### Task 4: Annotated tag + push prompt

**Files:**
- Modify: `scripts/release.sh`

Replace line `git tag "v$NEW"` with the annotated form. Replace the final `echo "Done. To publish: ..."` block with an interactive prompt.

- [ ] **Step 1: Change the tag line** from:

```bash
git tag "v$NEW"
```

to:

```bash
git tag -a "v$NEW" -m "Release v$NEW"
```

- [ ] **Step 2: Replace the final hint block** — remove:

```bash
echo ""
echo "Done. To publish:"
echo "  git push && git push --tags"
```

and replace with:

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

- [ ] **Step 3: Verify script still parses**

```bash
bash -n scripts/release.sh
```

Expected: no output.

- [ ] **Step 4: Commit**

```bash
git add scripts/release.sh
git commit -m "feat(release): annotated tag, interactive push prompt"
```

---

### Task 5: Manual acceptance testing

No automated test framework. Run the 7-scenario checklist from the spec against the final script. Work on a throwaway branch or use `--dry-run` where noted.

- [ ] **Scenario 1 — Happy path (decline push)**

Prerequisites: master synced with origin, no pending tags, clean tree.

```bash
./scripts/release.sh patch
```

At "Continue? [y/N]" enter `y`. At "Push to origin now? [y/N]" enter `n`.

Verify:
```bash
grep '^version' Cargo.toml              # shows new version
grep "^## \[v" CHANGELOG.md | head -1  # shows new v-prefixed heading
git log --oneline -1                    # chore(release): vX.Y.Z
git tag -n "v$(grep '^version' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')"
# shows annotated tag with "Release vX.Y.Z"
```

Undo the test bump:
```bash
git reset --hard HEAD~1
git tag -d "v<NEW>"
```

- [ ] **Scenario 2 — Branch guard**

```bash
git checkout -b test-branch
./scripts/release.sh patch
```

Expected output contains: `Error: must be on master`

```bash
git checkout master && git branch -d test-branch
```

- [ ] **Scenario 3 — Dirty tree guard**

```bash
echo " " >> Cargo.toml
./scripts/release.sh patch
```

Expected output contains: `Error: working tree not clean`

```bash
git checkout -- Cargo.toml
```

- [ ] **Scenario 4 — Out-of-sync guard**

```bash
git reset --hard HEAD~1
./scripts/release.sh patch
```

Expected output contains: `Error: local master not in sync with origin/master`

```bash
git reset --hard origin/master
```

- [ ] **Scenario 5 — Tag exists locally**

```bash
git tag v9.9.9
./scripts/release.sh patch   # adjust so NEW == 9.9.9 if needed, else pick a matching version
```

Or more directly: temporarily set version in Cargo.toml to `9.9.8`, create tag `v9.9.9`, run `patch`.

Expected output contains: `Error: tag v9.9.9 already exists locally`

```bash
git tag -d v9.9.9
git checkout -- Cargo.toml   # if you edited it
```

- [ ] **Scenario 6 — ERR trap rollback**

Temporarily break git-cliff to trigger a mid-flow failure. After pre-flight passes and `sed` has already bumped `Cargo.toml`, simulate `git-cliff` failure:

```bash
# Rename git-cliff binary temporarily
CLIFF_PATH="$(which git-cliff)"
sudo mv "$CLIFF_PATH" "${CLIFF_PATH}.bak"
./scripts/release.sh patch
```

Expected: trap fires, prints "Rolling back working tree...", `Cargo.toml` shows original version.

```bash
sudo mv "${CLIFF_PATH}.bak" "$CLIFF_PATH"
```

- [ ] **Scenario 7 — Push prompt (y)**

Only run this on a real branch you control and can clean up. Accept push at prompt. Verify:

```bash
git log origin/master --oneline -1    # shows release commit
git ls-remote --tags origin | grep "v<NEW>"  # shows annotated tag on remote
```

Clean up remote if this was a test:
```bash
git push origin --delete "v<NEW>"
git push origin master --force  # only if you want to undo
```

> Note: Scenario 7 pushes to real `origin`. Only run on a branch/remote you control for testing, or skip and accept that this path was exercised by the real next release.

- [ ] **All 7 scenarios passed → card #36 done**

```bash
# Update kanban card status in docs/kanban/release-sh-hardening.md: status: done
# Append Narrative entry with date and summary
```
