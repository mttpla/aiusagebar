---
id: 48
status: done
priority: Normal
tags: [ui, debug, providers, pre-1.0]
depends_on: [44]
spec: ../superpowers/specs/2026-06-22-details-in-other-submenu-design.md
plan: ../superpowers/plans/2026-06-22-details-in-other-submenu.md
created: 2026-06-19
updated: 2026-06-22
---
# Move provider Details into the Other submenu

Remove the flat "Details…" item from the bottom of each provider section and relocate it under the always-present "Other ▶" submenu, as a per-provider sub-submenu sitting next to "Diagnostics". Each provider gets a "Claude ▶" / "Copilot ▶" entry inside "Other" containing its "Details…" item.

```
Other ▶
  Claude ▶      → Details…
  Copilot ▶     → Details…
  Diagnostics ▶ → Copy diagnostic log
```

## Narrative

- 2026-06-19: Brainstormed after shipping card #45. Current flat "Details…" feels aesthetically out of place. Natural home is inside the diagnostic section introduced by card #44. Decided provider-name label ("Claude", "Copilot") rather than account name to keep Copilot multi-account simple — the raw JSON body already contains per-account separators. Deferred to post-#44 so the diagnostic section exists first.
- 2026-06-22: Moved to doing. Dep #44 confirmed done. Code re-verified: `append_claude_section`/`append_copilot_section` return `(Option<MenuId>, MenuId)` where the 2nd is the flat Details item id; `build_layout` index math counts each section via `section_item_count` (header + windows + 1 details row).
- 2026-06-22: Scope revised. Original plan (submenu row per provider inside its own section) rejected — it duplicated the provider name (header already reads "Claude — max") and left Details in the section. New decision: relocate Details into the existing "Other ▶" submenu as per-provider sub-submenus next to "Diagnostics". Removes Details from provider sections entirely; `section_item_count` drops by 1; `append_other` grows to take provider states and own the Details `MenuId`s. Title/description/scope rewritten accordingly.
- 2026-06-22: Open questions resolved. (1) Provider "Details…" entry shown only when that provider has raw JSON. (2) Diagnostics entry hidden entirely when the diag log is empty (drop the "No diagnostics" placeholder there). (3) Order inside Other: providers first, Diagnostics last. Edge case: if no provider has raw JSON and diag is empty, Other would be empty — default to a single disabled "No diagnostics" placeholder only in that fully-empty case.
- 2026-06-22: Code finding — raw JSON is not in `UsageState`; it lives behind `provider.raw_json()` (mutex `last_raw_json`). `build_menu` currently takes only `&[(ProviderKind, &UsageState)]`. To gate entries on raw-JSON presence, thread a per-provider raw-JSON-present bool (or `Option<&str>`) from `main.rs` (which holds the providers) through `build_menu` → `base::append_other`. `main.rs` click handler at lines 148-157 already re-fetches `raw_json()` at click time and stays unchanged.
- 2026-06-22: Spec written and linked (`specs/2026-06-22-details-in-other-submenu-design.md`). Split check: keep as a single card — the change is atomic (removing flat Details before wiring Other would leave no Details). Proceeding to writing-plans.
- 2026-06-22: Implemented via subagent-driven development on branch `feat/details-in-other-submenu` (commits dcb4e6e, 03389a5, 222ae08, acb270d). Per-task reviews + opus whole-branch review all clean (0 Critical/Important). Final state: Details lives only inside "Other ▶" as per-provider sub-submenus, shown only when the provider has raw JSON; Diagnostics shown only when the diag log is non-empty; disabled "No diagnostics" placeholder only when Other would otherwise be empty. `section_item_count` dropped by 1; click handler in main.rs unchanged. 185 tests pass, clippy clean. Plan brief had one arithmetic slip in a build_layout test (caught by implementer, confirmed by reviewer). Done.

## Scope

- Remove the flat `MenuItem::new("Details…")` from `append_claude_section` / `append_copilot_section` in `ui/claude.rs` and `ui/copilot.rs`. These functions stop returning a details `MenuId` (return only the `setup` Option for `NotConfigured`).
- `section_item_count` drops by 1: header + windows for `Ok`, header only otherwise.
- `base::append_other` builds, for each provider, a `Submenu` named after the provider containing a single "Details…" item; returns the inner item IDs. "Diagnostics ▶" stays as the last entry.
- `append_other` now needs the provider states/kinds (currently takes none) to know which provider sub-submenus to render and wire their `MenuId`s.
- `MenuBuild.details_claude` / `details_copilot` are populated from `append_other`'s return, not from the provider sections. Click handler in `main.rs` unchanged (still matches those IDs, same `details::show()` call).
- `build_layout` footer indices recomputed after the per-section count change; "Other ▶" remains a single top-level row.
- `styled.rs` unaffected by content of Other (no top-level row added/removed).

## Out of scope

- Per-account submenus for Copilot (future, if multi-account details needed separately).
- Any changes to `details::show()` or `prepare_content`.

## Open questions (resolve in spec)

- Show a provider's "Details…" entry always, or only when there is raw JSON to show (e.g. hide for `NotConfigured`)?
- "No diagnostics" placeholder: when diag log is empty but providers exist, "Other" still has provider entries — does the placeholder still appear under Diagnostics only, or is the whole Diagnostics entry hidden?
- Provider ordering inside "Other" and position relative to Diagnostics (providers first, Diagnostics last assumed).
