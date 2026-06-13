---
id: 35
status: backlog
priority: Normal
tags: [release, security, post-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Notarized DMG distribution (Developer ID)

Upgrade release pipeline from ad-hoc signed (#11) to fully signed + notarized + stapled DMG. Eliminates Gatekeeper "damaged"/quarantine friction. Requires paid Apple Developer Program enrolment.

## Prerequisites

- Apple Developer Program enrolment ($99/yr individual)
- "Developer ID Application" certificate in Keychain
- App-specific password or Notary API key for `notarytool`
- App-specific password stored in Keychain item `notary-pwd`

## Scope

- Replace ad-hoc `codesign -s -` with `Developer ID Application` cert
- Add hardened runtime + entitlements.plist
- `xcrun notarytool submit ... --wait` for `.app` zip
- `xcrun stapler staple AiUsageBar.app`
- Sign + notarize + staple the `.dmg` as well
- CI: secrets for Developer ID, API key, team ID
- Update README: remove `xattr` workaround section

## Out of scope

- Homebrew cask (separate card if pursued)
- Sparkle / auto-update (covered by #20)

## Narrative
- 2026-06-13: Split from #11. User not currently enrolled in Apple Developer Program. Park as post-1.0 — pick up when willing to pay $99/yr or revenue justifies. Until then #11 ad-hoc DMG ships.
