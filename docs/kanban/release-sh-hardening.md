---
id: 36
status: todo
priority: Normal
tags: [release, tooling]
spec: superpowers/specs/2026-06-14-release-sh-hardening-design.md
plan: superpowers/plans/2026-06-16-release-sh-hardening.md
created: 2026-06-14
updated: 2026-06-16
---
# release.sh hardening

Add safety rails to `scripts/release.sh`: pre-flight checks, annotated tag, ERR-trap rollback of working tree, and a post-tag push prompt. Motivated by v0.2.0 being tagged locally on 2026-06-13 and never pushed because the script's final hint was missed.

## Scope

- Pre-flight, abort early:
  - HEAD branch == `master`
  - Working tree clean (no unstaged, no staged)
  - Local `master` == `origin/master` (after `git fetch`)
  - Tag `v$NEW` does not exist locally **or** on `origin`
- Annotated tag: `git tag -a "v$NEW" -m "Release v$NEW"` (replaces current lightweight tag).
- ERR trap installed after pre-flight, before first mutation:
  - On failure: `git checkout -- Cargo.toml CHANGELOG.md` restores tree to `origin/master` baseline.
  - Prints manual recovery hint if commit was already made (no automatic `git reset --hard`).
- Auto-push prompt after tag: `Push to origin now? [y/N]` → if yes, `git push origin master && git push origin "v$NEW"`. Defaults to no.
- Manual acceptance checklist (no shell test framework): happy path, branch guard, dirty tree, out-of-sync, tag-exists, simulated `git-cliff` failure for rollback, push branch.
- `cliff.toml` template: drop `trim_start_matches(pat="v")` so CHANGELOG headings match tag names (`## [v0.2.0]` instead of `## [0.2.0]`). Backfill the existing `## [0.2.0]` entry in `CHANGELOG.md`.

## Out of scope

- GitHub Release creation (`gh release create`) — Card B.
- GitHub Actions tag-triggered build workflow — Card B.
- DMG build — #11.
- Signed tags (`git tag -s`) — no signing key configured.
- `--force` override for pre-flight failures — manual override is rare; not worth the surface.
- Multi-branch releases — project ships from `master`.
- Tag body = changelog section — user chose minimal `"Release v$NEW"`.

## Narrative
- 2026-06-14: Captured from brainstorming. Trigger: v0.2.0 tagged locally on 2026-06-13 (lightweight tag, never pushed). Spec: `docs/superpowers/specs/2026-06-14-release-sh-hardening-design.md`. Chosen approach: hardening in place (no rewrite). Decided to split original brainstorm scope: this card = script hardening; #11 = DMG build (had GH Action bullet, removed); Card B (pending brainstorm) = GH Action on tag → build → upload to GitHub Release. Decisions: auto-push as interactive prompt (not flag), annotated tag with one-line body (`"Release v$NEW"`), pre-flight includes tag-exists check (local + remote), ERR trap for working-tree-only rollback. Rejected: `git tag -s` (no key), `gh release create` (Card B), `--force` override (low value), changelog section in tag body (Card B's GitHub Release will pull from `CHANGELOG.md` instead). v0.2.0 should be pushed manually before this card's PR merges: `git push origin v0.2.0`.
- 2026-06-14: Added `cliff.toml` template fix + `CHANGELOG.md` backfill to this card. Reason: tag names use the `v` prefix (`v0.2.0`), but `cliff.toml:5` currently strips it (`trim_start_matches(pat="v")`), producing `## [0.2.0]` headings. Card #37 (GH Action) extracts release-body sections by tag name, so the heading must match. Fix is one-line cliff template edit + one-line manual backfill of the existing entry. Folded here (vs. a separate card) because it belongs to the release-pipeline coherence story.
