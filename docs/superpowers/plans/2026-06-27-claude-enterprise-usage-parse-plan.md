# Claude Enterprise Usage Parse — Dual-Shape Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One parse path handles both Pro/Max and enterprise Claude usage shapes, so enterprise accounts see a Spend bar (percent + dollars) instead of a persistent parse error.

**Architecture:** Make `five_hour`/`seven_day`/`spend` all optional in the usage response; `parse_response` returns a `Vec<LimitWindow>` pushing one window per present field — mutual exclusivity falls out of the data shape, no license flag is threaded. `LimitWindow` gains optional raw money fields; the view formats them. The plan label comes solely from the profile's `organization.organization_type` (strip `claude_` prefix) with a `has_claude_*` fallback.

**Tech Stack:** Rust, `serde`/`serde_json` (deserialization), `objc2`/`objc2_app_kit` (macOS menu view).

## Global Constraints

- Tokens are read-only — never write Keychain or credential files.
- No token refresh for Claude. On 401: `Stale`, never refresh.
- Every provider failure is a state (`NotConfigured`/`Stale`/`Error`), never a panic.
- All `.rs` string literals must be English (Italian only for runtime i18n, not present here).
- Never `pub` in this bin crate — default `pub(crate)`/private so real dead code surfaces.
- Never add `#[allow(dead_code)]` — delete unused symbols instead. (Consequence: every struct field added must be read by non-test code in the same commit, or `cargo clippy` fails.)
- Before every commit run `cargo clippy -- -D warnings && cargo test` — both must be clean.
- `serde` keeps ignoring unknown fields (no `deny_unknown_fields`), so rotating codename budget windows pass silently. Do not add `deny_unknown_fields`.
- Pro/Max rendering stays byte-identical to today.
- Commit messages: Conventional Commits, no `Co-Authored-By` trailer.

---

## File Structure

- `src/provider/mod.rs` — `LimitWindow` struct gains `spent`/`budget`/`currency` optional money fields.
- `src/ui/styled.rs` — new pure `format_money` + `format_detail` helpers; `make_progress_row_view` uses `format_detail` for the detail line.
- `src/provider/claude.rs` — usage structs become optional + money-typed; `parse_response` returns `Vec<LimitWindow>`; `do_fetch` caller adjusted; profile gains `organization_type`-based plan label.
- Mechanical: `src/provider/copilot.rs`, `src/icon.rs`, `src/ui/copilot.rs` (test) — existing `LimitWindow` full struct literals converted to `..Default::default()` spread so they survive the new fields.

---

## Task 1: `LimitWindow` money fields + view formatting

Adds the three optional money fields and makes the view render them. The view is the **reader** of the new fields, so this task keeps `cargo clippy` clean (no "field never read"). The parser still returns `[LimitWindow; 2]` here and never populates money — `format_detail` falls back to `format_reset`, so Pro/Max rendering is unchanged. This task also converts every existing full `LimitWindow` literal to spread form so the field addition compiles crate-wide.

**Files:**
- Modify: `src/provider/mod.rs:4-12` (add fields)
- Modify: `src/ui/styled.rs` (add `format_money`, `format_detail`; wire into `make_progress_row_view:250`; add tests)
- Modify: `src/provider/copilot.rs:37`, `src/provider/copilot.rs:107`, `src/provider/copilot.rs:117` (spread)
- Modify: `src/provider/claude.rs:156`, `src/provider/claude.rs:164` (spread; parse_response body, return type unchanged here)
- Modify: `src/icon.rs:77`, `src/icon.rs:173` (test literals, spread)
- Modify: `src/ui/copilot.rs:64` (test literal, spread)
- Modify: `src/provider/claude.rs:516`, `src/provider/claude.rs:606` (test literals, spread)
- Test: `src/ui/styled.rs` (test module, `format_money`/`format_detail`)

**Interfaces:**
- Consumes: nothing from other tasks.
- Produces:
  - `LimitWindow` fields `spent: Option<f64>`, `budget: Option<f64>`, `currency: Option<String>` (all default `None` via existing `#[derive(Default)]`).
  - `fn format_money(spent: f64, budget: f64, currency: &str) -> String` (private to `styled.rs`).
  - `fn format_detail(window: &crate::provider::LimitWindow) -> String` (private to `styled.rs`) — money line if `spent`+`budget` both `Some`, else `format_reset(window)`.

- [ ] **Step 1: Add money fields to `LimitWindow`**

In `src/provider/mod.rs`, change the struct (keep `#[derive(Debug, Clone, PartialEq, Default)]`):

```rust
pub(crate) struct LimitWindow {
    pub(crate) name: String,
    pub(crate) percent_used: Option<f32>,
    pub(crate) limit: Option<u32>,
    pub(crate) remaining: Option<u32>,
    pub(crate) resets_at: Option<String>,
    pub(crate) unlimited: bool,
    pub(crate) spent: Option<f64>,
    pub(crate) budget: Option<f64>,
    pub(crate) currency: Option<String>,
}
```

- [ ] **Step 2: Convert existing full literals to spread form (make it compile)**

Adding fields breaks every literal that lists all fields exhaustively. Convert each of the following, keeping the meaningful assignments and dropping default-valued ones in favor of `..Default::default()`.

`src/provider/copilot.rs:37`:
```rust
        windows.push(LimitWindow {
            name: format!("{} / {}", login, key),
            percent_used: Some(percent_used),
            limit,
            remaining,
            resets_at: resets_at.clone(),
            ..Default::default()
        });
```

`src/provider/copilot.rs:107`:
```rust
            ok_windows.push(LimitWindow {
                name: format!("@{} — token expired, re-login", account),
                ..Default::default()
            });
```

`src/provider/copilot.rs:117`:
```rust
            ok_windows.push(LimitWindow {
                name: msg,
                ..Default::default()
            });
```

`src/provider/claude.rs:155-172` (`parse_response` body — return type and logic unchanged, just spread the two literals):
```rust
    Ok([
        LimitWindow {
            name: "5h session".to_string(),
            percent_used: Some(resp.five_hour.utilization),
            resets_at: resp.five_hour.resets_at,
            ..Default::default()
        },
        LimitWindow {
            name: "7d weekly".to_string(),
            percent_used: Some(resp.seven_day.utilization),
            resets_at: resp.seven_day.resets_at,
            ..Default::default()
        },
    ])
```

`src/icon.rs:77` (test helper):
```rust
    fn window(pct: Option<f32>) -> LimitWindow {
        LimitWindow {
            name: "t".into(),
            percent_used: pct,
            ..Default::default()
        }
    }
```

`src/icon.rs:173` (test — note `unlimited: true` is non-default, keep it):
```rust
        let w = LimitWindow {
            name: "t".into(),
            percent_used: Some(90.0),
            unlimited: true,
            ..Default::default()
        };
```

`src/ui/copilot.rs:64` (test helper):
```rust
    fn make_window(name: &str, pct: Option<f32>, resets_at: Option<&str>) -> LimitWindow {
        LimitWindow {
            name: name.to_owned(),
            percent_used: pct,
            resets_at: resets_at.map(str::to_owned),
            ..Default::default()
        }
    }
```

`src/provider/claude.rs:516` and `src/provider/claude.rs:606` (both identical test caches):
```rust
        let cache = Mutex::new(Some(vec![LimitWindow {
            name: "5h session".to_string(),
            percent_used: Some(42.0),
            ..Default::default()
        }]));
```

- [ ] **Step 3: Write the failing tests for money formatting**

In the `src/ui/styled.rs` `#[cfg(test)] mod tests` block, add:

```rust
    #[test]
    fn format_money_usd_uses_dollar_symbol() {
        assert_eq!(super::format_money(0.0, 50.0, "USD"), "$0.00 / $50.00");
    }

    #[test]
    fn format_money_non_usd_prefixes_currency_code() {
        assert_eq!(super::format_money(1.5, 20.0, "EUR"), "EUR 1.50 / EUR 20.00");
    }

    #[test]
    fn format_detail_money_present_formats_dollars() {
        let mut w = make_window("Spend", Some(0.0), None);
        w.spent = Some(0.0);
        w.budget = Some(50.0);
        w.currency = Some("USD".to_string());
        assert_eq!(super::format_detail(&w), "$0.00 / $50.00");
    }

    #[test]
    fn format_detail_money_absent_falls_back_to_reset() {
        let w = make_window("7d weekly", Some(15.0), Some("2026-07-01T00:00:00+00:00"));
        assert_eq!(super::format_detail(&w), super::format_reset(&w));
    }
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test --lib ui::styled::tests::format_ -- --nocapture`
Expected: FAIL — `format_money` / `format_detail` not found (cannot resolve function).

- [ ] **Step 5: Implement `format_money` and `format_detail`**

In `src/ui/styled.rs`, next to `format_reset` (around line 140), add:

```rust
fn format_money(spent: f64, budget: f64, currency: &str) -> String {
    let sym = if currency == "USD" {
        "$".to_string()
    } else {
        format!("{} ", currency)
    };
    format!("{sym}{spent:.2} / {sym}{budget:.2}")
}

fn format_detail(window: &crate::provider::LimitWindow) -> String {
    match (window.spent, window.budget) {
        (Some(spent), Some(budget)) => {
            let currency = window.currency.as_deref().unwrap_or("USD");
            format_money(spent, budget, currency)
        }
        _ => format_reset(window),
    }
}
```

- [ ] **Step 6: Wire `format_detail` into the view**

In `make_progress_row_view`, change `src/ui/styled.rs:250` from:

```rust
    let detail = format_reset(window);
```

to:

```rust
    let detail = format_detail(window);
```

- [ ] **Step 7: Run clippy + full test suite**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: PASS — no "field never read" (view reads `spent`/`budget`/`currency`), all tests green.

- [ ] **Step 8: Commit**

```bash
git add src/provider/mod.rs src/ui/styled.rs src/provider/copilot.rs src/provider/claude.rs src/icon.rs src/ui/copilot.rs
git commit -m "feat(claude): add money fields to LimitWindow and money-aware detail line"
```

---

## Task 2: Optional usage structs + Spend window parsing

Rewrites the Claude usage deserialization so `five_hour`/`seven_day`/`spend` are all optional and money-typed, and `parse_response` returns `Vec<LimitWindow>` — one window per present field. Enterprise bodies (null windows + `spend`) now produce one Spend window with populated money fields (read by the Task 1 view, so clippy stays clean).

**Files:**
- Modify: `src/provider/claude.rs:141-173` (structs + `parse_response`)
- Modify: `src/provider/claude.rs:238-243` (`do_fetch` Ok arm)
- Test: `src/provider/claude.rs` (test module)

**Interfaces:**
- Consumes: `LimitWindow` money fields from Task 1.
- Produces: `fn parse_response(body: &str) -> Result<Vec<LimitWindow>, String>`.

- [ ] **Step 1: Write failing parser tests**

In the `src/provider/claude.rs` `#[cfg(test)] mod tests` block, add these. (`USAGE_ENTERPRISE` is the real body from the bug log; it includes a codename window `cinder_cove` to assert it is ignored.)

```rust
    const USAGE_PRO_MAX: &str = r#"{"five_hour":{"utilization":12.5,"resets_at":"2026-06-26T22:00:00+00:00"},"seven_day":{"utilization":40.0,"resets_at":"2026-07-01T00:00:00+00:00"}}"#;

    const USAGE_ENTERPRISE: &str = r#"{"five_hour":null,"seven_day":null,"cinder_cove":{"utilization":1.3e-06,"resets_at":"2026-09-21T07:09:14.289383+00:00","limit_dollars":1000,"used_dollars":1.3e-05,"remaining_dollars":999.999987},"limits":[],"spend":{"used":{"amount_minor":0,"currency":"USD","exponent":2},"limit":{"amount_minor":5000,"currency":"USD","exponent":2},"percent":0,"severity":"normal","enabled":true}}"#;

    // Richer real enterprise body: many extra null/codename windows plus deeply
    // nested objects (extra_usage, spend.cap) — all must be ignored. spend limit
    // amount_minor 5000 / exponent 2 -> $50.00.
    const USAGE_ENTERPRISE_FULL: &str = r#"{"five_hour":null,"seven_day":null,"seven_day_oauth_apps":null,"seven_day_opus":null,"seven_day_sonnet":null,"seven_day_cowork":null,"seven_day_omelette":null,"tangelo":null,"iguana_necktie":null,"omelette_promotional":{"utilization":0.0,"resets_at":null,"limit_dollars":null,"used_dollars":null,"remaining_dollars":null},"cinder_cove":{"utilization":1.2999999999999998e-06,"resets_at":"2026-09-21T07:09:14.289383+00:00","limit_dollars":1000,"used_dollars":1.3e-05,"remaining_dollars":999.999987},"amber_ladder":{"utilization":0.0,"resets_at":"2026-09-02T06:59:59+00:00","limit_dollars":25000,"used_dollars":0.0,"remaining_dollars":25000.0},"extra_usage":{"is_enabled":true,"monthly_limit":5000,"used_credits":0.0,"utilization":null,"currency":"USD","decimal_places":2,"disabled_reason":null,"daily":null,"weekly":null},"limits":[],"spend":{"used":{"amount_minor":0,"currency":"USD","exponent":2},"limit":{"amount_minor":5000,"currency":"USD","exponent":2},"percent":0,"severity":"normal","enabled":true,"disabled_reason":null,"cap":{"money":null,"credits":{"amount_minor":5000,"exponent":2}},"balance":null,"auto_reload":null,"disclaimer":"Usage credits cover you when you hit your plan limits.","can_purchase_credits":false,"can_toggle":false}}"#;

    #[test]
    fn parse_pro_max_yields_two_windows_no_money() {
        let windows = super::parse_response(USAGE_PRO_MAX).unwrap();
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].name, "5h session");
        assert_eq!(windows[0].percent_used, Some(12.5));
        assert_eq!(windows[0].resets_at.as_deref(), Some("2026-06-26T22:00:00+00:00"));
        assert_eq!(windows[1].name, "7d weekly");
        assert_eq!(windows[1].percent_used, Some(40.0));
        assert!(windows[0].spent.is_none() && windows[0].budget.is_none());
        assert!(windows[1].spent.is_none() && windows[1].budget.is_none());
    }

    #[test]
    fn parse_enterprise_yields_single_spend_window() {
        let windows = super::parse_response(USAGE_ENTERPRISE).unwrap();
        assert_eq!(windows.len(), 1, "enterprise must produce exactly one window");
        let w = &windows[0];
        assert_eq!(w.name, "Spend");
        assert_eq!(w.percent_used, Some(0.0));
        assert_eq!(w.spent, Some(0.0));
        assert_eq!(w.budget, Some(50.0));
        assert_eq!(w.currency.as_deref(), Some("USD"));
    }

    #[test]
    fn parse_enterprise_full_ignores_extra_keys_single_spend_window() {
        let windows = super::parse_response(USAGE_ENTERPRISE_FULL).unwrap();
        assert_eq!(windows.len(), 1, "extra null/codename/nested keys must be ignored");
        let w = &windows[0];
        assert_eq!(w.name, "Spend");
        assert_eq!(w.percent_used, Some(0.0));
        assert_eq!(w.spent, Some(0.0));
        assert_eq!(w.budget, Some(50.0));
        assert_eq!(w.currency.as_deref(), Some("USD"));
    }

    #[test]
    fn parse_money_minor_units_to_dollars() {
        // amount_minor 5000, exponent 2 -> 50.0
        let windows = super::parse_response(USAGE_ENTERPRISE).unwrap();
        assert_eq!(windows[0].budget, Some(50.0));
    }

    #[test]
    fn parse_empty_object_yields_no_windows() {
        let windows = super::parse_response("{}").unwrap();
        assert!(windows.is_empty());
    }

    #[test]
    fn parse_malformed_json_is_error() {
        assert!(super::parse_response("not json").is_err());
    }
```

Delete the now-obsolete `parse_missing_field_is_error` test (`src/provider/claude.rs:392-395`) — `{}` no longer errors; `parse_empty_object_yields_no_windows` replaces it. Keep `parse_valid_response` and `parse_response_null_resets_at_is_ok` (still valid: both bodies carry `five_hour`+`seven_day` objects → 2 windows).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib provider::claude::tests::parse_`
Expected: FAIL — `parse_enterprise_yields_single_spend_window` etc. fail to compile/assert (current `parse_response` returns `[LimitWindow; 2]` and crashes on null windows).

- [ ] **Step 3: Replace usage structs**

In `src/provider/claude.rs`, replace `UsageResponse` and `WindowData` (lines 141-151) with:

```rust
#[derive(Deserialize)]
struct UsageResponse {
    five_hour: Option<WindowData>,
    seven_day: Option<WindowData>,
    spend: Option<SpendData>,
}

#[derive(Deserialize)]
struct WindowData {
    utilization: f32,
    resets_at: Option<String>,
}

#[derive(Deserialize)]
struct SpendData {
    percent: f32,
    used: Money,
    limit: Money,
}

#[derive(Deserialize)]
struct Money {
    amount_minor: i64,
    exponent: u32,
    currency: String,
}

fn money_to_dollars(m: &Money) -> f64 {
    m.amount_minor as f64 / 10f64.powi(m.exponent as i32)
}
```

- [ ] **Step 4: Rewrite `parse_response` to return `Vec<LimitWindow>`**

Replace the whole `parse_response` function (`src/provider/claude.rs:153-173`) with:

```rust
fn parse_response(body: &str) -> Result<Vec<LimitWindow>, String> {
    let resp: UsageResponse = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let mut windows = Vec::new();
    if let Some(w) = resp.five_hour {
        windows.push(LimitWindow {
            name: "5h session".to_string(),
            percent_used: Some(w.utilization),
            resets_at: w.resets_at,
            ..Default::default()
        });
    }
    if let Some(w) = resp.seven_day {
        windows.push(LimitWindow {
            name: "7d weekly".to_string(),
            percent_used: Some(w.utilization),
            resets_at: w.resets_at,
            ..Default::default()
        });
    }
    if let Some(s) = resp.spend {
        windows.push(LimitWindow {
            name: "Spend".to_string(),
            percent_used: Some(s.percent),
            spent: Some(money_to_dollars(&s.used)),
            budget: Some(money_to_dollars(&s.limit)),
            currency: Some(s.used.currency),
            ..Default::default()
        });
    }
    Ok(windows)
}
```

- [ ] **Step 5: Adjust the `do_fetch` Ok arm**

`parse_response` now returns a `Vec`, so the `.to_vec()` is gone. Change `src/provider/claude.rs:238-243` from:

```rust
        Ok(body) => match parse_response(&body) {
            Ok(windows) => {
                let windows = windows.to_vec();
                *last_ok.lock().unwrap() = Some(windows.clone());
                (UsageState::Ok(windows, profile_string), None)
            }
```

to:

```rust
        Ok(body) => match parse_response(&body) {
            Ok(windows) => {
                *last_ok.lock().unwrap() = Some(windows.clone());
                (UsageState::Ok(windows, profile_string), None)
            }
```

- [ ] **Step 6: Run clippy + full test suite**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: PASS — all new parser tests green; `do_fetch_200_valid_returns_ok_and_populates_cache` still sees 2 windows; money fields read by the Task 1 view so no dead-code warning.

- [ ] **Step 7: Commit**

```bash
git add src/provider/claude.rs
git commit -m "feat(claude): parse enterprise spend window via optional usage structs"
```

---

## Task 3: Plan label from `organization_type`

Derives the plan label from `organization.organization_type` (strip `claude_` prefix → `pro`/`max`/`enterprise`), with the existing `has_claude_*` flags as fallback when the field is missing/empty/non-`claude_`. Profile is the single source for the plan label; window selection (Task 2) stays independent.

**Files:**
- Modify: `src/provider/claude.rs:119-139` (`ProfileResponse`, `plan_label`, `parse_profile_response`)
- Test: `src/provider/claude.rs` (test module)

**Interfaces:**
- Consumes: nothing from Tasks 1-2.
- Produces: `fn plan_label(org_type: Option<&str>, has_max: bool, has_pro: bool) -> String`.

- [ ] **Step 1: Write failing plan-label tests**

In the `src/provider/claude.rs` test module, add. (Fixtures anonymized per the spec: names → `User`, email → `user@example.com`, UUIDs → all-zero.)

```rust
    const PROFILE_PRO: &str = r#"{"account":{"uuid":"00000000-0000-0000-0000-000000000000","full_name":"User","display_name":"User","email":"user@example.com","has_claude_max":false,"has_claude_pro":true,"created_at":"2025-04-03T14:32:38.156445Z"},"organization":{"uuid":"00000000-0000-0000-0000-000000000000","name":"User's Organization","organization_type":"claude_pro","billing_type":"stripe_subscription","seat_tier":null},"application":{"uuid":"00000000-0000-0000-0000-000000000000","name":"Claude Code","slug":"claude-code"},"enabled_plugins":[]}"#;

    const PROFILE_ENTERPRISE: &str = r#"{"account":{"email":"user@example.com","has_claude_max":false,"has_claude_pro":false},"organization":{"organization_type":"claude_enterprise"}}"#;

    #[test]
    fn parse_profile_pro_from_organization_type() {
        let pd = super::parse_profile_response(PROFILE_PRO).unwrap();
        assert_eq!(pd.email, "user@example.com");
        assert_eq!(pd.plan, "pro");
    }

    #[test]
    fn parse_profile_enterprise_from_organization_type() {
        let pd = super::parse_profile_response(PROFILE_ENTERPRISE).unwrap();
        assert_eq!(pd.plan, "enterprise");
    }

    #[test]
    fn plan_label_strips_claude_prefix() {
        assert_eq!(super::plan_label(Some("claude_max"), false, false), "max");
        assert_eq!(super::plan_label(Some("claude_enterprise"), false, false), "enterprise");
    }

    #[test]
    fn plan_label_falls_back_when_org_type_missing_or_empty() {
        // missing -> use flags
        assert_eq!(super::plan_label(None, true, false), "max");
        assert_eq!(super::plan_label(None, false, true), "pro");
        assert_eq!(super::plan_label(None, false, false), "free");
        // empty after strip -> use flags
        assert_eq!(super::plan_label(Some("claude_"), false, true), "pro");
        // non-claude prefix -> use flags
        assert_eq!(super::plan_label(Some("team"), false, false), "free");
    }
```

Note: the existing `parse_profile_max_plan`/`parse_profile_pro_plan`/`parse_profile_free_plan` tests (`src/provider/claude.rs:548-568`) send bodies with **no** `organization` object — they exercise the fallback path and must still pass unchanged. Keep them.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib provider::claude::tests::plan_label provider::claude::tests::parse_profile`
Expected: FAIL — `plan_label` arity/return type mismatch; `organization_type` not parsed.

- [ ] **Step 3: Add the organization struct and make it optional on the profile**

In `src/provider/claude.rs`, after `ProfileAccount` (line 117), add and extend `ProfileResponse` (lines 119-122):

```rust
#[derive(Deserialize)]
struct ProfileOrganization {
    organization_type: Option<String>,
}

#[derive(Deserialize)]
struct ProfileResponse {
    account: ProfileAccount,
    organization: Option<ProfileOrganization>,
}
```

- [ ] **Step 4: Rewrite `plan_label` and `parse_profile_response`**

Replace `plan_label` (lines 129-131) and `parse_profile_response` (lines 133-139) with:

```rust
fn plan_label(org_type: Option<&str>, has_max: bool, has_pro: bool) -> String {
    if let Some(stripped) = org_type.and_then(|t| t.strip_prefix("claude_")) {
        if !stripped.is_empty() {
            return stripped.to_string();
        }
    }
    if has_max {
        "max".to_string()
    } else if has_pro {
        "pro".to_string()
    } else {
        "free".to_string()
    }
}

fn parse_profile_response(body: &str) -> Result<ProfileData, String> {
    let resp: ProfileResponse = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let org_type = resp.organization.as_ref().and_then(|o| o.organization_type.as_deref());
    let plan = plan_label(org_type, resp.account.has_claude_max, resp.account.has_claude_pro);
    Ok(ProfileData {
        email: resp.account.email,
        plan,
    })
}
```

- [ ] **Step 5: Run clippy + full test suite**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: PASS — new plan-label tests green; existing `parse_profile_*` fallback tests still green.

- [ ] **Step 6: Commit**

```bash
git add src/provider/claude.rs
git commit -m "feat(claude): derive plan label from organization_type with flag fallback"
```

---

## Self-Review

**Spec coverage:**
- §1 `LimitWindow` money fields → Task 1, Step 1. ✓
- §2 usage structs optional + money typed + minor-unit conversion → Task 2, Steps 3-4. ✓
- §3 `parse_response` single pass returning `Vec`, one window per field, no bool/license branch → Task 2, Step 4. ✓
- §4 plan label from `organization_type`, strip prefix + fallback → Task 3, Steps 3-4. ✓
- §5 view formats money (`$spent / $budget`, USD vs other), fallback to `format_reset` → Task 1, Steps 5-6. ✓
- Testing §1 plan-label mapping → Task 3 Step 1; §2 strip+fallback → Task 3 Step 1; §3 Pro/Max 2 windows, money None → Task 2 Step 1; §4 enterprise 1 Spend window + codename ignored → Task 2 Step 1; §5 money conversion → Task 2 Step 1; §6 bar selection per case → Task 2 Step 1 (counts + names + money populated/None); §7 detail-line formatting → Task 1 Step 3. ✓
- Constraints (read-only, no refresh, no panic) untouched — no code path in this plan touches auth/network flow control. ✓

**Placeholder scan:** No TBD/"handle edge cases"/"similar to" — every code step shows full code. ✓

**Type consistency:** `parse_response -> Result<Vec<LimitWindow>, String>` consistent between Task 2 definition and `do_fetch` caller. `plan_label(Option<&str>, bool, bool) -> String` consistent between Task 3 definition, callers, and tests. `format_detail`/`format_money` signatures match between Task 1 definition, view call site, and tests. `Money`/`SpendData`/`ProfileOrganization` field names match the JSON fixtures. ✓
