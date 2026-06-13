---
id: 11
status: backlog
priority: High
tags: [release, security, pre-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Sign + notarize + DMG distribution

Produce a signed and notarized `.app` packaged in a `.dmg` so first-launch does not hit Gatekeeper "damaged, move to trash". Requires Apple Developer ID cert ($99/yr), `codesign`, `notarytool submit --wait`, `xcrun stapler staple`, then `create-dmg` or `hdiutil`.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Without notarization, non-dev users cannot run the app. Considered ad-hoc signing (free) — rejected: Gatekeeper still blocks downloaded apps. Self-signed cert in CLAUDE.md is dev-only. Hard requirement for 1.0.0.
