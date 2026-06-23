# Kanban folder restructure — status subfolders + date-prefixed filenames

## Problem

`docs/kanban/` holds 56 cards in one flat directory with kebab-slug filenames.
`ls` is alphabetical, so status is invisible without opening each file and live
cards drown among completed ones. The board is no longer human-scannable.

Status and dates already live in frontmatter (`status`, `created`, `updated`),
but nothing surfaces them at the filesystem level.

## Goal

Make the board human-manageable from the filesystem alone, without changing card
content, and update the `kanban` skill so all future card operations honor the new
layout.

## Design

### 1. Directory layout

Status is encoded by the parent folder (the card's *path*), one directory per
status value already defined by the skill:

```
docs/kanban/
  backlog/
  todo/
  doing/
  done/
  archive/
    2026-06-23-centralize-config-constants.md
```

### 2. Filename

`<created>-<slug>.md` where:
- `<created>` is the card's existing `created` frontmatter date (`YYYY-MM-DD`).
- `<slug>` is the existing kebab-case slug (unchanged).

`created` is immutable, so the filename never changes after creation. This gives a
correct chronological (birth-order) sort within each folder via plain `ls`.

Example: `2026-06-23-centralize-config-constants.md`.

`updated` is **not** used in the filename — using a mutable date would force a
rename on every edit, churning git history and breaking Obsidian backlinks.
`updated` stays in frontmatter only.

### 3. Frontmatter

Schema unchanged. The `status:` field is **kept** (Obsidian and third-party kanban
tools group by it). New invariant:

> A card's `status:` value MUST equal the name of its parent folder.

Both move together, always, in a single edit set. `id` field stays as-is.

### 4. Skill rewrite (`~/.claude/skills/kanban-skill/SKILL.md`)

Edited in place. Changes:

- **Card file format (line ~17):** replace
  `File name: docs/kanban/<slug>.md` with the subfolder + date-prefix rule:
  `docs/kanban/<status>/<created>-<slug>.md`.
- **Create operation:** write the new file directly into the folder matching its
  initial status (`backlog/` or `todo/`), named `<created>-<slug>.md`.
- **Move operation:** `git mv` the file into the destination status folder **and**
  update `status:`, `updated:`, and append a Narrative entry — one atomic change.
  Keep the existing `blocked_by` enforcement.
- **Show board / Query operations:** walk the five status directories instead of a
  single flat scan; group output by directory.
- **Rules section:** add the `status == parent folder` invariant.
- **id generation:** `max(id)+1` scan now walks all subfolders, not one directory.

### 5. Migration (one-shot, 56 cards)

A script performs the move; no card content is edited.

For each `*.md` directly under `docs/kanban/`:
1. Read frontmatter `status` and `created`.
2. `git mv` the file into `docs/kanban/<status>/<created>-<slug>.md`.

`git mv` preserves per-file history. The 5 status directories are created first.

**Verification:**
- Card count after == 56 (count before).
- Every file resides in a folder whose name equals its `status:` value.
- No file content diff beyond the rename (git shows pure renames).

### 6. Out of scope

- Card body/content, priorities, the `id` field — all unchanged.
- No `BOARD.md` index file: the folder layout *is* the board view.
- `styled.rs` / app code — untouched (this is a docs + skill change only).

## Coupling note

Migration (data) and skill rewrite (process) are one spec on purpose: the skill's
filename/move/list rules must match the migrated layout exactly, or the next card
created drifts from the structure. Ship together.
