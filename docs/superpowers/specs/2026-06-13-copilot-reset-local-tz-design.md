# Copilot reset time in local OS timezone

## Problem

The Copilot provider stores `LimitWindow.resets_at` as the raw ISO 8601 UTC string from GitHub's `quota_reset_date_utc` field (e.g. `2026-07-01T00:00:00Z`). `src/ui/copilot.rs:19` renders it verbatim. On a machine set to `Europe/Rome` the user sees a UTC timestamp instead of their local time. This card mirrors card #31 (Claude) for the Copilot provider.

## Goal

Render Copilot's `resets_at` in the menu using the OS local timezone, with the same compact format used by card #31:

- `HH:MM` when the reset is on the same local calendar day as `Local::now()`.
- `YYYY-MM-DD HH:MM` otherwise.
- Malformed input → raw passthrough.

## Non-goals

- Claude UI conversion (card #31).
- Configurable timezone override.
- Locale-aware date/time formatting.
- Relative time display.
- Reformatting Copilot's monthly midnight-UTC convention into something more "natural" (e.g. "next month"). Out of scope.

## Design

### Shared helper

The conversion logic is identical to card #31. Place it in a new module `src/ui/time.rs` exposing a single function:

```rust
use chrono::{DateTime, Local};

pub(crate) fn format_reset_local(iso_utc: &str, now: DateTime<Local>) -> String {
    match DateTime::parse_from_rfc3339(iso_utc) {
        Ok(dt) => {
            let local = dt.with_timezone(&Local);
            if local.date_naive() == now.date_naive() {
                local.format("%H:%M").to_string()
            } else {
                local.format("%Y-%m-%d %H:%M").to_string()
            }
        }
        Err(_) => iso_utc.to_string(),
    }
}
```

Wire it into `src/ui/mod.rs` as `pub(crate) mod time;`.

Both `src/ui/claude.rs` and `src/ui/copilot.rs` call `super::time::format_reset_local`.

### Call site change in `src/ui/copilot.rs`

`row_label` becomes:

```rust
pub(crate) fn row_label(window: &LimitWindow, now: DateTime<Local>) -> String {
    let pct = window
        .percent_used
        .map(|p| format!("{:.1}%", p))
        .unwrap_or_else(|| "—".to_string());
    let reset = window
        .resets_at
        .as_deref()
        .map(|s| super::time::format_reset_local(s, now))
        .unwrap_or_else(|| "?".to_string());
    format!("  {} — {}  resets {}", window.name, pct, reset)
}
```

`append_copilot_section` calls `row_label(w, Local::now())`.

Existing unit tests for `row_label` are updated to pass a fixed `DateTime<Local>` and assert against either exact `HH:MM` (shape only) or the date+time branch.

### Coordination with card #31

Card #31 may or may not have extracted the helper to `src/ui/time.rs` already:

- **If #31 lands first and extracts `src/ui/time.rs`:** this card only adds the call site change in `copilot.rs`, updates `row_label` tests, and uses the existing helper. No `time.rs` work.
- **If #31 lands first with the helper inlined in `claude.rs`:** this card extracts the helper to `src/ui/time.rs`, switches `claude.rs` to use it, removes the duplicate, and adds the Copilot call site.
- **If this card lands first:** it creates `src/ui/time.rs` and wires Copilot. #31 then consumes the same helper.

The narrative in `docs/kanban/copilot-reset-local-tz.md` flags this; the implementation plan checks `src/ui/` state before deciding which of the three branches to execute.

## Tests

Add to `src/ui/copilot.rs`'s existing `#[cfg(test)]` block:

1. **Same-day reset:** `now` fixed, input ISO is on the same local date → result shape is `^\d{2}:\d{2}$` and assertion checks `row_label` produces `"  Daily — 42.5%  resets HH:MM"` with `HH:MM` matched as a regex.
2. **Different-day reset:** input ISO is later → `row_label` ends with `"resets YYYY-MM-DD HH:MM"` matched as a regex.
3. **Malformed `resets_at`:** input `"not-a-date"` → `row_label` ends with `"resets not-a-date"`.
4. **`resets_at` is None:** unchanged behavior, ends with `"resets ?"`.

Add to a new `#[cfg(test)] mod tests` in `src/ui/time.rs`:

1. Parse + same-day branch (regex shape).
2. Parse + different-day branch (regex shape).
3. Parse failure passthrough.

The regex-shape approach keeps tests TZ-agnostic and CI-portable, matching the choice made in card #31's spec.

## Dependencies

- `chrono` already in `Cargo.toml`.
- `regex` is **not** currently a dependency. Tests can avoid it by using `str::chars` checks (e.g. `assert!(out.ends_with_pattern_hhmm())`) or `str::split`/`str::matches`. Prefer the hand-rolled approach over adding a dev-dependency for four assertions.

No new runtime crates. Possibly no new dev-dependencies either.

## Rollout

Single PR. No migration. Behavior change is cosmetic in the Copilot section of the tray menu. If card #31 is in flight on the same branch, coordinate to land them in series to avoid a merge conflict in `src/ui/mod.rs`.

## Status quo (2026-06-15)

Card #31 landed with `format_reset_local` inlined as a private `fn` in `src/ui/claude.rs:20`. No `src/ui/time.rs` existed. **Branch 2 applied:** extracted helper to `src/ui/time.rs`, switched `claude.rs` to call it, added call site in `copilot.rs`. Implementation complete.
