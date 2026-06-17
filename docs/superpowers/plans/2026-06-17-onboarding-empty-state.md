# Onboarding Empty State Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the silent "not configured" disabled menu label with a clickable "not signed in · Setup…" row per provider that opens a dedicated setup page.

**Architecture:** The per-provider UI modules (`ui/claude.rs`, `ui/copilot.rs`) return an `Option<MenuId>` for the setup row when `NotConfigured`. `MenuBuild` exposes those ids; `main.rs` routes clicks to `open <url>`.

**Tech Stack:** Rust, `tray-icon` (MenuItem/MenuId), `std::process::Command::new("open")`.

## Global Constraints

- All string literals in `.rs` files must be English.
- `cargo clippy -- -D warnings && cargo test` must pass before every commit.
- Never add `#[allow(dead_code)]`.
- `section_item_count` stays 1 for `NotConfigured` — layout indices must not shift.
- No new ObjC2 code.
- URLs: `https://github.com/mttpla/aiusagebar/blob/master/claude-setup.md` and `https://github.com/mttpla/aiusagebar/blob/master/copilot-setup.md`.

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `src/ui/claude.rs` | Modify | Label text, enabled MenuItem, `Option<MenuId>` return |
| `src/ui/copilot.rs` | Modify | Same as above for Copilot |
| `src/ui/mod.rs` | Modify | `MenuBuild` new fields, propagate setup ids |
| `src/main.rs` | Modify | URL constants, App fields, event handler branches |
| `claude-setup.md` | Create | Stub Claude setup page |
| `copilot-setup.md` | Create | Stub Copilot setup page |

---

### Task 1: Update `src/ui/claude.rs` — label + clickable MenuItem

**Files:**
- Modify: `src/ui/claude.rs`

**Interfaces:**
- Produces: `append_claude_section(menu: &Menu, state: &UsageState) -> Option<MenuId>`
  — `Some(id)` when `NotConfigured`, `None` otherwise.
  Calling code in `mod.rs` currently discards the return with `let _`, so it still
  compiles before Task 3 touches `mod.rs`.

- [ ] **Step 1: Update the failing test**

In `src/ui/claude.rs`, change the `header_not_configured` test:

```rust
#[test]
fn header_not_configured() {
    assert_eq!(
        header_label("Claude", &UsageState::NotConfigured),
        "Claude — not signed in · Setup…"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -q ui::claude::tests::header_not_configured 2>&1
```

Expected: FAIL — `"Claude: not configured"` ≠ `"Claude — not signed in · Setup…"`

- [ ] **Step 3: Update `header_label` in `src/ui/claude.rs`**

Change the `NotConfigured` arm (line 11):

```rust
UsageState::NotConfigured => format!("{} — not signed in · Setup…", name),
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -q ui::claude::tests::header_not_configured 2>&1
```

Expected: PASS

- [ ] **Step 5: Update import and rewrite `append_claude_section`**

Change the import at the top of `src/ui/claude.rs` (line 2):

```rust
use tray_icon::menu::{Menu, MenuId, MenuItem};
```

Replace `append_claude_section` (starting at line 29) with:

```rust
pub(crate) fn append_claude_section(menu: &Menu, state: &UsageState) -> Option<MenuId> {
    if let UsageState::NotConfigured = state {
        let item = MenuItem::new(
            header_label(ProviderKind::Claude.display_name(), state),
            true,
            None,
        );
        let id = item.id().clone();
        menu.append(&item).expect("menu append failed");
        return Some(id);
    }
    super::append_label(menu, header_label(ProviderKind::Claude.display_name(), state));
    if let UsageState::Ok(windows, _) = state {
        let now = Local::now();
        for w in windows {
            let reset = w
                .resets_at
                .as_deref()
                .map(|s| super::time::format_reset_local(s, now))
                .unwrap_or_else(|| "?".to_string());
            super::append_label(
                menu,
                format!("  {} — {}  resets {}", w.name, pct_label(w.percent_used), reset),
            );
        }
    }
    None
}
```

Note: `section_item_count` is a separate pure function and is **not touched**.

- [ ] **Step 6: Run clippy and all tests**

```bash
cargo clippy -- -D warnings && cargo test 2>&1
```

Expected: all tests PASS, no clippy warnings.

- [ ] **Step 7: Commit**

```bash
git add src/ui/claude.rs
git commit -m "feat(ui): clickable setup row for Claude NotConfigured state"
```

---

### Task 2: Update `src/ui/copilot.rs` — label + clickable MenuItem

**Files:**
- Modify: `src/ui/copilot.rs`

**Interfaces:**
- Produces: `append_copilot_section(menu: &Menu, state: &UsageState) -> Option<MenuId>`
  — `Some(id)` when `NotConfigured`, `None` otherwise.

- [ ] **Step 1: Update the failing test**

In `src/ui/copilot.rs`, find the `header_not_configured` test (it currently expects
`"GitHub Copilot: not configured"` — check the exact string by reading the file).
Update it to:

```rust
#[test]
fn header_not_configured() {
    let state = UsageState::NotConfigured;
    assert_eq!(
        header_label("GitHub Copilot", &state),
        "GitHub Copilot — not signed in · Setup…"
    );
}
```

If this test doesn't exist yet, add it inside the `#[cfg(test)] mod tests` block.

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -q ui::copilot::tests::header_not_configured 2>&1
```

Expected: FAIL

- [ ] **Step 3: Update `header_label` in `src/ui/copilot.rs`**

Change the `NotConfigured` arm (line 11):

```rust
UsageState::NotConfigured => format!("{} — not signed in · Setup…", name),
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -q ui::copilot::tests::header_not_configured 2>&1
```

Expected: PASS

- [ ] **Step 5: Update import and rewrite `append_copilot_section`**

Change the import at the top of `src/ui/copilot.rs` (line 2):

```rust
use tray_icon::menu::{Menu, MenuId, MenuItem};
```

Replace `append_copilot_section` (starting at line 37) with:

```rust
pub(crate) fn append_copilot_section(menu: &Menu, state: &UsageState) -> Option<MenuId> {
    if let UsageState::NotConfigured = state {
        let item = MenuItem::new(
            header_label(ProviderKind::Copilot.display_name(), state),
            true,
            None,
        );
        let id = item.id().clone();
        menu.append(&item).expect("menu append failed");
        return Some(id);
    }
    super::append_label(menu, header_label(ProviderKind::Copilot.display_name(), state));
    if let UsageState::Ok(windows, _) = state {
        let now = Local::now();
        for w in windows {
            super::append_label(menu, row_label(w, now));
        }
    }
    None
}
```

- [ ] **Step 6: Run clippy and all tests**

```bash
cargo clippy -- -D warnings && cargo test 2>&1
```

Expected: all tests PASS, no clippy warnings.

- [ ] **Step 7: Commit**

```bash
git add src/ui/copilot.rs
git commit -m "feat(ui): clickable setup row for Copilot NotConfigured state"
```

---

### Task 3: Expose setup ids in `src/ui/mod.rs`

**Files:**
- Modify: `src/ui/mod.rs`

**Interfaces:**
- Consumes: `claude::append_claude_section -> Option<MenuId>` (Task 1), `copilot::append_copilot_section -> Option<MenuId>` (Task 2)
- Produces: `MenuBuild { setup_claude: Option<MenuId>, setup_copilot: Option<MenuId>, … }`

- [ ] **Step 1: Add fields to `MenuBuild`**

In `src/ui/mod.rs`, update the `MenuBuild` struct (around line 12):

```rust
pub struct MenuBuild {
    pub menu: Menu,
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
    pub update: Option<MenuId>,
    pub setup_claude: Option<MenuId>,
    pub setup_copilot: Option<MenuId>,
}
```

- [ ] **Step 2: Propagate setup ids in `build_menu`**

In `build_menu`, replace the provider loop (around line 85):

```rust
let mut setup_claude: Option<MenuId> = None;
let mut setup_copilot: Option<MenuId> = None;
for (kind, state) in states {
    match kind {
        ProviderKind::Claude => {
            setup_claude = claude::append_claude_section(&menu, state);
        }
        ProviderKind::Copilot => {
            setup_copilot = copilot::append_copilot_section(&menu, state);
        }
    }
}
```

Update the `MenuBuild` return value at the end of `build_menu`:

```rust
MenuBuild {
    menu,
    about: footer.about,
    refresh: footer.refresh,
    quit: footer.quit,
    update: update_id,
    setup_claude,
    setup_copilot,
}
```

- [ ] **Step 3: Run clippy and all tests**

```bash
cargo clippy -- -D warnings && cargo test 2>&1
```

Expected: all tests PASS, no clippy warnings.
Note: existing `build_menu` tests in `mod.rs` do not construct `MenuBuild` directly,
so no test edits are needed here. Compilation is the coverage for the new fields.

- [ ] **Step 4: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat(ui): expose setup MenuIds in MenuBuild"
```

---

### Task 4: Wire event handling in `src/main.rs` + create setup pages

**Files:**
- Modify: `src/main.rs`
- Create: `claude-setup.md`
- Create: `copilot-setup.md`

**Interfaces:**
- Consumes: `MenuBuild { setup_claude, setup_copilot }` (Task 3)

- [ ] **Step 1: Add URL constants**

Add two constants near the top of `src/main.rs`, after the `use` blocks and before
`struct App`:

```rust
const CLAUDE_SETUP_URL: &str =
    "https://github.com/mttpla/aiusagebar/blob/master/claude-setup.md";
const COPILOT_SETUP_URL: &str =
    "https://github.com/mttpla/aiusagebar/blob/master/copilot-setup.md";
```

- [ ] **Step 2: Add setup id fields to `App`**

In the `App` struct (around line 30), add after `id_update`:

```rust
id_setup_claude: Option<tray_icon::menu::MenuId>,
id_setup_copilot: Option<tray_icon::menu::MenuId>,
```

- [ ] **Step 3: Update `App::refresh` to capture setup ids**

In `App::refresh` (around line 60), after `self.id_update = build.update;` add:

```rust
self.id_setup_claude = build.setup_claude;
self.id_setup_copilot = build.setup_copilot;
```

- [ ] **Step 4: Add event handler branches**

In `about_to_wait`, after the `id_update` branch (around line 103):

```rust
} else if self.id_setup_claude.as_ref().is_some_and(|id| ev.id == *id) {
    let _ = std::process::Command::new("open").arg(CLAUDE_SETUP_URL).spawn();
} else if self.id_setup_copilot.as_ref().is_some_and(|id| ev.id == *id) {
    let _ = std::process::Command::new("open").arg(COPILOT_SETUP_URL).spawn();
}
```

- [ ] **Step 5: Initialise new fields in `main()`**

In `main()`, the `App { … }` literal (around line 151) must include the two new
fields. After `id_update: build.update,` add:

```rust
id_setup_claude: build.setup_claude,
id_setup_copilot: build.setup_copilot,
```

- [ ] **Step 6: Run clippy and all tests**

```bash
cargo clippy -- -D warnings && cargo test 2>&1
```

Expected: all tests PASS, no clippy warnings.

- [ ] **Step 7: Create stub setup pages at repo root**

Create `claude-setup.md`:

```markdown
# Claude Setup

AIUsageBar reads your Claude session token to display live usage data in the menu bar.

## How to sign in

1. Open [claude.ai](https://claude.ai) in your browser and sign in to your account.
2. Open the Claude desktop app (macOS) and sign in there too.
   AIUsageBar reads the token that the Claude app stores in your macOS Keychain.
3. On first launch, macOS will show a dialog asking whether to allow AIUsageBar to
   read the Keychain item — click **Always Allow**.

## Troubleshooting

- **Still shows "not signed in" after signing in:** click ↺ Refresh in the menu bar.
- **Keychain dialog never appeared:** open Keychain Access, search for
  `Claude Code-credentials`, and grant access manually.
- **Usage shows "account unavailable":** your Claude plan may not expose usage data
  via the API. Max plan subscribers see full data.
```

Create `copilot-setup.md`:

```markdown
# GitHub Copilot Setup

AIUsageBar reads your Copilot token to display live seat / premium-request usage.

## Token priority

AIUsageBar checks these sources in order and uses the first one found:

1. `COPILOT_GITHUB_TOKEN` environment variable (fine-grained PAT — recommended)
2. `GH_TOKEN` environment variable
3. `GITHUB_TOKEN` environment variable
4. macOS Keychain item `copilot-cli`
5. `~/.copilot/config.json`
6. `~/.config/gh/hosts.yml` (set by `gh auth login`)

## Recommended: fine-grained PAT

1. Go to **GitHub → Settings → Developer settings → Personal access tokens →
   Fine-grained tokens**.
2. Create a token with **read-only** access to your account's Copilot usage.
3. Export it in your shell profile:
   ```sh
   export COPILOT_GITHUB_TOKEN="github_pat_..."
   ```
4. Restart AIUsageBar (or click ↺ Refresh).

## Troubleshooting

- **Still shows "not signed in":** run `gh auth status` to confirm `gh` has a valid
  token, then restart AIUsageBar.
- **Usage data is empty:** your Copilot plan may not expose usage metrics via the API.
  Business and Enterprise plans have full coverage; Individual plans may have limited
  data.
```

- [ ] **Step 8: Run final check**

```bash
cargo clippy -- -D warnings && cargo test 2>&1
```

Expected: all tests PASS, no clippy warnings.

- [ ] **Step 9: Manual smoke test**

```bash
make dev
```

With no Claude / Copilot token configured, the menu should show:
- `Claude — not signed in · Setup…` (clickable, bold, brand-coloured)
- `GitHub Copilot — not signed in · Setup…` (clickable, bold, brand-coloured)

Click each row → browser opens the respective setup page on GitHub.

- [ ] **Step 10: Commit**

```bash
git add src/main.rs claude-setup.md copilot-setup.md
git commit -m "feat: wire setup-page click handler + add provider setup docs"
```

---

## Self-Review

**Spec coverage:**
- ✅ Header label changes to "not signed in · Setup…" — Task 1 & 2
- ✅ Header is clickable (enabled MenuItem) — Task 1 & 2
- ✅ Click opens dedicated page — Task 4
- ✅ `section_item_count` unchanged at 1 — not touched in any task
- ✅ No new ObjC2 — confirmed, zero ObjC2 changes
- ✅ Separate pages (`claude-setup.md`, `copilot-setup.md`) — Task 4 Step 7
- ✅ README untouched — not in file map
- ✅ `build_layout` unchanged — not touched

**Placeholder scan:** no TBD, no TODO, all code blocks complete.

**Type consistency:**
- `append_claude_section` → `Option<MenuId>` defined Task 1, consumed Task 3 ✅
- `append_copilot_section` → `Option<MenuId>` defined Task 2, consumed Task 3 ✅
- `MenuBuild.setup_claude / setup_copilot` defined Task 3, consumed Task 4 ✅
