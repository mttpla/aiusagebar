---
id: 48
status: todo
priority: Normal
tags: [ui, debug, providers, pre-1.0]
depends_on: [44]
created: 2026-06-19
updated: 2026-06-19
---
# Move Details into provider submenu row

Replace the flat "Details…" menu item at the bottom of each provider section with a submenu row labelled with the provider name (e.g. "Claude ▶", "Copilot ▶"). Clicking opens the submenu; "Details…" lives inside it.

## Narrative

- 2026-06-19: Brainstormed after shipping card #45. Current flat "Details…" feels aesthetically out of place. Natural home is inside the diagnostic section introduced by card #44. Decided provider-name label ("Claude", "Copilot") rather than account name to keep Copilot multi-account simple — the raw JSON body already contains per-account separators. Deferred to post-#44 so the diagnostic section exists first.

## Scope

- Replace `MenuItem::new("Details…")` in `ui/claude.rs` and `ui/copilot.rs` with a `Submenu` named after the provider.
- Single "Details…" item inside each submenu.
- Click handler in `main.rs` wired to the inner item's `MenuId` (same `details::show()` call, unchanged).
- `MenuBuild.details_claude` / `details_copilot` point to the inner item ID (no change to `App` struct or click handler logic).
- `section_item_count` updated: submenu counts as 1 item regardless of state.

## Out of scope

- Per-account submenus for Copilot (future, if multi-account details needed separately).
- Any changes to `details::show()` or `prepare_content`.
