---
id: 7
status: done
priority: High
tags: [ui, refactor]
spec: superpowers/specs/2026-06-12-ui-module-restructure.md
created: 2026-06-12
updated: 2026-06-12
plan: superpowers/plans/2026-06-12-ui-module-restructure.md
---
# UI module restructure

Extract `build_menu`, `append_label`, `MenuBuild` from `main.rs` into `src/ui/` (mod.rs, base.rs, claude.rs, copilot.rs). Pure refactor — zero visual change, no new ObjC2.

## Narrative
- 2026-06-12: Split from archived card #6. Prerequisite for all subsequent UI styling cards (#8, #9). Pure refactor — behaviour identical to current output, all 73 tests must stay green.
- 2026-06-12: Moved to doing. Writing implementation plan.
- 2026-06-12: Completed. src/ui/{mod,claude,copilot,base}.rs created. main.rs cleaned of all menu-building code. 84 tests green (73 original + 11 new ui tests). Release build clean.
