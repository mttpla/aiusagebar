---
id: 24
status: backlog
priority: Normal
tags: [ui, ux, post-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# ⌘R keyboard shortcut for Refresh

`setKeyEquivalent:@"r"` + `setKeyEquivalentModifierMask:NSEventModifierFlagCommand` on the Refresh `NSMenuItem`. Standard macOS muscle memory.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Trivial ObjC2 call inside `src/ui/base.rs` footer build. Post-1.0 — Refresh is one click away in current UI; not blocking.
