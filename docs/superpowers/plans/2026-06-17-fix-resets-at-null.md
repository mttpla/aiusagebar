# Fix resets_at Nullable Crash Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix serde crash when Anthropic returns `"resets_at": null` by typing `WindowData.resets_at` as `Option<String>`.

**Architecture:** Single struct field change in `WindowData` + remove the now-redundant `Some(…)` wrap in `parse_response`. No other types change.

**Tech Stack:** Rust, serde_json.

## Global Constraints

- Only `src/provider/claude.rs` is modified.
- `LimitWindow`, `UsageState`, and all other types stay untouched.
- Run `cargo clippy -- -D warnings && cargo test` before committing.

---

### Task 1: Fix WindowData.resets_at and parse_response

**Files:**
- Modify: `src/provider/claude.rs:133-135` (struct field)
- Modify: `src/provider/claude.rs:138-160` (parse_response body)
- Test: `src/provider/claude.rs` (existing `#[cfg(test)]` block, ~line 302)

**Interfaces:**
- Consumes: `WindowData` (private struct), `parse_response` (private fn returning `Result<[LimitWindow; 2], String>`)
- Produces: same public API — no caller changes required

- [ ] **Step 1: Write the failing test**

Add inside the existing `#[cfg(test)]` `mod test` block in `src/provider/claude.rs` (after `parse_missing_field_is_error`, ~line 319):

```rust
#[test]
fn parse_response_null_resets_at_is_ok() {
    let body = r#"{"five_hour":{"utilization":10.0,"resets_at":null},"seven_day":{"utilization":5.0,"resets_at":null}}"#;
    let windows = super::parse_response(body).unwrap();
    assert_eq!(windows[0].resets_at, None);
    assert_eq!(windows[1].resets_at, None);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test parse_response_null_resets_at_is_ok 2>&1 | tail -20
```

Expected: FAIL — `invalid type: null, expected a string`

- [ ] **Step 3: Fix the struct field**

In `src/provider/claude.rs`, change `WindowData`:

```rust
// before
struct WindowData {
    utilization: f32,
    resets_at: String,
}

// after
struct WindowData {
    utilization: f32,
    resets_at: Option<String>,
}
```

- [ ] **Step 4: Fix parse_response assignments**

`LimitWindow.resets_at` is already `Option<String>`, so drop the `Some(…)` wrap:

```rust
// before
resets_at: Some(resp.five_hour.resets_at),
// ...
resets_at: Some(resp.seven_day.resets_at),

// after
resets_at: resp.five_hour.resets_at,
// ...
resets_at: resp.seven_day.resets_at,
```

- [ ] **Step 5: Run clippy + all tests**

```bash
cargo clippy -- -D warnings && cargo test
```

Expected: 0 warnings, all tests pass (including `parse_valid_response` and the new `parse_response_null_resets_at_is_ok`).

- [ ] **Step 6: Commit**

```bash
git add src/provider/claude.rs
git commit -m "fix(claude): accept null resets_at from API"
```
