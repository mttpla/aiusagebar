# Claude Reset Time — Local OS Timezone Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert Claude's raw UTC `resets_at` string to the OS local timezone before rendering in the tray menu.

**Architecture:** Add a pure helper `format_reset_local(iso_utc, now)` in `src/ui/claude.rs`; wire it into `append_claude_section` replacing the raw `resets_at` passthrough. `LimitWindow.resets_at` stays as the raw ISO 8601 string — UI owns presentation. `now` is injected for deterministic tests.

**Tech Stack:** `chrono` (already in `Cargo.toml`) — `DateTime<Local>`, `parse_from_rfc3339`, `date_naive`, `format`.

---

### Task 1: Write failing tests for `format_reset_local`

**Files:**
- Modify: `src/ui/claude.rs` — append to `#[cfg(test)]` block

- [ ] **Step 1: Add the failing tests to the test module**

  Append inside the existing `#[cfg(test)] mod tests { ... }` block in `src/ui/claude.rs`, after the last `}` of the last existing test and before the closing `}` of the module:

  ```rust
      // ---- format_reset_local ----

      fn now_local_from_utc(rfc3339: &str) -> chrono::DateTime<chrono::Local> {
          chrono::DateTime::parse_from_rfc3339(rfc3339)
              .unwrap()
              .with_timezone(&chrono::Local)
      }

      #[test]
      fn reset_same_day_returns_hhmm() {
          // now == input instant → guaranteed same local date
          let now = now_local_from_utc("2026-06-13T12:30:00Z");
          let result = format_reset_local("2026-06-13T12:30:00Z", now);
          // shape: HH:MM (exactly 5 chars, no date part)
          assert!(
              chrono::NaiveTime::parse_from_str(&result, "%H:%M").is_ok(),
              "expected HH:MM, got '{}'",
              result
          );
      }

      #[test]
      fn reset_different_day_returns_datetime() {
          // now is 30 days before input → guaranteed different local date
          let now = now_local_from_utc("2026-05-14T12:30:00Z");
          let result = format_reset_local("2026-06-13T12:30:00Z", now);
          // shape: YYYY-MM-DD HH:MM (exactly 16 chars)
          assert!(
              chrono::NaiveDateTime::parse_from_str(&result, "%Y-%m-%d %H:%M").is_ok(),
              "expected YYYY-MM-DD HH:MM, got '{}'",
              result
          );
      }

      #[test]
      fn reset_midnight_cross_valid_shape() {
          // UTC 23:30 on June 13 → local date/time is TZ-dependent
          // assert output is one of the two valid formats (TZ-agnostic)
          let now = now_local_from_utc("2026-06-13T10:00:00Z");
          let result = format_reset_local("2026-06-13T23:30:00Z", now);
          let valid = chrono::NaiveTime::parse_from_str(&result, "%H:%M").is_ok()
              || chrono::NaiveDateTime::parse_from_str(&result, "%Y-%m-%d %H:%M").is_ok();
          assert!(valid, "unexpected format: '{}'", result);
      }

      #[test]
      fn reset_malformed_passthrough() {
          let now = now_local_from_utc("2026-06-13T10:00:00Z");
          assert_eq!(format_reset_local("not-a-date", now), "not-a-date");
      }
  ```

- [ ] **Step 2: Verify the tests fail to compile (function not yet defined)**

  ```bash
  cargo test -p aiusagebar 2>&1 | head -20
  ```

  Expected: compile error `cannot find function 'format_reset_local'`.

---

### Task 2: Implement `format_reset_local` and wire it up

**Files:**
- Modify: `src/ui/claude.rs` — add import + helper fn + call site

- [ ] **Step 3: Add the chrono import at the top of the file**

  Current top of `src/ui/claude.rs`:
  ```rust
  use tray_icon::menu::Menu;
  use crate::provider::{ProviderKind, UsageState};
  ```

  Replace with:
  ```rust
  use chrono::{DateTime, Local};
  use tray_icon::menu::Menu;
  use crate::provider::{ProviderKind, UsageState};
  ```

- [ ] **Step 4: Add the helper function after `pct_label`**

  After the closing `}` of `pct_label` (currently at line 17), insert:

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

- [ ] **Step 5: Wire the helper into `append_claude_section`**

  Current line 33 in `append_claude_section`:
  ```rust
              let reset = w.resets_at.as_deref().unwrap_or("?");
  ```

  Replace with:
  ```rust
              let reset = w
                  .resets_at
                  .as_deref()
                  .map(|s| format_reset_local(s, Local::now()))
                  .unwrap_or_else(|| "?".to_string());
  ```

- [ ] **Step 6: Run all tests and verify they pass**

  ```bash
  cargo test 2>&1
  ```

  Expected: all tests pass, zero warnings from new code.

- [ ] **Step 7: Confirm it compiles clean**

  ```bash
  cargo clippy 2>&1
  ```

  Expected: no warnings or errors.

---

### Task 3: Commit

**Files:** `src/ui/claude.rs`

- [ ] **Step 8: Commit**

  ```bash
  git add src/ui/claude.rs
  git commit -m "feat(ui): render Claude reset time in local OS timezone"
  ```

---

## Self-Review

**Spec coverage:**

| Spec requirement | Task |
|---|---|
| Convert `resets_at` UTC → OS local TZ | Task 2 step 4–5 |
| Same-day → `HH:MM`, other → `YYYY-MM-DD HH:MM` | Task 2 step 4 |
| Malformed input → passthrough | Task 2 step 4 |
| `now` injected for deterministic tests | Task 1 + Task 2 |
| Same-day test | Task 1 step 1 `reset_same_day_returns_hhmm` |
| Different-day test | Task 1 step 1 `reset_different_day_returns_datetime` |
| Midnight-crossing test | Task 1 step 1 `reset_midnight_cross_valid_shape` |
| Malformed test | Task 1 step 1 `reset_malformed_passthrough` |
| No new crates | `chrono` already in `Cargo.toml` |
| TZ-agnostic CI (regex-shape approach) | `NaiveTime`/`NaiveDateTime::parse_from_str` checks |

**Placeholder scan:** None found.

**Type consistency:** `format_reset_local` signature is `(iso_utc: &str, now: DateTime<Local>) -> String` — consistent across definition (Task 2 step 4), call site (Task 2 step 5), and tests (Task 1 step 1). `now_local_from_utc` helper returns `DateTime<Local>`, matches the parameter type.
