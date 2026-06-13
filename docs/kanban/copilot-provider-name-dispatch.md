---
id: 33
status: done
priority: High
tags: [bug, refactor, provider, ui]
plan: ../superpowers/plans/2026-06-13-copilot-provider-name-dispatch.md
created: 2026-06-13
updated: 2026-06-13
---
# Fix Copilot provider dispatch + replace string keys with ProviderKind enum

Copilot section renders `"GitHub: unknown provider"` in tray menu. `CopilotProvider::name()` returns `"GitHub"` but UI dispatch in `src/ui/mod.rs` (`build_menu`, `build_layout`) matches on `"Copilot"`. Fallback arm prints "unknown provider". Regression introduced by UI restructure commits `4bd4676` + `1870c84` — old UI read state directly without string dispatch.

## Goal

Eliminate whole class of bug at compile time: trait returns enum, UI matches exhaustively. Provider rename drift becomes impossible to ship.

## Scope

- Replace `UsageProvider::name(&self) -> &'static str` with `kind(&self) -> ProviderKind`.
- Move `ProviderKind` from `src/ui/mod.rs` into `src/provider/mod.rs` (trait surface).
- `ClaudeProvider::kind()` → `ProviderKind::Claude`. `CopilotProvider::kind()` → `ProviderKind::Copilot`.
- Update `build_menu` + `build_layout` to take `&[(ProviderKind, &UsageState)]`, match exhaustively (no fallback arm).
- Update `main.rs` registration + refresh loop to pass `ProviderKind` instead of `&str`.
- Update display label: header uses literal `"Copilot"` (already correct in `src/ui/copilot.rs`); confirm preserved.
- README rename: line 15 table row "GitHub" → "Copilot". Keep "GitHub Copilot" in prose intro (line 3).
- Tests (T3, TDD-first): exhaustive enum dispatch is the compile-time guard; add runtime contract test that real registered providers produce a menu with no "unknown provider" label and that header labels match `ProviderKind` display.

## Out of scope

- Codex provider (separate kanban card).
- Tray icon / header style changes.

## Narrative

- 2026-06-13: Captured from brainstorming.
  - **Symptom:** menu shows `"GitHub: unknown provider"` for Copilot section.
  - **Root cause:** `src/provider/copilot.rs:127` returns `"GitHub"`; `src/ui/mod.rs:43-62 + 87-90` dispatch matches `"Claude"` / `"Copilot"` only, fallback prints `unknown provider`.
  - **Why silent:** no test asserts `provider.name()` agrees with UI dispatch keys.
  - **Decisions:**
    - Display label = `"Copilot"` (user pick; official is GitHub Copilot but Copilot is concise).
    - Fix strategy = A+C: minimal name change + refactor to `ProviderKind` enum. Pure-A rejected — leaves stringly-typed dispatch and bug can regress.
    - Tests = T3 only: enum exhaustiveness replaces T1 (name assertion) and T2 (no-"unknown" runtime check) at compile time. Add one runtime contract test for header label rendering.
    - Order = TDD (tests first, then refactor, single commit).
    - No split: trait surface + UI + README + tests are tightly coupled.
  - **Rejected options:**
    - Pure-A (change `name()` → `"Copilot"`, leave string dispatch): regression-prone; rejected.
    - Pure-B (change UI to match `"GitHub"`): contradicts header literal `"Copilot"` and plans/kanban naming; rejected.
    - Split into two cards (refactor / README): churn for 1-line README diff; rejected.
- 2026-06-13: DONE. Trait now exposes `kind() -> ProviderKind`. UI dispatch
  exhaustive — fallback arm removed. Copilot section renders "Copilot" header.
  README provider table normalised. All tests pass, no clippy warnings.
