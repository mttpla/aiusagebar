---
id: 57
status: done
priority: Normal
tags: [kanban, tooling, docs]
spec: superpowers/specs/2026-06-23-kanban-folder-restructure-design.md
plan: superpowers/plans/2026-06-23-kanban-folder-restructure.md
created: 2026-06-23
updated: 2026-06-23
---
# Restructure kanban into status subfolders with date-prefixed filenames

The flat `docs/kanban/` directory holds 56 cards and is no longer human-scannable:
`ls` is alphabetical, status is invisible without opening each file, and live cards
drown among completed ones. Move to status subfolders with `created`-date-prefixed
filenames, and rewrite the kanban skill to match.

## Scope
- **Layout:** `docs/kanban/<status>/<created>-<slug>.md`. Five status dirs
  (`backlog/todo/doing/done/archive`). Status encoded by parent folder.
- **Filename:** `<created>-<slug>.md`. `created` is immutable → filename never
  changes; gives chronological birth-order sort via plain `ls`. `updated` stays in
  frontmatter only (mutable date in filename = rename churn).
- **Frontmatter:** schema unchanged. `status:` kept; new invariant `status` value
  == parent folder name. `id` kept.
- **Skill rewrite** (`~/.claude/skills/kanban-skill/SKILL.md`, edited in place):
  filename rule, create-into-folder, move = `git mv` + update status/updated/Narrative,
  board/query walk the 5 dirs, id scan walks subfolders, add folder invariant.
- **Migration:** one-shot script `git mv`s all 56 cards into the correct folder
  renamed `<created>-<slug>.md`. No content edits. Verify: 56 in == 56 out, each
  file's status == its folder, git shows pure renames.

## Out of scope
- Card body/content, priorities, `id` values — unchanged.
- No `BOARD.md` index — folders are the view.
- App code (`styled.rs` etc.) — docs + skill only.

## Narrative
- 2026-06-23: Captured from brainstorming. Decisions: status subfolders (chosen
  over flat+BOARD.md and over status-in-filename); date prefix uses `created`
  (immutable) not `updated` (rename churn); 5 folders (not merged done+archive);
  no BOARD.md (folders ARE the view); `status:` frontmatter kept and must match
  folder (Obsidian/3rd-party tools read it) rather than dropped. Rejected:
  status-in-filename (mutable → churn, breaks git/backlinks), updated-date prefix
  (same churn). Migration + skill rewrite kept as ONE card — coupled, skill rules
  must match migrated layout or next card drifts. Spec committed at
  `superpowers/specs/2026-06-23-kanban-folder-restructure-design.md`. Next:
  writing-plans.
- 2026-06-23: Executed via subagent-driven-development on branch
  `chore/kanban-folder-restructure`. Task 1 (migration): 57 cards `git mv`d into
  status subfolders (backlog 19, todo 3, doing 0, done 33, archive 2); verified
  count, status==folder, pure renames (100% similarity, no content edits). Note:
  count was 57 not the plan's 56 — this very card was added after the spec.
  zsh gotcha: `status` is a read-only special var, renamed loop vars to `st`/`cr`.
  Task 2 (skill rewrite): 6 edits to `~/.claude/skills/kanban-skill/SKILL.md`
  (filename rule, folder invariant, create/move/board/query ops, intro, workflow
  step 4); verified no stale `<slug>.md` refs and live board matches documented
  format. Subagent dispatch was bash-permission-blocked, so both tasks run inline.
  Moved to `done`.
