# Move provider Details into the Other submenu

Card: [details-submenu](../../kanban/details-submenu.md) (id 48)

## Context

Each provider section currently ends with a flat `Details…` item that opens the
raw-JSON window for that provider. It sits awkwardly between the provider rows and
the footer. Card #44 introduced the always-present `Other ▶` submenu, currently
holding `Diagnostics ▶ → Copy diagnostic log`. We relocate the per-provider
`Details…` into `Other`, grouped per provider, next to `Diagnostics`.

### Current structure

```
Claude — max
  5h — 39%  resets 14:00
  7d — 15%  resets Mon
Details…                  ← flat item, per provider section
Copilot — …
  …
Details…
Other ▶
  Diagnostics ▶ → Copy diagnostic log
↺ Refresh
─────────
ℹ About AIUsageBar
Quit
```

### Target structure

```
Claude — max
  5h — 39%  resets 14:00
  7d — 15%  resets Mon
Copilot — …
  …
Other ▶
  Claude ▶      → Details…     (only if Claude has raw JSON)
  Copilot ▶     → Details…     (only if Copilot has raw JSON)
  Diagnostics ▶ → Copy diagnostic log   (only if diag log non-empty)
↺ Refresh
─────────
ℹ About AIUsageBar
Quit
```

## Decisions

1. **Details leaves the provider sections.** `append_claude_section` /
   `append_copilot_section` no longer append `Details…` and no longer return a
   details `MenuId`. Their return type drops to `Option<MenuId>` (the setup id for
   `NotConfigured`), or is adjusted to return only what remains.
2. **`section_item_count` drops by 1.** `Ok` → `1 + windows.len()`; all other
   states → `1` (header only).
3. **Provider `Details…` shown only when that provider has raw JSON.** Presence is
   determined by `provider.raw_json().is_some()`, computed in `main.rs` and passed
   into the menu builder (raw JSON is not part of `UsageState`).
4. **Diagnostics shown only when the diag log is non-empty.** When empty, the whole
   `Diagnostics ▶` entry is omitted (no "No diagnostics" placeholder under it).
5. **Order inside `Other`:** providers first (Claude, then Copilot), `Diagnostics`
   last.
6. **Fully-empty `Other` fallback.** If no provider has raw JSON and the diag log is
   empty, `Other` would be empty. In that single case, render one disabled
   `No diagnostics` placeholder so the submenu is never empty.

## Code changes

### `src/ui/claude.rs` / `src/ui/copilot.rs`
- `append_claude_section` / `append_copilot_section`: remove the `Details…`
  `MenuItem` and stop returning its id. Signature becomes
  `fn append_*_section(menu, state) -> Option<MenuId>` (setup id only).
- `section_item_count`: `Ok(windows, _) => 1 + windows.len()`, `_ => 1`.
- Update the section-count unit tests accordingly (the `+1 details` expectations).

### `src/ui/base.rs`
- `append_other` gains parameters describing which providers have raw JSON, e.g.
  `append_other(menu, &[(ProviderKind, bool)])` (bool = raw-JSON present), and
  returns the per-provider details ids plus the existing copy-diag id. Suggested
  return: a small struct `OtherIds { details_claude: Option<MenuId>,
  details_copilot: Option<MenuId>, copy_diag: Option<MenuId> }`.
- Build order: for each provider with raw JSON present, append a
  `Submenu::new(provider.display_name(), true)` containing a single
  `MenuItem::new("Details…", true, None)`; record its id. Then, if diag non-empty,
  append the existing `Diagnostics ▶ → Copy diagnostic log`. If nothing was
  appended, append the disabled `No diagnostics` placeholder.

### `src/ui/mod.rs`
- `build_menu` / `build_layout` signature: thread per-provider raw-JSON presence.
  Cleanest: extend the input tuple to `(ProviderKind, &UsageState, bool)` where the
  bool is raw-JSON-present, or pass a parallel slice. `build_layout` ignores the new
  bool (Other stays a single top-level row; footer indices only change because of
  the `section_item_count` decrease).
- `MenuBuild`: `details_claude` / `details_copilot` now come from `append_other`'s
  return rather than the provider sections. Struct fields unchanged.
- Update the `build_layout` unit tests: per-section counts drop by 1, so
  `refresh_idx` / `quit_idx` / window indices shift. Recompute expected values.

### `src/main.rs`
- Where `refs` is built (lines ~89 and ~197), also compute each provider's
  `raw_json().is_some()` and include it in the tuple/slice passed to `build_menu`.
- Click handler (lines ~148-157) unchanged — it already re-reads `raw_json()` at
  click time and matches `id_details_claude` / `id_details_copilot`.

## Tests

- `ui/claude.rs`, `ui/copilot.rs`: adjust `section_item_count` expectations.
- `ui/mod.rs`: recompute `build_layout` index assertions after the per-section
  count change.
- `ui/base.rs`: add tests for `append_other` — (a) both providers with raw JSON +
  non-empty diag → 3 entries; (b) one provider without raw JSON → omitted;
  (c) empty diag → Diagnostics omitted; (d) nothing present → single disabled
  placeholder.

## Out of scope

- Per-account submenus for Copilot.
- Any change to `details::show()` / `prepare_content`.
