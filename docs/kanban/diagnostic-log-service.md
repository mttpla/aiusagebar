---
id: 44
title: Diagnostic log service + "Other" menu
status: backlog
priority: Normal
created: 2026-06-17
updated: 2026-06-17
spec: specs/2026-06-17-diagnostic-log-design.md
---
# Diagnostic log service + "Other" menu

Central in-memory FIFO log (100 entries, 2KB/entry cap, no disk). Any module calls
`diag!(Level::Err, "...")`. Exposed in menu via `Other ▶ Diagnostics ▶ Copy diagnostic
log` (hidden when buffer empty). User copies to clipboard and pastes in TextEdit/mail.

## Narrative

- 2026-06-17: Designed after debugging the `resets_at: null` parse error (card #43).
  Root need: errors in the orange row give zero context; no way to report or reproduce.
  Decided on zero-dep `VecDeque` approach — `log`/`tracing` crates add nothing for
  user-facing diagnostic copy. In-menu preview rejected (entries too large, not
  scrollable). Disk writes rejected (overkill). "Other" submenu chosen as future home
  for Settings and other secondary items. Marked Normal priority but target pre-1.0
  — makes error reporting actionable for end users before public release.
  Hook points v1: parse error + last-ok snapshot + HTTP errors + token load failure.
  Spec: docs/superpowers/specs/2026-06-17-diagnostic-log-design.md.
