---
id: 5
status: done
priority: Normal
tags: [ui, about, build]
spec: specs/2026-06-11-about-icon-design.md
plan: plans/2026-06-12-about-icon-version.md
created: 2026-06-11
updated: 2026-06-13
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
- 2026-06-12: Plan written (plans/2026-06-12-about-icon-version.md). Moved to doing.
  Implementation to proceed in a separate worktree (card #6 active on master).
- 2026-06-13: Implementation complete. 5 tasks via subagent-driven dev. One API fix
  (NSImage::initWithData safe, AnyThread needed for alloc). Dark mode fix added
  (setTemplate(true)). Test strengthened with image::load_from_memory dimension check.
  Cargo.toml conflict resolved on merge with card #8 (combined feature sets). Merged
  to master, 90/90 tests pass.
