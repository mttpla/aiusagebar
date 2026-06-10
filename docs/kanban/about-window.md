---
id: 3
status: backlog
priority: Normal
tags: [ui, about]
spec: specs/2026-06-10-about-window-design.md
created: 2026-06-10
updated: 2026-06-10
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
