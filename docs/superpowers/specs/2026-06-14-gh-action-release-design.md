# GitHub Action: tag-triggered DMG release

## Problem

Today the release flow is half-manual: `scripts/release.sh` (card #36) creates and pushes an annotated tag, but everything after that — building the `.dmg`, drafting a GitHub Release, and uploading the asset — requires the user to run commands locally and then click through the GitHub UI. The DMG build script itself lives in card #11. There is no automated, reproducible build pipeline that fires on a tag push.

The original `#11` scope included a "GitHub Actions release workflow" bullet. That was split out to this card so #11 stays focused on the build script + README workaround.

## Goal

When a `v*` tag is pushed to `origin/master`, a GitHub Action builds the arm64 DMG, creates a **draft** GitHub Release for that tag with the section of `CHANGELOG.md` matching the tag as the body, and attaches the DMG as the release asset. The user reviews and clicks Publish.

A secondary goal: detect regressions in the workflow or the build script before they reach a tag, by re-running the build on pull requests that touch them.

## Non-goals

- Universal binary (arm64 + x86_64) or Intel Mac support. arm64 only; Intel Macs are out of the supported population for this project.
- Notarization / Developer ID signing. Tracked in #35 (post-1.0). This workflow consumes whatever `release-dmg.sh` produces — ad-hoc signed for now.
- Homebrew cask. Out of scope for both #11 and this card.
- Publishing the release automatically. Draft requires a manual click — intentional safety net.
- Multi-platform releases (Linux, Windows). Project is macOS-only.
- Backfilling past releases (v0.2.0 etc.). User pushes those tags manually if desired.
- `workflow_dispatch` manual trigger. Can be added later if needed; not part of MVP.

## Dependencies

- **Hard dependency**: `scripts/release-dmg.sh` must exist and produce `AiUsageBar.dmg` at the repo root. That script is the deliverable of card #11. This card is `blocked_by: [11]`.
- No dependency on card #36 (release.sh hardening). The Action only requires that a `v*` tag arrives on `origin` — it does not care how `release.sh` produced it. Annotated vs lightweight tag is irrelevant to this workflow.

## Design

### File

`.github/workflows/release.yml` — single workflow, two trigger paths, one job.

```yaml
name: Release

on:
  push:
    tags: ['v*']
  pull_request:
    paths:
      - .github/workflows/release.yml
      - scripts/release-dmg.sh

permissions:
  contents: write

jobs:
  build-dmg:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Build DMG
        run: ./scripts/release-dmg.sh

      - name: Upload DMG artifact (PR only)
        if: github.event_name == 'pull_request'
        uses: actions/upload-artifact@v4
        with:
          name: AiUsageBar-dmg
          path: AiUsageBar.dmg
          if-no-files-found: error

      - name: Extract changelog section
        if: startsWith(github.ref, 'refs/tags/')
        id: changelog
        run: |
          TAG="${GITHUB_REF#refs/tags/}"
          # extract block between "## [vX.Y.Z]" and the next "## [" heading
          awk "/^## \\[$TAG\\]/{flag=1;next}/^## \\[/{flag=0}flag" CHANGELOG.md > /tmp/notes.md
          echo "notes_file=/tmp/notes.md" >> "$GITHUB_OUTPUT"

      - name: Create draft release with asset
        if: startsWith(github.ref, 'refs/tags/')
        uses: softprops/action-gh-release@v2
        with:
          draft: true
          files: AiUsageBar.dmg
          body_path: ${{ steps.changelog.outputs.notes_file }}
          fail_on_unmatched_files: true
```

### Behavior matrix

| Trigger | Build | Artifact | Changelog | Release |
|---|---|---|---|---|
| `push` of `refs/tags/v*` | yes | no | extracted | created as draft, DMG attached |
| `pull_request` touching workflow or script | yes | uploaded for inspection | skipped | skipped |
| `pull_request` elsewhere | not triggered | — | — | — |

### Runner and architecture

`macos-14` is GitHub's arm64 runner image, free on public repos. The output DMG is arm64-native. Anyone on an Intel Mac will not be able to install (`#11` README workaround does not cover arch mismatch — a follow-up README note is in scope for #11 if not already covered).

### Caching

`Swatinem/rust-cache@v2` caches `~/.cargo` registry and the `target/` directory across runs. Significantly cuts cold-build time (Rust deps + first cargo build ≈ minutes). Cache key derives from `Cargo.lock` automatically.

### Permissions and secrets

- `permissions: contents: write` is required for `softprops/action-gh-release` to create the release and upload assets via `GITHUB_TOKEN`.
- No additional secrets are needed at MVP. When #35 lands, secrets for Developer ID + notarytool API key will be added then, in #35's PR, not here.

### Changelog extraction

`CHANGELOG.md` is produced by `git-cliff` (card #2) and uses the `## [v0.2.0] - 2026-06-13` heading format. The awk extractor reads from the matching heading up to (but not including) the next `## [`. If the tag has no matching section (e.g., the user manually pushed a tag without running `release.sh`), `/tmp/notes.md` is empty and the draft release is created with an empty body — non-blocking; the user fills it in via the GitHub UI before publishing.

### Re-run and idempotency

`softprops/action-gh-release` upserts the release for a tag. Re-running the workflow on the same tag updates the draft (replaces the asset, refreshes the body). Safe to re-run after fixing a transient build failure without deleting the tag.

### Failure modes

| Failure | Outcome |
|---|---|
| `scripts/release-dmg.sh` missing (e.g., #11 not yet shipped) | Job fails on the `Build DMG` step with `command not found`. Loud, expected pre-#11. |
| `release-dmg.sh` exits non-zero (e.g., codesign error) | Job fails on `Build DMG`. No release created. Tag stays. Re-run after fix. |
| `AiUsageBar.dmg` not produced | `fail_on_unmatched_files: true` makes the upload step fail loudly. |
| Empty changelog section | Empty draft body. Non-blocking. User edits in UI. |
| Tag already has a published (non-draft) release | `softprops/action-gh-release@v2` updates the existing release in place. User notices via the diff. Not a typical case because the workflow always creates as draft. |

## Tests

No unit tests possible (the workflow is YAML executed by GitHub). Manual acceptance:

1. **PR test path**: open a draft PR that edits a comment in `scripts/release-dmg.sh`. Confirm the `build-dmg` job runs, no changelog/release steps execute, and the DMG appears in the PR's Checks → Artifacts.
2. **PR no-trigger**: open a PR touching only `src/`. Confirm the workflow does not run.
3. **Tag happy path**: push `v0.3.0-test` (a throwaway tag) after merging both this card and #11. Confirm:
   - Job runs on `macos-14`.
   - Draft release `v0.3.0-test` exists, marked draft.
   - DMG asset attached.
   - Body matches the `## [v0.3.0-test]` section of `CHANGELOG.md`.
4. **Empty changelog**: push a tag with no corresponding `CHANGELOG.md` section. Confirm draft is created with empty body, no error.
5. **Re-run idempotent**: re-run the workflow on the same tag from the Actions tab. Confirm the draft is updated, asset replaced, no second release created.
6. **Cleanup**: delete the test draft + test tag (`git push --delete origin v0.3.0-test`).

Document this checklist in the PR body so the reviewer can replay.

## Rollout

This card is **pre-1.0**: required before the 1.0.0 release because the alternative is manually building and uploading a DMG to GitHub Releases for every ship event.

Merge order before 1.0:

1. **#36** (release.sh hardening + `cliff.toml` v-prefix fix) — must land first so `CHANGELOG.md` headings match tag names at extraction time.
2. **#11** (DMG build script) — must land before this card so `scripts/release-dmg.sh` exists when the workflow runs.
3. **This card (#37)** adds `.github/workflows/release.yml` only. No code changes outside the workflow file.

After merge, the first real tag push (e.g., `v0.3.0` via `release.sh`) exercises the path end-to-end. Until then, the PR path is the only working trigger and the test plan must be exercised on the PR itself.

No migration. v0.2.0 (already-pushed-locally tag) is not retroactively built; the user can either push it (`git push origin v0.2.0`) which would trigger this workflow once it exists, or skip it entirely.

Notarization (#35) is **not** required for 1.0; the DMG ships ad-hoc signed with the README `xattr` workaround per #11.

## Future work (explicitly deferred)

- `workflow_dispatch` manual trigger with version input — useful for rebuilds.
- Universal binary via `lipo` — when Intel Mac support becomes a real requirement.
- Notarization secrets + steps — folded into #35's PR.
- Auto-publish (drop draft) — gated by sufficient confidence in the pipeline.
- Multi-arch matrix — same trigger.
- Slack/email notification on release publish.
