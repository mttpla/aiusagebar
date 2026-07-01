# Centralize Config into settings.rs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Relocate scattered production config values into `settings.rs` — one promoted `Settings` field and four plain consts — with zero behavior change.

**Architecture:** Two tiers. Tier A = App-held `Settings` struct fields (live knobs); promote `UPDATE_CHECK_INTERVAL_HOURS` into one. Tier B = plain `settings.rs` consts consumed by `OnceLock` globals / a pure display fn (`HTTP_TIMEOUT`, `DIAG_LOG_MAX_MESSAGES`, `BAR_WARN_PCT`, `BAR_ALERT_PCT`); centralized but not yet live fields.

**Tech Stack:** Rust, `std::time::Duration`, `chrono`, `ureq`. macOS menu-bar bin crate.

## Global Constraints

- Bin crate: default to `pub(crate)` / private visibility, never bare `pub` (real dead code must surface).
- All `.rs` string literals in English.
- Never add a `Co-Authored-By: Claude` trailer to commits.
- `cargo clippy -- -D warnings && cargo test` must pass before every commit.
- Pure relocation: idle CPU ~0% and runtime behavior unchanged.
- Consts stay `pub(crate)`; consumers reference `crate::settings::<NAME>` (or `settings::<NAME>` from `main.rs`).

Spec: `docs/superpowers/specs/2026-07-01-centralize-config-settings-design.md`

---

### Task 1: Promote update-check interval to a `Settings` field

**Files:**
- Modify: `src/settings.rs:7` (const), `src/settings.rs:8-25` (struct + Default)
- Modify: `src/main.rs:161`, `src/main.rs:249-274` (read field, fix move ordering)
- Test: `src/settings.rs` `#[cfg(test)] mod tests`

**Interfaces:**
- Produces: `Settings.update_check_interval_hours: i64`; `settings::DEFAULT_UPDATE_CHECK_INTERVAL_HOURS: i64 = 24`.
- The loose const `UPDATE_CHECK_INTERVAL_HOURS` is removed; both `main.rs` read sites move to the field.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src/settings.rs`:

```rust
    #[test]
    fn default_update_check_interval_is_24_hours() {
        assert_eq!(Settings::default().update_check_interval_hours, 24);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib default_update_check_interval_is_24_hours`
Expected: FAIL — `no field 'update_check_interval_hours' on type 'Settings'` (compile error).

- [ ] **Step 3: Promote the const to a field**

In `src/settings.rs`, rename the loose const and add the struct field + default. Replace line 7:

```rust
pub(crate) const DEFAULT_UPDATE_CHECK_INTERVAL_HOURS: i64 = 24;
```

Add the field to `struct Settings` (after `backoff_cap`):

```rust
    pub(crate) update_check_interval_hours: i64,
```

Add to the `Default` impl body (after `backoff_cap: DEFAULT_BACKOFF_CAP,`):

```rust
            update_check_interval_hours: DEFAULT_UPDATE_CHECK_INTERVAL_HOURS,
```

- [ ] **Step 4: Update the two `main.rs` read sites**

`src/main.rs:161` — replace the module-path const read with the field:

```rust
            self.next_update_check_after = Local::now() + chrono::Duration::hours(self.settings.update_check_interval_hours);
```

`src/main.rs:249-272` — `settings` is moved into the struct at the `settings,` line before line 272 reads it, so capture the value into a local first. Immediately after `let settings = Settings::default();` (line 249) add:

```rust
    let update_check_interval_hours = settings.update_check_interval_hours;
```

Then replace line 272 to use the local instead of the removed const:

```rust
        next_update_check_after: Local::now() + chrono::Duration::hours(update_check_interval_hours),
```

- [ ] **Step 5: Run the full test + lint to verify pass**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: PASS — no `UPDATE_CHECK_INTERVAL_HOURS` references remain (`git grep UPDATE_CHECK_INTERVAL_HOURS` returns only `DEFAULT_UPDATE_CHECK_INTERVAL_HOURS`).

- [ ] **Step 6: Commit**

```bash
git add src/settings.rs src/main.rs
git commit -m "refactor(settings): promote update-check interval to a Settings field"
```

---

### Task 2: Add Tier B consts and rewire their consumers

**Files:**
- Modify: `src/settings.rs` (add four consts)
- Modify: `src/http.rs:21`
- Modify: `src/diag.rs:4` (remove `CAPACITY`), `:32`, `:64`, and test refs `:141`-`:154`
- Modify: `src/ui/styled.rs:126`, `:128`
- Test: existing `diag.rs` capacity tests guard behavior; no new test (pure relocation).

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: `settings::HTTP_TIMEOUT: Duration`, `settings::DIAG_LOG_MAX_MESSAGES: usize`, `settings::BAR_WARN_PCT: f32`, `settings::BAR_ALERT_PCT: f32`. All `pub(crate)`.

- [ ] **Step 1: Add the four consts to `settings.rs`**

Append after the existing `DEFAULT_*` consts block (keep the `use std::time::Duration;` at top — already present):

```rust
/// HTTP request timeout for the shared ureq agent.
pub(crate) const HTTP_TIMEOUT: Duration = Duration::from_secs(15);
/// Max messages retained in the in-memory diagnostic log ring buffer.
pub(crate) const DIAG_LOG_MAX_MESSAGES: usize = 100;
/// Progress-bar color zone boundaries (percent). Separate from the icon/notify
/// alert threshold — these drive bar fill color only.
pub(crate) const BAR_WARN_PCT: f32 = 60.0;
pub(crate) const BAR_ALERT_PCT: f32 = 80.0;
```

- [ ] **Step 2: Rewire `http.rs`**

`src/http.rs:21` — replace the inline duration:

```rust
            .timeout_global(Some(crate::settings::HTTP_TIMEOUT))
```

- [ ] **Step 3: Rewire `diag.rs`**

`src/diag.rs:4` — delete the line `const CAPACITY: usize = 100;` (keep `MAX_MSG_BYTES` on line 5 as-is).

`src/diag.rs:32` — inside `buffer()`:

```rust
    DIAG.get_or_init(|| Mutex::new(VecDeque::with_capacity(crate::settings::DIAG_LOG_MAX_MESSAGES)))
```

`src/diag.rs:64` — inside `push_entry`:

```rust
    if buf.len() == crate::settings::DIAG_LOG_MAX_MESSAGES {
```

`src/diag.rs` test refs (lines ~152, ~154 assert against `CAPACITY`; line ~141 asserts `MAX_MSG_BYTES` — leave that one). Replace the two `CAPACITY` test uses with `crate::settings::DIAG_LOG_MAX_MESSAGES`:

```rust
        assert_eq!(buf.len(), crate::settings::DIAG_LOG_MAX_MESSAGES, "buffer must cap at {}", crate::settings::DIAG_LOG_MAX_MESSAGES);
```
```rust
        assert_eq!(all.lines().count(), crate::settings::DIAG_LOG_MAX_MESSAGES);
```

- [ ] **Step 4: Rewire `styled.rs`**

`src/ui/styled.rs:126` and `:128` in `bar_fill_color`:

```rust
    if pct < crate::settings::BAR_WARN_PCT {
        srgb(0.204, 0.780, 0.349) // #34C759 green
    } else if pct <= crate::settings::BAR_ALERT_PCT {
```

- [ ] **Step 5: Verify no stray literals remain**

Run: `git grep -n 'CAPACITY' src/diag.rs` → expect no match.
Run: `git grep -nE 'from_secs\(15\)' src/http.rs` → expect no match.
Run: `git grep -nE '60\.0|80\.0' src/ui/styled.rs` → expect no match in `bar_fill_color`.

- [ ] **Step 6: Run the full test + lint to verify pass**

Run: `cargo clippy -- -D warnings && cargo test`
Expected: PASS — all existing diag capacity tests green, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/settings.rs src/http.rs src/diag.rs src/ui/styled.rs
git commit -m "refactor(settings): centralize http timeout, diag log size, and bar color thresholds"
```

---

### Task 3: Close the kanban card

**Files:**
- Modify: `docs/kanban/doing/2026-06-23-centralize-config-constants.md` → move to `docs/kanban/done/`

- [ ] **Step 1: Move card to done**

```bash
cd /Users/matteo.paoli/private/aiusagebar
git mv docs/kanban/doing/2026-06-23-centralize-config-constants.md docs/kanban/done/2026-06-23-centralize-config-constants.md
```

Edit the moved file: set `status: done` in frontmatter, bump `updated:` to the completion date, append a Narrative line noting completion (both commits, tests green).

- [ ] **Step 2: Commit**

```bash
git add docs/kanban/
git commit -m "chore(card-56): close — config centralized into settings.rs"
```

---

## Self-Review

**Spec coverage:**
- Tier A promote `update_check_interval_hours` → Task 1. ✓
- Tier B `HTTP_TIMEOUT`, `DIAG_LOG_MAX_MESSAGES` (rename), `BAR_WARN_PCT`, `BAR_ALERT_PCT` → Task 2. ✓
- Byte caps / domain identity untouched → not referenced in any task (correct — out of scope). ✓
- New default test for promoted field → Task 1 Step 1. ✓
- clippy + test gate → every task. ✓

**Placeholder scan:** No TBD/TODO; all edits show exact code and paths. ✓

**Type consistency:** `update_check_interval_hours: i64` matches `chrono::Duration::hours(i64)`. `DIAG_LOG_MAX_MESSAGES: usize` matches `VecDeque::with_capacity(usize)` and `buf.len()` comparison. `HTTP_TIMEOUT: Duration` matches `timeout_global(Some(Duration))`. `BAR_*_PCT: f32` matches `pct: f32` comparisons. ✓
