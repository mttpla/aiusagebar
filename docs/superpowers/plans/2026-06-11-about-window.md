# About Window Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add "About AIUsageBar" menu item that shows a native NSAlert with version, copyright, tagline, and a button that opens matteopaoli.it.

**Architecture:** New `src/about.rs` module with pure logic functions (testable) and a single `show()` function that calls NSAlert via `objc2-app-kit` (already in the dep tree via winit). `main.rs` adds the menu item and dispatches to `about::show()` on click.

**Tech Stack:** Rust, `objc2` 0.6, `objc2-app-kit` 0.3 (NSAlert), `objc2-foundation` 0.3 (NSString), `chrono` (already present), `std::env` for locale detection.

---

### Task 1: Pure logic + unit tests in `src/about.rs`

**Files:**
- Create: `src/about.rs`

- [ ] **Step 1: Create `src/about.rs` with pure logic functions**

```rust
use chrono::Datelike;

pub fn copyright_year_str(current_year: i32) -> String {
    if current_year == 2026 {
        "2026".to_string()
    } else {
        format!("2026–{}", current_year)
    }
}

pub fn is_italian() -> bool {
    std::env::var("LANG")
        .unwrap_or_default()
        .to_lowercase()
        .starts_with("it")
}

pub fn body_text(version: &str, copyright_year: &str, italian: bool) -> String {
    let tagline = if italian {
        "Monitor in sola lettura. Non invia prompt, non consuma quota, non modifica credenziali."
    } else {
        "A read-only monitor. Never sends prompts, never spends quota, never modifies credentials."
    };
    format!(
        "AIUsageBar {version}\n\
         \u{00a9} {copyright_year} Matteo Paoli \u{00b7} MIT License\n\
         https://github.com/mttpla/aiusagebar\n\
         \n\
         {tagline}\n\
         \n\
         This software is provided \"as is\", without warranty of any kind.\n\
         The author is not liable for any damages arising from its use."
    )
}

#[cfg(target_os = "macos")]
pub fn show() {
    // implemented in Task 2
    let _ = (copyright_year_str(0), is_italian(), body_text("", "", false));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copyright_year_start_year_is_just_2026() {
        assert_eq!(copyright_year_str(2026), "2026");
    }

    #[test]
    fn copyright_year_after_start_year_shows_range() {
        assert_eq!(copyright_year_str(2027), "2026\u{2013}2027");
    }

    #[test]
    fn body_english_contains_english_tagline() {
        let body = body_text("0.1.0", "2026", false);
        assert!(body.contains("read-only monitor"));
        assert!(!body.contains("sola lettura"));
    }

    #[test]
    fn body_italian_contains_italian_tagline() {
        let body = body_text("0.1.0", "2026", true);
        assert!(body.contains("sola lettura"));
        assert!(!body.contains("read-only monitor"));
    }

    #[test]
    fn body_contains_version_and_year() {
        let body = body_text("1.2.3", "2026\u{2013}2028", false);
        assert!(body.contains("1.2.3"));
        assert!(body.contains("2026\u{2013}2028"));
    }

    #[test]
    fn body_contains_github_url() {
        let body = body_text("0.1.0", "2026", false);
        assert!(body.contains("https://github.com/mttpla/aiusagebar"));
    }

    #[test]
    fn body_contains_disclaimer() {
        let body = body_text("0.1.0", "2026", false);
        assert!(body.contains("as is"));
        assert!(body.contains("not liable"));
    }
}
```

- [ ] **Step 2: Wire module in `src/main.rs`** (just the `mod` declaration, wiring comes in Task 3)

Add `mod about;` after `mod version;` in `src/main.rs` (line 6).

- [ ] **Step 3: Run tests to verify they pass**

```bash
cargo test about
```

Expected: 7 tests pass, 0 fail.

- [ ] **Step 4: Commit**

```bash
git add src/about.rs src/main.rs
git commit -m "feat(about): add pure logic + unit tests"
```

---

### Task 2: Add deps + implement `show()` with NSAlert

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/about.rs`

- [ ] **Step 1: Add objc2 deps to `Cargo.toml`**

In `Cargo.toml`, extend the macOS target section:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
security-framework = "3"
core-foundation = "0.9"
objc2 = "0.6"
objc2-app-kit = { version = "0.3", features = ["NSAlert"] }
objc2-foundation = "0.3"
```

- [ ] **Step 2: Verify deps resolve**

```bash
cargo check
```

Expected: compiles cleanly (no errors).

- [ ] **Step 3: Implement `show()` in `src/about.rs`**

Replace the placeholder `show()` with the real implementation. The full updated `src/about.rs` (keep all existing code, replace only the `show()` function):

```rust
#[cfg(target_os = "macos")]
pub fn show() {
    use chrono::Local;
    use objc2_app_kit::{NSAlert, NSAlertSecondButtonReturn};
    use objc2_foundation::NSString;

    let version = crate::version::app_version();
    let year_str = copyright_year_str(Local::now().year());
    let body = body_text(&version, &year_str, is_italian());

    unsafe {
        let alert = NSAlert::new();
        alert.setMessageText(&NSString::from_str("AIUsageBar"));
        alert.setInformativeText(&NSString::from_str(&body));
        alert.addButtonWithTitle(&NSString::from_str("OK"));
        alert.addButtonWithTitle(&NSString::from_str("matteopaoli.it"));
        let response = alert.runModal();
        if response == NSAlertSecondButtonReturn {
            let _ = std::process::Command::new("open")
                .arg("https://www.matteopaoli.it")
                .spawn();
        }
    }
}
```

> **Note:** `NSAlert::new()` calls `[[NSAlert alloc] init]`. `runModal()` blocks until the user dismisses. First `addButtonWithTitle` call = default (return key) button = response 1000; second = response 1001 (`NSAlertSecondButtonReturn`). This must be called from the main thread — which `about_to_wait` guarantees.

- [ ] **Step 4: Verify it compiles**

```bash
cargo check
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/about.rs
git commit -m "feat(about): implement NSAlert show() via objc2-app-kit"
```

---

### Task 3: Wire About into `src/main.rs`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add `id_about` to `MenuBuild`**

In `struct MenuBuild` (line 27–31), add field:

```rust
struct MenuBuild {
    menu: Menu,
    about: tray_icon::menu::MenuId,
    refresh: tray_icon::menu::MenuId,
    quit: tray_icon::menu::MenuId,
}
```

- [ ] **Step 2: Add `id_about` to `App`**

In `struct App` (line 38–47), add field:

```rust
struct App {
    tray: tray_icon::TrayIcon,
    icons: Icons,
    id_about: tray_icon::menu::MenuId,
    id_quit: tray_icon::menu::MenuId,
    id_refresh: tray_icon::menu::MenuId,
    providers: Vec<Box<dyn UsageProvider>>,
    last_refreshed_at: Option<DateTime<Local>>,
    settings: Settings,
    next_poll_at: Instant,
}
```

- [ ] **Step 3: Add About menu item in `build_menu`**

In `App::build_menu`, insert the About item at the top, before the `for (name, state)` loop (line 52). Also update the `MenuBuild` return value:

```rust
fn build_menu(states: &[(&str, &UsageState)], last_updated: Option<&str>) -> MenuBuild {
    let menu = Menu::new();

    let item_about = MenuItem::new("About AIUsageBar", true, None);
    menu.append(&item_about).expect("menu append failed");
    menu.append(&PredefinedMenuItem::separator())
        .expect("menu append failed");

    for (name, state) in states {
        // ... existing code unchanged ...
    }
    // ... rest of existing code unchanged ...

    let item_refresh = MenuItem::new("Refresh", true, None);
    let item_quit = MenuItem::new("Quit", true, None);
    menu.append(&item_refresh).expect("menu append failed");
    menu.append(&item_quit).expect("menu append failed");
    MenuBuild {
        about: item_about.id().clone(),
        refresh: item_refresh.id().clone(),
        quit: item_quit.id().clone(),
        menu,
    }
}
```

- [ ] **Step 4: Update `App::refresh` to track `id_about`**

In `App::refresh` (lines 96–115), after `let build = Self::build_menu(...)`, add:

```rust
self.id_about = build.about;
self.id_refresh = build.refresh;
self.id_quit = build.quit;
```

(replace the two existing `self.id_*` assignments)

- [ ] **Step 5: Handle About event in `about_to_wait`**

In `about_to_wait`, in the `MenuEvent` block (lines 134–141), add handling for `id_about`:

```rust
if let Ok(ev) = MenuEvent::receiver().try_recv() {
    if ev.id == self.id_quit {
        event_loop.exit();
    } else if ev.id == self.id_about {
        about::show();
    } else if ev.id == self.id_refresh && !did_refresh {
        self.refresh();
        self.next_poll_at = Instant::now() + self.settings.poll_interval;
    }
}
```

- [ ] **Step 6: Update `App` initialization in `main()`**

In `main()`, the `App { ... }` struct literal (lines 184–193) needs `id_about`:

```rust
let mut app = App {
    tray,
    icons,
    id_about: build.about,
    id_quit: build.quit,
    id_refresh: build.refresh,
    providers,
    last_refreshed_at: None,
    settings,
    next_poll_at,
};
```

- [ ] **Step 7: Build and verify**

```bash
cargo build
```

Expected: compiles cleanly, no warnings.

- [ ] **Step 8: Smoke test**

```bash
make dev
```

Expected: app launches, tray menu shows "About AIUsageBar" at the top (above separator), clicking it shows NSAlert with correct content, "matteopaoli.it" button opens the browser, "OK" dismisses.

- [ ] **Step 9: Commit**

```bash
git add src/main.rs
git commit -m "feat(about): wire About menu item and event handler"
```

---

### Task 4: Move kanban card to done

**Files:**
- Modify: `docs/kanban/about-window.md`

- [ ] **Step 1: Update card status and narrative**

In `docs/kanban/about-window.md`, set `status: done`, `updated: 2026-06-11`, link plan, and append narrative:

```yaml
status: done
updated: 2026-06-11
plan: plans/2026-06-11-about-window.md
```

Append to `## Narrative`:
```
- 2026-06-11: Implemented. NSAlert via objc2-app-kit 0.3 (objc2 0.6). Pure logic
  in copyright_year_str/body_text tested with 7 unit tests. show() blocks on main
  thread (safe from about_to_wait). Locale via $LANG env var.
```

- [ ] **Step 2: Commit**

```bash
git add docs/kanban/about-window.md docs/superpowers/plans/2026-06-11-about-window.md
git commit -m "kanban: close card #3 — about window complete"
```
