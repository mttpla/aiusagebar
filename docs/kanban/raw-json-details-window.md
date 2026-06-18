---
id: 45
status: todo
priority: Normal
tags: [ui, debug, providers]
spec: specs/2026-06-18-raw-json-details-window.md
created: 2026-06-18
updated: 2026-06-18
---
# Raw JSON details window

Add a "Details…" menu item to each provider section (Claude, Copilot). Clicking opens a macOS window (NSAlert + NSScrollView + NSTextView) showing the last raw HTTP response body from that provider's API — full fidelity, including error bodies.

## Narrative
- 2026-06-18: Captured from brainstorming. Goal: see all API response details without
  parsing loss, useful for debugging and curiosity.
  Key decisions:
  - Raw JSON cached in each provider via `Mutex<Option<String>>`, exposed via new
    `raw_json()` trait method. Not embedded in UsageState (debug data ≠ state).
  - `http::get` signature changed to return `(Result<String, HttpError>, Option<String>)`
    so error response bodies (4xx) are also captured and stored.
  - Copilot multi-account: responses concatenated into one string with `--- @account ---`
    separators. One "Details…" item per provider, not per account.
  - UI: NSAlert window (same pattern as About), not a submenu — avoids index
    complexity and gives room for long JSON. NSScrollView 600×300, monospace NSTextView.
  - Content: serde_json pretty-print if valid JSON, raw string as-is otherwise.
    "No data yet" if never fetched.
  - Rejected: embedding raw JSON in UsageState (wrong semantics), per-account
    submenus (too many indices to track), history of responses (only last needed).
