# Claude enterprise usage parse — dual-shape support

## Problem

The Claude usage endpoint (`/api/oauth/usage`) returns a different JSON shape for
enterprise contracts. For Pro/Max accounts `five_hour` and `seven_day` are
objects; for enterprise both are `null`, and usage is carried in a `spend`
object plus a set of dollar "budget" windows with rotating codename keys.

The current parser declares `five_hour: WindowData` / `seven_day: WindowData`
(non-optional), so an enterprise response crashes deserialization:

```
invalid type: null, expected struct WindowData at line 1 column 17
```

Enterprise users get a persistent `Parse error` and no usage display.

### Enterprise response (observed)

```
"five_hour": null,
"seven_day": null,
...rotating codename windows (cinder_cove, amber_ladder, ...) — ignored...
"spend": {
  "percent": 0,
  "used":  {"amount_minor": 0,    "currency": "USD", "exponent": 2},
  "limit": {"amount_minor": 5000, "currency": "USD", "exponent": 2},
  "severity": "normal", ...
}
```

### Profile shape (observed, Pro account)

```
"account":      {"email": "...", "has_claude_pro": true, "has_claude_max": false, ...},
"organization": {"organization_type": "claude_pro", "billing_type": "stripe_subscription",
                 "seat_tier": null, ...}
```

`organization.organization_type` is the real plan discriminator (`claude_pro`,
presumably `claude_max`, and an enterprise value such as `claude_enterprise`).
`/api/oauth/profile` is an internal, undocumented endpoint — no official schema
exists, so the enterprise `organization_type` value is inferred, not confirmed.

## Goal

One parse path handles both Pro/Max and enterprise usage shapes. Enterprise
users see a Spend bar (percent + dollars spent / limit). The plan label
(`pro` / `max` / `enterprise`) comes from a single source: the profile.

No license-conditional branching beyond reading which optional fields are
present. Pro/Max rendering stays byte-identical to today.

## Design

### 1. `LimitWindow` gains optional money fields (`provider/mod.rs`)

```rust
spent:    Option<f64>,    // dollars used
budget:   Option<f64>,    // dollars limit
currency: Option<String>, // e.g. "USD"
```

`percent_used` is already `Option<f32>`. The parser emits raw values only — no
pre-formatted strings. The view formats them. Existing `limit`/`remaining`
(token-count `u32`, unused by the styled renderer) are left untouched.

### 2. Usage structs all optional, money typed (`provider/claude.rs`)

```rust
struct UsageResponse {
    five_hour: Option<WindowData>,
    seven_day: Option<WindowData>,
    spend:     Option<SpendData>,
}
struct SpendData { percent: f32, used: Money, limit: Money }
struct Money { amount_minor: i64, exponent: u32, currency: String }
```

Money to dollars: `amount_minor as f64 / 10f64.powi(exponent as i32)`.
`serde` keeps ignoring unknown fields (no `deny_unknown_fields`), so the
rotating codename budget windows pass silently.

### 3. `parse_response` — single pass, no bool, no license branch

Return type changes from `[LimitWindow; 2]` to `Vec<LimitWindow>`. One window is
pushed per present field:

- `five_hour` `Some` → 5h window (`percent_used` = utilization, `resets_at`)
- `seven_day` `Some` → 7d window
- `spend` `Some` → Spend window (`percent_used`, `spent`, `budget`, `currency`)

Pro/Max → 2 windows. Enterprise → 1 Spend window. Absent fields produce no row.
Mutual exclusivity falls out of the data shape — no flag is threaded.

### 4. Plan label from `organization_type` (`provider/claude.rs`)

`ProfileResponse` gains `organization { organization_type: String }`. The plan
label is derived by stripping the `claude_` prefix:

```
"claude_pro"        -> "pro"
"claude_max"        -> "max"
"claude_enterprise" -> "enterprise"   (exact value unconfirmed; strip handles any)
unknown / empty     -> fallback: has_claude_max -> "max", has_claude_pro -> "pro", else "free"
```

This is the single source for the plan label. There is **no** usage-based
enterprise inference. Window selection (which usage shape) and plan label
(profile) are independent concerns.

### 5. View formats money (`ui/styled.rs`)

In `make_progress_row_view`, the detail line becomes:

- if `spent` and `budget` are `Some` → `format!("{sym}{spent:.2} / {sym}{budget:.2}")`
  where `sym` = `"$"` for `currency == "USD"`, else `"{currency} "`.
- else → `format_reset(window)` (today's behavior).

The percent label and bar already handle `percent_used == None` (show `—`, empty
bar). Only windows that carry data are created, so no empty bars appear. The
menu header already shows username + plan label.

Enterprise user result: one row — **Spend — 0.0% · $0.00 / $50.00**.

## Testing

### Mock fixtures (string consts in test module)

- Profile: `pro` (the observed dump), `max`, `enterprise` (best-guess
  `organization_type: "claude_enterprise"`, both `has_*` flags false).
- Usage: Pro/Max shape (`five_hour` + `seven_day` objects), enterprise shape
  (`five_hour`/`seven_day` null + `spend`, plus a codename window to assert it
  is ignored).

### Tests

1. `parse_profile_response` → plan label per fixture: `claude_pro`→`pro`,
   `claude_max`→`max`, `claude_enterprise`→`enterprise`.
2. `organization_type` strip + fallback: empty/missing `organization_type`
   falls back to `has_claude_max`/`has_claude_pro`/`free`.
3. `parse_response` Pro/Max usage → 2 windows (5h, 7d), `percent_used` +
   `resets_at` correct, money fields `None`.
4. `parse_response` enterprise usage → exactly 1 Spend window;
   `percent_used`/`spent`/`budget`/`currency` populated; no 5h/7d window;
   codename window ignored.
5. Money conversion: `amount_minor` 5000, `exponent` 2 → `50.0`.
6. Bar selection per case: assert the returned `Vec<LimitWindow>` (count, names,
   which money fields are populated vs `None`) — this is the regression guard
   for "which bars are shown" across pro/max/enterprise.
7. Detail-line formatting: money present → `"$0.00 / $50.00"`; money absent →
   reset text (existing `format_reset` paths unchanged).

## Constraints (unchanged)

Tokens read-only; no refresh; 180s poll floor; matching `User-Agent`; every
failure is a state, never a panic. None of these are touched by this work.
