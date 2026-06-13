# Claude reset time in local OS timezone

## Problem

The Claude provider's `LimitWindow.resets_at` is the raw ISO 8601 UTC string returned by `api.anthropic.com/api/oauth/usage` (e.g. `2026-06-06T14:00:00Z`). The UI renders it verbatim at `src/ui/claude.rs:33`, so users see UTC instead of their system's local time. On a machine set to `Europe/Rome`, a reset at `14:00:00Z` should display as `16:00` (CEST) or `15:00` (CET), not `2026-06-06T14:00:00Z`.

## Goal

Render Claude's `resets_at` in the menu using the operating system's local timezone, with a compact format that hides the date when the reset is on the same calendar day as "now".

## Non-goals

- Copilot UI conversion. Same pattern exists at `src/ui/copilot.rs:19` but is tracked under a separate kanban card.
- Configurable timezone override (always use OS local).
- Locale-aware date/time formatting. `%H:%M` and `%Y-%m-%d %H:%M` are universal enough.
- Relative time display ("in 2h 30m"). User requested absolute time in the local timezone.

## Design

### Display format

| Window | Reset is today (local) | Reset is later |
|---|---|---|
| 5h session | `14:30` | `2026-06-14 02:30` |
| 7d weekly  | `08:00` (rare) | `2026-06-20 08:00` |

"Today" = same calendar date as `Local::now()` at render time.

### Code layout

A single helper in `src/ui/claude.rs`:

```rust
fn format_reset_local(iso_utc: &str, now: DateTime<Local>) -> String {
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

`append_claude_section` calls it with `Local::now()`:

```rust
let reset = w
    .resets_at
    .as_deref()
    .map(|s| format_reset_local(s, Local::now()))
    .unwrap_or_else(|| "?".to_string());
```

### Why UI layer

`LimitWindow.resets_at` stays as the raw ISO 8601 string. Provider remains pure (machine-readable wire data); UI owns presentation. Mirrors the existing `pct_label` helper pattern in the same file.

### Why `now` is a parameter

Lets unit tests pass a fixed `DateTime<Local>` to exercise both same-day and different-day branches deterministically. Production call site supplies `Local::now()`.

### Error handling

`parse_from_rfc3339` failure (e.g. API returns an unexpected format) → return the raw input string. Preserves current behavior on bad data and avoids hiding the value from a debugging user.

## Tests

Add to the existing `#[cfg(test)]` block in `src/ui/claude.rs`:

1. **Same-day, Europe/Rome:** `now = 2026-06-13T10:00:00+02:00`, input `2026-06-13T12:30:00Z` → `"14:30"` (CEST).
2. **Different day, Europe/Rome:** `now = 2026-06-13T10:00:00+02:00`, input `2026-06-20T08:00:00Z` → `"2026-06-20 10:00"`.
3. **Local midnight crossing:** `now = 2026-06-13T10:00:00+02:00`, input `2026-06-13T23:30:00Z` → `"2026-06-14 01:30"` (next day in Rome).
4. **Malformed input:** input `"not-a-date"` → `"not-a-date"` (passthrough).

Tests construct `DateTime<Local>` via `Local.from_local_datetime(...)` or `DateTime::parse_from_rfc3339(...).with_timezone(&Local)` so they do not depend on the test runner's actual clock or timezone — except the local-display branches, which inherently render in the runner's TZ. For CI determinism, the same-day/different-day decisions are based on the injected `now`, not the system clock; the formatted output for branches 1–3 will only match the assertions when the runner is in `Europe/Rome`. Mark those three tests `#[cfg(target_os = "macos")]` and gate them with an env check (`TZ=Europe/Rome` developer convention documented in CLAUDE.md) — OR keep tests TZ-agnostic by asserting only the *shape* (e.g. regex `^\d{2}:\d{2}$`) and the date-vs-time branch selection. Choose the regex-shape approach to keep CI green on any host.

## Dependencies

- `chrono` already in `Cargo.toml`.
- `chrono::Local` and `chrono::DateTime` already imported elsewhere in the module (`src/provider/claude.rs:64` uses them for `format_reset_day`).

No new crates.

## Rollout

Single PR. No migration. No config changes. Behavior change is purely cosmetic in the tray menu.
