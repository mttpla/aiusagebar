---
id: 11
status: doing
priority: High
tags: [release, pre-1.0]
plan: superpowers/plans/2026-06-16-binary-release-pipeline.md
created: 2026-06-13
updated: 2026-06-16
---
# Binary release: ad-hoc signed arm64 binary

Extend `scripts/release.sh` so that, as part of the release flow, it runs quality checks, builds and signs the binary, then publishes a draft GitHub Release with the binary attached. No DMG, no .app bundle — raw binary with version suffix. DMG deferred to #42.

## Scope

Extend `scripts/release.sh` with these steps (in order, integrated with existing flow):

1. **Pre-quality gate** (before version bump): `cargo clippy -- -D warnings` + `cargo test`. Abort on any failure — nothing is modified if checks fail.
2. **Build** (after version bump, changelog, commit, tag, push): `cargo build --release`.
3. **Sign**: `codesign -s - target/release/aiusagebar` (ad-hoc — prevents "damaged" Gatekeeper error).
4. **Copy with version suffix**: `mkdir -p dist && cp target/release/aiusagebar dist/aiusagebar-macos-arm64-v$NEW`.
5. **Publish**: `gh release create "v$NEW" --title "v$NEW" --notes-file <(changelog section for v$NEW) dist/aiusagebar-macos-arm64-v$NEW`.
   - `gh` CLI must be authenticated (`gh auth login`).

README "Installation" section:
- Download `aiusagebar-macos-arm64-vX.Y.Z` from GitHub Releases
- `chmod +x aiusagebar-macos-arm64-vX.Y.Z && mv aiusagebar-macos-arm64-vX.Y.Z /usr/local/bin/aiusagebar` (or any path)
- First launch from Finder: right-click → Open, **or** `xattr -dr com.apple.quarantine /path/to/aiusagebar`

## Out of scope

- DMG / .app bundle — deferred to #42.
- Developer ID signing, notarization — #35 (post-1.0).
- GitHub Actions CI release (upload from GH runner) — #37 scope reduced to PR build verification only.
- Homebrew cask — post-1.0.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Without notarization, non-dev users cannot run the app. Considered ad-hoc signing (free) — rejected: Gatekeeper still blocks downloaded apps. Self-signed cert in CLAUDE.md is dev-only. Hard requirement for 1.0.0.
- 2026-06-13: Re-scoped. User not enrolled in Apple Developer Program ($99/yr). Pragmatic path: ship ad-hoc signed DMG + document `xattr` / "Open Anyway" workaround. Friction acceptable for 1.0 alpha audience. Notarization deferred to #35. README revamp (#34) still blocked_by [10, 11] — works with new scope since DMG is downloadable, just unsigned.
- 2026-06-14: Removed GH Actions release workflow from scope. Splitting into separate card (Card B) — brainstorm pending. Rationale: cleaner boundaries, independent rollout, and the Action can land after this card without coupling. This card now covers only the DMG build script + README workaround.
- 2026-06-16: Re-scoped from DMG to raw binary. DMG moved to #42 (future). Scope is now: extend release.sh with clippy + tests gate (before version bump) + build + ad-hoc codesign + cp with version suffix + gh release create (published immediately, no draft) + attach binary. No .app bundle, no hdiutil. release.sh does the full local release including GitHub upload (option A) — no Apple signing secrets on GH runner, simpler pipeline. #37 reduced to PR CI build verification only. Draft dropped: `gh release create` is the last step, failure modes are all immediate and recoverable in <30s.
