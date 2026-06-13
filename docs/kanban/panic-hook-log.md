---
id: 28
status: backlog
priority: Normal
tags: [robustness, logging, post-1.0]
created: 2026-06-13
updated: 2026-06-13
---
# Panic hook → `~/Library/Logs/AiUsageBar/panic.log`

`std::panic::set_hook` writes panic message + backtrace to a log file under `~/Library/Logs/AiUsageBar/`. On next launch, if file exists newer than last clean shutdown marker, surface a menu row `⚠ Previous session crashed — view log`.

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Essential for diagnosing field crashes once distributed. Use `RUST_BACKTRACE=1` env or `backtrace` crate. Post-1.0 because no installed base yet to crash; ship with 1.1 alongside auto-update (#20).
