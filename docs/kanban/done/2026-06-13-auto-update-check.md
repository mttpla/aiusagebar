---
id: 20
status: done
priority: Normal
tags: [release, ux, post-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Auto-update check via GitHub Releases

Daily poll `api.github.com/repos/<owner>/<repo>/releases/latest`, compare `tag_name` to `env!("CARGO_PKG_VERSION")`. On newer, show `↑ Update available 0.2.0` row that opens the release page. Cheap, no Sparkle framework.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Sparkle considered — rejected: heavy ObjC dep, signing complexity, overkill until installed base justifies it. GitHub anonymous endpoint is rate-limited 60/h per IP, plenty for daily check. Post-1.0 because 1.0 is the first signed release; no upgrade path needed yet.
