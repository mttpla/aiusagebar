# About Window — Design Spec

**Date:** 2026-06-10
**Status:** Approved

---

## Overview

Add an "About AIUsageBar" menu item that opens a native macOS alert showing app identity, version, copyright, tagline, and GitHub repo URL, with a button that opens the author's personal website.

---

## Trigger

Menu item "About AIUsageBar" added to the tray menu, placed above the existing "Refresh" and "Quit" items.

---

## Implementation

Native macOS `NSAlert` via `objc2-app-kit` (already a transitive dependency via `tray-icon`). No custom window, no new crates.

Function `show()` in new module `src/about.rs`. Called from `src/main.rs` when the `id_about` menu event fires.

---

## Alert Content

**Title:** `AIUsageBar`

**Body (localised it/en, disclaimer always English):**

```
AIUsageBar v{version}
© {copyright_year} Matteo Paoli · MIT License
https://github.com/mttpla/aiusagebar

{tagline}

This software is provided "as is", without warranty of any kind.
The author is not liable for any damages arising from its use.
```

### Field rules

| Field | Source |
|---|---|
| `{version}` | `env!("CARGO_PKG_VERSION")` — compile-time |
| `{copyright_year}` | Runtime `chrono::Local::now().year()`: if == 2026 → `2026`; otherwise `2026 – {year}` |
| `{tagline}` | Localised string (see below) |
| GitHub URL | Hardcoded `https://github.com/mttpla/aiusagebar` |

### Tagline strings

| Locale | Text |
|---|---|
| `it` | `Monitor in sola lettura. Non invia prompt, non consuma quota, non modifica credenziali.` |
| `en` | `A read-only monitor. Never sends prompts, never spends quota, never modifies credentials.` |

Locale selection follows the project-wide OS locale rule (REQUIREMENTS §13): read system locale, fall back to `en`.

---

## Buttons

Two buttons, right-to-left macOS order:

| Button | Action |
|---|---|
| "OK" (default) | Dismiss alert |
| "matteopaoli.it" | `std::process::Command::new("open").arg("https://www.matteopaoli.it")` then dismiss |

---

## Menu changes (`src/main.rs`)

- Add `id_about: tray_icon::menu::MenuId` field to `App` and `MenuBuild`.
- Insert `MenuItem::new("About AIUsageBar", true, None)` at the top of the menu, before provider sections.
- Handle `ev.id == self.id_about` in `about_to_wait` → call `about::show()`.

---

## Module: `src/about.rs`

Single public function:

```rust
pub fn show() { ... }
```

Responsibilities:
1. Build `{version}` string from `env!("CARGO_PKG_VERSION")`.
2. Build `{copyright_year}` string at runtime via `chrono`.
3. Select `{tagline}` based on OS locale.
4. Compose full body string.
5. Display `NSAlert` with title, body, and two buttons.
6. On "matteopaoli.it" button: `open https://www.matteopaoli.it`.

No state, no dependencies beyond `chrono` (already present) and `objc2-app-kit` (transitive via `tray-icon`).

---

## Out of scope

- Clickable inline hyperlinks (NSAlert limitation)
- App icon displayed in the alert (requires bundle; dev builds use placeholder)
- "Check for updates" button (REQUIREMENTS §14: no auto-update)
