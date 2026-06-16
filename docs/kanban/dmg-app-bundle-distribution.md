---
id: 42
status: backlog
priority: Normal
blocked_by: [11, 35]
tags: [release, future]
created: 2026-06-16
updated: 2026-06-16
---
# DMG distribution with .app bundle

Replace raw binary release (#11) with a proper macOS `.dmg` containing `AiUsageBar.app`. Requires Developer ID signing (#35) to avoid Gatekeeper friction on first open. Provides standard drag-to-Applications install UX.

## Scope

- `scripts/release-dmg.sh`:
  - `cargo build --release`
  - Assemble `AiUsageBar.app` bundle: `Contents/MacOS/aiusagebar`, `Contents/Info.plist`, `Contents/Resources/icon.icns`
  - `codesign --deep --force --sign "Developer ID Application: ..." AiUsageBar.app`
  - `xcrun notarytool submit ...` + `xcrun stapler staple AiUsageBar.app`
  - `hdiutil create -volname AiUsageBar -srcfolder AiUsageBar.app -ov -format UDZO AiUsageBar-vX.Y.Z.dmg`
  - Sign + staple the `.dmg` as well
- Integrate into release.sh in place of raw binary step
- Update README: remove `xattr` workaround, standard drag-and-drop install

## Out of scope

- Homebrew cask (separate card if pursued)
- Auto-update / Sparkle (see #20)

## Narrative
- 2026-06-16: Split from #11 re-scope. Raw binary is sufficient for pre-1.0 alpha audience. DMG provides better UX (drag-to-Applications, no xattr workaround) but requires paid Apple Developer Program (#35). Park until notarization is in scope. Blocked by #11 (build pipeline must be stable first) and #35 (Developer ID cert required for Gatekeeper-clean DMG).
