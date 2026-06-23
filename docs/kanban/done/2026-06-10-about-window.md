---
id: 3
status: done
priority: Normal
tags: [ui, about]
spec: specs/2026-06-10-about-window-design.md
plan: plans/2026-06-11-about-window.md
created: 2026-06-10
updated: 2026-06-11
---
# About window

Show app identity, version, copyright, and GitHub URL in a native macOS NSAlert,
triggered by an "About AIUsageBar" menu item.

## Narrative
- 2026-06-10: Captured from brainstorming. Chosen approach: NSAlert (no custom
  window, no new crates). Two buttons: "OK" (dismiss) + "matteopaoli.it" (opens
  https://www.matteopaoli.it via `open`). GitHub repo URL shown as plain text in
  body. Rejected: custom winit window (too heavy), NSApp standard About panel
  (requires Info.plist bundle). Copyright year range computed at runtime via
  chrono (start year 2026 hardcoded). Tagline localised it/en; disclaimer always
  English. New module src/about.rs with single pub fn show().
- 2026-06-11: Implemented. objc2 0.6 / objc2-app-kit 0.3 (NSAlert feature +
  extras for runModal/addButtonWithTitle). NSAlert::new(mtm) requires
  MainThreadMarker — safe from about_to_wait. START_YEAR const extracted.
  Locale via $LANG env var. 7 unit tests on pure logic. 66/66 total tests pass.
  Note: Italian tagline lives in src/about.rs as a string literal — conflicts
  with "no Italian in code" memory rule, but spec was approved after that rule
  was written. Defer to locale resource files when i18n is formalized.
