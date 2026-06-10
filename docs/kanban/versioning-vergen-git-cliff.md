---
id: 2
status: backlog
priority: Normal
tags: [versioning, release, build]
spec: superpowers/specs/2026-06-10-versioning-design.md
created: 2026-06-10
updated: 2026-06-10
---
# Versioning: vergen + git-cliff + release script

Embed git-describe version at compile time via `vergen-git2`. Dev builds show
`0.1.0-3-gabcdef`; release builds show clean semver. A bash script handles
version bump, git tag, and CHANGELOG.md generation via git-cliff.

## Narrative
- 2026-06-10: Captured from brainstorming session. Chosen approach: vergen-git2
  in build.rs (no git CLI dep), git-cliff for changelog, minimal bash script
  (~40 lines) for release. Rejected: cargo-release (overkill, no crates.io
  publish needed), Python/LLM-generated changelog (non-reproducible).
  Cargo.toml only touched at release time — never dirty during dev.
  Push remains explicit manual step after script runs.
  Spec: docs/superpowers/specs/2026-06-10-versioning-design.md
