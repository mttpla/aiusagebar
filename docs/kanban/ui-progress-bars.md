---
id: 9
status: done
priority: Normal
tags: [ui, objc2, aesthetics]
blocked_by: [8]
spec: superpowers/specs/2026-06-12-ui-progress-bars.md
plan: superpowers/plans/2026-06-13-ui-progress-bars.md
created: 2026-06-12
updated: 2026-06-13
---
# UI progress bar rows

Replace plain-text window rows with custom `NSView` items: label + pct + 4pt NSBox bar colored by threshold + detail line. High ObjC2 complexity — isolated, can be deferred without breaking #7 or #8.

## Narrative
- 2026-06-12: Split from archived card #6. Green/amber/red thresholds at <60/60-80/>80%. NSBox for bar fill. resets_at formatting: relative for 5h session, absolute date for 7d weekly. Isolated by design so it can slip without blocking other work. Blocked by #8.
- 2026-06-13: Implementation plan written. Architecture: extend MenuLayout with window_items, wire make_progress_row_view into style_menu post-build pass. 4 tasks: Cargo feature, MenuLayout extension (TDD), pure helpers (TDD), ObjC view builder + wiring.
- 2026-06-13: DONE. 8 commits (234f2fb..61ec5aa). 102 tests passing. API notes: MainThreadMarker required for NSView/NSTextField alloc, NSBoxType::Custom (not NSBoxCustom), labelWithString takes mtm arg. Fixes applied: timezone-safe 7d test, secondaryLabelColor for unknown pct, Clippy deref_addrof + unnecessary_unwrap.
