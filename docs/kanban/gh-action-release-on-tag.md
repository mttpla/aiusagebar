---
id: 37
status: backlog
priority: Normal
blocked_by: [11, 36]
tags: [release, ci, pre-1.0]
spec: superpowers/specs/2026-06-14-gh-action-release-design.md
created: 2026-06-14
updated: 2026-06-14
---
# GitHub Action: tag-triggered DMG release

Add `.github/workflows/release.yml`. On `v*` tag push: build arm64 DMG (via `scripts/release-dmg.sh` from #11), create a draft GitHub Release for the tag, attach the DMG, and use the matching `## [vX.Y.Z]` section of `CHANGELOG.md` as the body. On PRs that touch the workflow or the build script, re-run the build and upload the DMG as an artifact (no release).

## Scope

- New file: `.github/workflows/release.yml`.
- Triggers:
  - `push` of `refs/tags/v*` → full build + draft release.
  - `pull_request` with `paths: [.github/workflows/release.yml, scripts/release-dmg.sh]` → build only, upload DMG as PR artifact, skip changelog + release.
- Runner: `macos-14` (arm64).
- Steps: checkout → Rust toolchain → `Swatinem/rust-cache@v2` → `./scripts/release-dmg.sh` → (tag) extract `## [$TAG]` block from `CHANGELOG.md` → `softprops/action-gh-release@v2` with `draft: true`, DMG attached, `body_path` from extracted notes.
- `permissions: contents: write`. Only `GITHUB_TOKEN` — no Apple Developer secrets at MVP.
- Manual test plan documented in the spec (PR build path, tag happy path, empty changelog, re-run idempotency, cleanup).

## Out of scope

- Universal binary (arm64 + x86_64). arm64 only.
- Intel Mac runner.
- Notarization / Developer ID signing — #35.
- Homebrew cask.
- Auto-publish (drops draft). Draft requires manual click as a safety net.
- `workflow_dispatch` manual trigger — future enhancement.
- Backfilling pre-existing tags (v0.2.0).
- Slack / email notifications on release publish.

## Narrative
- 2026-06-14: Captured from brainstorming. Spec: `docs/superpowers/specs/2026-06-14-gh-action-release-design.md`. Split out from #11 (the GH Action bullet there was removed in commit on 2026-06-14). Key decisions: trigger on tag push (vs. release published or manual dispatch — picked tag for zero-friction); arm64-only runner (`macos-14`) — universal/Intel deferred; build invokes `scripts/release-dmg.sh` (vs. inline yaml steps or Makefile — picked script for local re-use and testability); release created as **draft** (vs. published or pre-release — picked draft for safety net); body extracted from `CHANGELOG.md` section (vs. empty body or tag annotation body — picked changelog because `git-cliff` already produces it); `softprops/action-gh-release@v2` for upsert idempotency; PR test job triggers only on changes to workflow or build script (vs. every PR — saves CI minutes). Initial assessment: `blocked_by: [11]` only — annotated vs. lightweight tag was thought irrelevant.
- 2026-06-14: Added `blocked_by: 36`. Reason: while preparing this spec, discovered that `cliff.toml:5` strips the `v` prefix from CHANGELOG headings (`## [0.2.0]`), but tag names carry it (`v0.2.0`). The workflow extracts the release body by matching `## [$TAG]` in `CHANGELOG.md`, so the heading must match the tag. Fix (drop `trim_start_matches` + backfill existing entry) was folded into #36 because it belongs to the release-pipeline coherence story. #37 now depends on #36 shipping first so the heading format matches at extraction time.
- 2026-06-14: Tagged `pre-1.0`. Hard requirement for 1.0.0: without this Action, the 1.0 release would require the user to manually build and upload the DMG to a GitHub Release every time — too much friction for the actual ship event. Ship order before 1.0: #36 (release pipeline coherence) → #11 (DMG build script) → #37 (this card). #35 (notarization) stays post-1.0.
