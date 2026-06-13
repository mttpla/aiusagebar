# release.sh hardening

## Problem

`scripts/release.sh` (40 lines, shipped in card #2) creates a local tag but never pushes it. v0.2.0 was tagged locally on 2026-06-13 and never appeared on GitHub because the user forgot the manual `git push --tags` printed in the script's final hint. The script also runs from any branch, with a dirty working tree, against a stale local `master` — none of which are caught until something downstream breaks.

The script's tag is also a lightweight tag (`git tag "v$NEW"`), which gives GitHub Releases no metadata (no author, no date, no message). Future GitHub Actions on tag push (Card B / #11) will benefit from an annotated tag with a stable message.

## Goal

Make `release.sh` safe to run repeatedly without thinking: refuse to proceed unless the repo is in a known-good state, restore the working tree if any mid-flow command fails, create an annotated tag, and offer to push immediately.

## Non-goals

- GitHub Release creation (`gh release create`). Tracked in Card B alongside the tag-triggered GitHub Action.
- DMG build. Tracked in #11.
- Signed tags (`git tag -s`). User has no signing key configured; defer until needed.
- `--force` override flag for pre-flight failures. Manual override (commit/stash/checkout) is rare enough to not justify the surface.
- Multi-branch releases (e.g., release from a `release/x.y` branch). Project ships from `master`.
- Tag message body = changelog section. User chose minimal `"Release v$NEW"` body.

## Design

### Pre-flight block

Runs after arg-parse and `git-cliff` availability check, before reading `Cargo.toml`. Any failure aborts before the script touches anything.

```bash
# 1. Branch
BRANCH="$(git rev-parse --abbrev-ref HEAD)"
[[ "$BRANCH" == "master" ]] || {
    echo "Error: must be on master (current: $BRANCH)" >&2
    exit 1
}

# 2. Working tree clean (no unstaged, no staged)
git diff --quiet && git diff --cached --quiet || {
    echo "Error: working tree not clean. Commit or stash first." >&2
    exit 1
}

# 3. Sync with origin
git fetch --quiet origin master
LOCAL="$(git rev-parse HEAD)"
REMOTE="$(git rev-parse origin/master)"
[[ "$LOCAL" == "$REMOTE" ]] || {
    echo "Error: local master not in sync with origin/master." >&2
    echo "  local:  $LOCAL" >&2
    echo "  remote: $REMOTE" >&2
    exit 1
}

# 4. Tag does not already exist (local OR remote)
if git rev-parse "v$NEW" >/dev/null 2>&1; then
    echo "Error: tag v$NEW already exists locally." >&2
    exit 1
fi
if git ls-remote --tags --exit-code origin "v$NEW" >/dev/null 2>&1; then
    echo "Error: tag v$NEW already exists on origin." >&2
    exit 1
fi
```

Check 4 runs after `$NEW` is computed (so it sits later in the script, after the `case $BUMP` block). Checks 1–3 run before bump computation.

### Rollback trap

Installed immediately before the first mutating command (`sed -i`), after pre-flight has confirmed the baseline is clean.

```bash
cleanup_on_error() {
    local rc=$?
    echo "" >&2
    echo "Error: release failed (exit $rc). Rolling back working tree..." >&2
    git checkout -- Cargo.toml CHANGELOG.md 2>/dev/null || true
    echo "  (no commit was created; if a commit was made, run: git reset --hard origin/master)" >&2
    exit "$rc"
}
trap cleanup_on_error ERR
```

`ERR` only fires on non-zero exit, so a successful run never invokes the cleanup. The pre-flight `clean tree + sync` guarantee means `git checkout -- <file>` restores the exact `origin/master` content.

The commit + tag steps happen after the trap is armed. If `git commit` succeeds but `git tag` fails (unlikely since the tag-exists pre-flight ran), the user sees the manual recovery hint. We do not attempt automatic `git reset --hard` — destructive operations stay opt-in.

### Annotated tag

Replace lightweight tag with annotated, fixed message:

```bash
git tag -a "v$NEW" -m "Release v$NEW"
```

No signing flag. Body intentionally one line: GitHub Releases (Card B) will pull the rich body from `CHANGELOG.md`, not from the tag.

### Auto-push prompt

Replaces the final "to publish: ..." hint:

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

Pushes the tag by name (not `--tags`) to avoid surprise-uploading any unrelated local tags. The prompt defaults to no for safety; the user must opt in each time.

### Final script flow

```
arg parse → git-cliff check → repo-root check
  → pre-flight 1–3 (branch / clean / sync)
  → parse current version → compute $NEW
  → pre-flight 4 (tag-not-exists local + remote)
  → user confirm "Bumping $CURRENT → $NEW"
  → trap cleanup_on_error ERR
  → sed Cargo.toml
  → git-cliff CHANGELOG.md
  → git add + git commit
  → git tag -a -m
  → push prompt
  → exit 0 (trap clears)
```

## Tests

Manual only. No test framework for shell in this repo, and the failure surface is small.

Manual acceptance checklist:

1. Happy path: clean master synced with origin, no `v$NEW` tag. Run `./scripts/release.sh patch`. Confirm bump, decline push. Verify Cargo.toml + CHANGELOG.md changed, commit created, annotated tag visible in `git tag -n`, no remote push.
2. Branch guard: checkout a branch, run script → exits with "must be on master".
3. Dirty tree: touch `Cargo.toml`, run script → exits with "working tree not clean".
4. Out-of-sync: `git reset --hard HEAD~1`, run script → exits with "not in sync with origin/master".
5. Tag exists (local): create `v9.9.9` locally, run script with arg producing same target → exits with "tag exists locally".
6. Rollback: simulate `git-cliff` failure (`mv $(which git-cliff) /tmp/`), run script after pre-flight passes → trap fires, Cargo.toml restored to pre-bump content.
7. Push branch: happy path with `y` at prompt → both `master` and `v$NEW` appear on `origin`.

Document this checklist as the test plan in the kanban card; do not commit a manual-test script.

## CHANGELOG.md tag prefix

`cliff.toml:5` currently strips the `v` prefix from version headings:

```
## [{{ version | trim_start_matches(pat="v") }}] - ...
```

so the current `CHANGELOG.md` reads `## [0.2.0] - 2026-06-13`. We want headings to match tag names exactly (`## [v0.2.0]`), both because tags carry the `v` prefix and because the GitHub Action workflow (#37) extracts release-body sections by tag name.

Changes folded into this card:

1. Edit `cliff.toml` to remove the trim filter:
   ```
   ## [{{ version }}] - {{ timestamp | date(format="%Y-%m-%d") }}
   ```
2. Backfill the existing `CHANGELOG.md` heading: `## [0.2.0]` → `## [v0.2.0]`. One-line manual edit; no need to re-run `git-cliff`. Same edit applies to any `## [Unreleased]` placeholder if present at PR time (today it is not).

`release.sh` already calls `git-cliff --tag "v$NEW"`, so the `version` template variable will render as `v0.3.0`, `v0.4.0`, etc. — no script change needed.

## Dependencies

No new tools. Already required: `bash`, `git`, `git-cliff` (Homebrew, already documented in script).

## Rollout

Single PR. No migration. Backward compatible: same CLI (`./scripts/release.sh major|minor|patch`), stricter behavior. Documented in PR body that v0.2.0 (already tagged with old script, lightweight, unpushed) should be pushed manually before merging this PR:

```
git push origin v0.2.0
```

After merge, future releases benefit from the hardened script.
