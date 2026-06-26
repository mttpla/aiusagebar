---
id: 58
status: doing
priority: High
tags: [claude, provider, parsing, enterprise]
spec: superpowers/specs/2026-06-26-claude-enterprise-usage-parse-design.md
plan: superpowers/plans/2026-06-27-claude-enterprise-usage-parse-plan.md
created: 2026-06-26
updated: 2026-06-27
---
# Claude enterprise usage parse — dual-shape support

Enterprise Claude accounts return a different usage JSON: `five_hour` and
`seven_day` are `null`, usage is carried in a `spend` object. The current
non-optional `WindowData` parser crashes (`invalid type: null, expected struct
WindowData`), leaving enterprise users with a persistent parse error and no
display. Make one parse path handle both Pro/Max and enterprise, and show
enterprise users a Spend bar (percent + dollars spent / limit) with the correct
plan label.

## Narrative
- 2026-06-26: Captured from brainstorming on the enterprise usage parse crash.
  Real enterprise body has `five_hour`/`seven_day` = null + a `spend` object;
  Pro/Max bodies have those as objects. Decisions:
  - **No license-conditional `if`.** Make `five_hour`/`seven_day`/`spend` all
    `Option`, single `parse_response` returning `Vec<LimitWindow>` (was
    `[LimitWindow; 2]`); push one window per present field. Mutual exclusivity
    falls out of the data shape, no bool threaded. (User explicitly rejected the
    earlier `(Vec, bool)` design.)
  - **Money source = `spend` only** (stable key: percent + used/limit
    `amount_minor`/`exponent`/`currency`). Rejected: the dollar "budget" windows
    (`cinder_cove`/`amber_ladder`) — keys are rotating codenames, fragile. Could
    revisit later via shape-detection if the small `spend` cap proves
    insufficient.
  - **`LimitWindow` carries raw money fields** `spent`/`budget`/`currency`
    (Option), not a pre-formatted string; the view (`styled.rs`) formats them in
    the detail line ("$0.00 / $50.00"), falling back to `format_reset` when
    absent. Bars/percent already handle `None`.
  - **Plan label from `organization.organization_type`** (strip `claude_`
    prefix → pro/max/enterprise), with `has_claude_*` fallback. Single source =
    profile. Rejected the earlier idea of inferring enterprise from the usage
    Spend window — user wanted it from the profile. Verified against a real Pro
    profile dump (`organization_type: "claude_pro"`); `/oauth/profile` is an
    undocumented internal endpoint, so the exact enterprise value is unconfirmed
    — the strip-prefix mapping handles any value.
  - **Tests required** with mock JSON fixtures (profile pro/max/enterprise,
    usage pro-max/enterprise) covering: plan-label mapping + fallback, window
    selection per case (which bars shown), money conversion, detail-line
    formatting. (User explicitly asked for these.)
  - Scope: single card (not split) — the three pieces (parse, plan label, view)
    are needed together for "enterprise displayed well".
- 2026-06-26: Test fixtures pinned in the spec. User supplied the real `pro`
  profile (four pasted blobs were identical — all `claude_pro`); embedded
  anonymized (names/email/UUIDs redacted) as the canonical base, with `max`,
  `enterprise`, and `fallback` derived from it. Enterprise usage fixture taken
  verbatim from the bug log. Updated `updated` to 2026-06-26.
- 2026-06-27: Moved todo → doing. Wrote implementation plan
  (`superpowers/plans/2026-06-27-claude-enterprise-usage-parse-plan.md`), 3 tasks
  (money fields + view; optional usage structs + Spend parse; plan label from
  organization_type). Key constraint baked in: each task bundles new struct
  fields with a non-test reader so per-task `clippy -D warnings` passes
  (no-allow-dead-code rule). User supplied a second, richer real enterprise body
  (many null/codename windows + nested `extra_usage`/`spend.cap`); added as
  `USAGE_ENTERPRISE_FULL` regression fixture asserting all extra keys ignored →
  single Spend window, budget $50.00 (amount_minor 5000 / exponent 2).
