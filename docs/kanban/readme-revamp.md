---
id: 34
status: backlog
priority: High
tags: [docs, pre-1.0]
blocked_by: [10, 11]
spec: superpowers/specs/2026-06-13-readme-revamp-design.md
created: 2026-06-13
updated: 2026-06-18
---
# README revamp for 1.0.0

Rewrite README so a non-contributor can install via DMG, complete first run without confusion, and recover from common errors. Drop all Codex/OpenAI references (not implemented). Add Install / First run / Troubleshooting / Keychain explainer / License sections. Move Rust toolchain requirement under Development. Add `LICENSE` (MIT) + `license = "MIT"` in `Cargo.toml`. Unify provider naming to **Claude** + **Copilot**. Add CLI prerequisites to provider table: `claude` CLI for Claude, `gh` CLI (with Copilot extension) or `COPILOT_GITHUB_TOKEN` for Copilot. Daily-workflow examples list only `make dev` (cargo run causes Keychain re-prompts on unsigned builds).

## Narrative
- 2026-06-13: Captured from 1.0.0 readiness review. Decisions: single card (not split); MIT license; first-run walkthrough uses screenshots (Keychain dialog, tray icon, menu open); reuse `assets/demo.png` from card #10 for menu-open screenshot to avoid duplicate file. Hard-blocked by #10 (provides `assets/demo.png`) and #11 (provides DMG artifact + Releases URL pattern). Unblocked sections can be drafted earlier (Codex removal, Configuration, Keychain explainer, Troubleshooting, Development, Releasing, License), but PR not closable until both blockers land. Rejected: CHANGELOG link in README (file already visible in tree); pre-1.0 status badge (version number suffices); Sparkle / auto-update mention (card #20, post-1.0); architecture docs (CLAUDE.md covers); CHANGELOG / Codex re-add deferred to follow-up cards. Note for implementer: replace `<owner>/<repo>` placeholder in Releases URL with the real GitHub coords.
- 2026-06-16: Partial work landed in worktree branch `worktree-readme-cleanup` (commit e853981). Changes done: removed OpenAI row from providers table, renamed Anthropic→Claude and GitHub Copilot→Copilot, added `![Demo screenshot](assets/demo.png)` placeholder (image file lands with #10). These sections don't overlap with the Installation section card #11 adds, so rebase/merge onto master after #11 lands will be conflict-free.
- 2026-06-18: **Implementer note — review release section carefully.** `release.sh` has evolved since the spec was written: it now handles the full local release flow (build, sign, `gh release create --draft`, binary upload). README releasing section may be stale or missing these steps. Audit actual `scripts/release.sh` before writing the Releasing section.
