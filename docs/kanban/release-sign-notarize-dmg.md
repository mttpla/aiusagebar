---
id: 11
status: backlog
priority: High
tags: [release, pre-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Unsigned DMG release (ad-hoc signed)

Ship a downloadable `.dmg` on GitHub Releases without paying Apple Developer Program. Use ad-hoc `codesign -s -` so the bundle is not flagged "damaged", and document the Gatekeeper quarantine workaround in README. Notarized release tracked separately in #35.

## Scope

- `make release-dmg` (or `scripts/release-dmg.sh`):
  - `cargo build --release`
  - Assemble `AiUsageBar.app` bundle (Info.plist, MacOS/, Resources/icon.icns)
  - `codesign --deep --force --sign - AiUsageBar.app` (ad-hoc)
  - `hdiutil create -volname AiUsageBar -srcfolder AiUsageBar.app -ov -format UDZO AiUsageBar.dmg`
- README "Installation" section:
  - Download `.dmg` from GitHub Releases
  - Drag `.app` to `/Applications`
  - First launch: System Settings → Privacy & Security → "Open Anyway", **or** `xattr -dr com.apple.quarantine /Applications/AiUsageBar.app`
- GitHub Actions release workflow: tag push → build DMG → upload as release asset.

## Out of scope

- Developer ID signing, notarization, stapling — moved to #35 (post-1.0, requires paid Apple Developer).
- Homebrew cask — post-1.0.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Without notarization, non-dev users cannot run the app. Considered ad-hoc signing (free) — rejected: Gatekeeper still blocks downloaded apps. Self-signed cert in CLAUDE.md is dev-only. Hard requirement for 1.0.0.
- 2026-06-13: Re-scoped. User not enrolled in Apple Developer Program ($99/yr). Pragmatic path: ship ad-hoc signed DMG + document `xattr` / "Open Anyway" workaround. Friction acceptable for 1.0 alpha audience. Notarization deferred to #35. README revamp (#34) still blocked_by [10, 11] — works with new scope since DMG is downloadable, just unsigned.
