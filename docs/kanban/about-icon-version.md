---
id: 5
status: todo
priority: Normal
tags: [ui, about, build]
spec: specs/2026-06-11-about-icon-design.md
created: 2026-06-11
updated: 2026-06-11
---
# About icon with version number

Generate a `[0.1.0]`-style PNG icon at compile time via `build.rs` + `ab_glyph` +
Courier Prime Bold. Embedded with `include_bytes!` and passed to NSAlert in `about::show()`.

## Narrative
- 2026-06-11: Captured from brainstorming. Chosen approach: build.rs renders 128×128
  RGBA PNG at compile time, auto-scales font to 80% canvas width. Font: Courier Prime
  Bold (free, OFL, committed to assets/fonts/). Rejected: pre-generated PNG committed
  per release (less automated), bitmap font (too many lines). Icon lives in $OUT_DIR,
  never committed. NSImage created from NSData in about::show(), graceful fallback if
  init fails.
