# Kanban Folder Restructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganize `docs/kanban/` into five status subfolders with `created`-date-prefixed filenames, and rewrite the kanban skill to match.

**Architecture:** A one-shot migration moves the 56 flat cards into `docs/kanban/<status>/<created>-<slug>.md` using `git mv` (history-preserving), with no content edits. The kanban skill (`~/.claude/skills/kanban-skill/SKILL.md`) is then rewritten in place so all future create/move/list operations honor the new layout. Migration runs first so the skill's rules describe an already-true structure.

**Tech Stack:** Bash (migration + verification), Markdown/YAML (cards + skill).

## Global Constraints

- Status values, verbatim: `backlog`, `todo`, `doing`, `done`, `archive`.
- Filename format: `<created>-<slug>.md` where `<created>` is the card's existing `created` frontmatter date (`YYYY-MM-DD`) and `<slug>` is the existing filename stem. `created` only — never `updated`.
- Invariant: a card's `status:` frontmatter value MUST equal its parent folder name.
- No card body/content/frontmatter edits during migration — pure file moves. `id` values unchanged.
- No `BOARD.md` index file.
- Skill file path: `~/.claude/skills/kanban-skill/SKILL.md`, edited in place (outside this git repo).
- Card/spec/plan content in English.

---

### Task 1: Migrate cards into status subfolders

**Files:**
- Create: `docs/kanban/backlog/`, `docs/kanban/todo/`, `docs/kanban/doing/`, `docs/kanban/done/`, `docs/kanban/archive/`
- Modify: every `docs/kanban/*.md` (moved, not edited)

**Interfaces:**
- Consumes: nothing (first task).
- Produces: the on-disk layout `docs/kanban/<status>/<created>-<slug>.md` that Task 2's skill rules describe.

- [ ] **Step 1: Record the pre-migration count**

Run:
```bash
cd /Users/matteo.paoli/private/aiusagebar
ls docs/kanban/*.md | wc -l
```
Expected: `56`. Note this number — it is the invariant for Step 5.

- [ ] **Step 2: Stage untracked cards so `git mv` works on every file**

Two cards are currently untracked (`centralize-config-constants.md`, `kanban-folder-restructure.md`). `git mv` requires tracked paths, so stage all cards first.

Run:
```bash
cd /Users/matteo.paoli/private/aiusagebar
git add docs/kanban
git status --short docs/kanban | head
```
Expected: each card shows `A ` (added) or clean; no errors.

- [ ] **Step 3: Create the five status directories**

Run:
```bash
cd /Users/matteo.paoli/private/aiusagebar/docs/kanban
mkdir -p backlog todo doing done archive
```
Expected: no output, directories exist.

- [ ] **Step 4: Move each card into `<status>/<created>-<slug>.md`**

This reads `status` and `created` from each card's frontmatter and `git mv`s it. No content is edited.

Run:
```bash
cd /Users/matteo.paoli/private/aiusagebar/docs/kanban
for f in *.md; do
  [ -e "$f" ] || continue
  status=$(grep -m1 '^status:' "$f" | sed -E 's/^status:[[:space:]]*//')
  created=$(grep -m1 '^created:' "$f" | sed -E 's/^created:[[:space:]]*//')
  slug="${f%.md}"
  if [ -z "$status" ] || [ -z "$created" ]; then
    echo "SKIP (missing status/created): $f"; continue
  fi
  git mv "$f" "$status/$created-$slug.md"
done
```
Expected: no `SKIP` lines, no `git mv` errors.

- [ ] **Step 5: Verify count is unchanged**

Run:
```bash
cd /Users/matteo.paoli/private/aiusagebar/docs/kanban
find backlog todo doing done archive -name '*.md' | wc -l
```
Expected: `56` (matches Step 1). Also confirm no stray cards remain flat:
```bash
ls docs/kanban/*.md 2>/dev/null | wc -l
```
Expected: `0`.

- [ ] **Step 6: Verify status == parent folder for every card**

Run:
```bash
cd /Users/matteo.paoli/private/aiusagebar/docs/kanban
bad=0
for d in backlog todo doing done archive; do
  for f in "$d"/*.md; do
    [ -e "$f" ] || continue
    s=$(grep -m1 '^status:' "$f" | sed -E 's/^status:[[:space:]]*//')
    if [ "$s" != "$d" ]; then echo "MISMATCH $f status=$s folder=$d"; bad=1; fi
  done
done
[ "$bad" = 0 ] && echo "OK: all status fields match folders"
```
Expected: `OK: all status fields match folders`, no `MISMATCH` lines.

- [ ] **Step 7: Verify moves are pure renames (no content modified)**

Run:
```bash
cd /Users/matteo.paoli/private/aiusagebar
git status --short docs/kanban | grep -E '^.M' && echo "CONTENT MODIFIED — investigate" || echo "OK: no content modifications"
```
Expected: `OK: no content modifications`. (Entries will show as `R` renames for the 54 previously-committed cards and `A` for the 2 formerly-untracked cards — both are fine; only `M` content changes are a failure.)

- [ ] **Step 8: Commit the migration**

```bash
cd /Users/matteo.paoli/private/aiusagebar
git add docs/kanban
git commit -m "refactor(kanban): migrate cards into status subfolders with date-prefixed filenames"
```
Expected: commit succeeds; `git show --stat HEAD` lists renames into `backlog/ todo/ doing/ done/ archive/`.

---

### Task 2: Rewrite the kanban skill for the new layout

**Files:**
- Modify: `~/.claude/skills/kanban-skill/SKILL.md`

**Interfaces:**
- Consumes: the on-disk layout produced by Task 1.
- Produces: skill rules so future create/move/list/id operations keep the layout consistent. No code consumes this — it is the durable process definition.

- [ ] **Step 1: Rewrite the "Card file format" filename line**

In `~/.claude/skills/kanban-skill/SKILL.md`, replace:
```
File name: `docs/kanban/<slug>.md` (kebab-case derived from the title).
```
with:
```
File name: `docs/kanban/<status>/<created>-<slug>.md` — the card lives in the
subfolder named after its `status`, and the filename is prefixed with its
immutable `created` date (`YYYY-MM-DD`) followed by the kebab-case slug derived
from the title. The `created` prefix gives a chronological (birth-order) `ls`
sort within each status folder. Never use `updated` in the filename — a mutable
date would force a rename on every edit.
```

- [ ] **Step 2: Add the folder invariant to the Rules section**

In the `## Rules` list, add a new bullet:
```
- A card's `status` value MUST equal the name of its parent folder. Whenever
  `status` changes, the file MUST move to the matching folder in the same edit,
  and vice versa. The folder and the `status` field are kept in lockstep.
```

- [ ] **Step 3: Rewrite the "Create a card" operation**

Replace the `**Create a card**` bullet with:
```
- **Create a card** ("crea una card per X", "add a task to ..."): create a new
  `.md` file at `docs/kanban/<status>/<created>-<slug>.md`, where `<status>` is
  the initial status folder (`backlog/`, or `todo/` if work is imminent). Use a
  fresh `id` (`max(id)+1` scanning all subfolders), set `created`/`updated` to
  today, and add a Narrative entry capturing the source (e.g. "Captured from
  brainstorming on <topic>"). Create the status folder if it does not exist.
```

- [ ] **Step 4: Rewrite the "Move a card" operation**

Replace the `**Move a card**` bullet with:
```
- **Move a card** ("sposta la card 2 in doing", "mark task 1 done"): `git mv`
  the card file into the destination status folder (the filename is unchanged —
  the `<created>` prefix and slug stay), AND update the `status` field, update
  `updated`, and append a Narrative entry noting the transition — all in one
  change so folder and `status` never diverge. Enforce the `blocked_by` rule.
```

- [ ] **Step 5: Update id-scan and board/query operations to walk subfolders**

In the `id` frontmatter-field row, replace `scan existing cards` with `scan existing cards across all status subfolders`.

Replace the `**Show the board**` bullet with:
```
- **Show the board** ("mostrami la board", "what's the state"): read all cards
  by walking the five status subfolders and print them grouped by status, in a
  compact table (id, title, priority, blocked).
```
Replace the `**Query**` bullet with:
```
- **Query** ("cosa è bloccato?", "what's in doing?", "high priority backlog"):
  walk the status subfolders and filter. ("what's in doing?" reads `doing/`.)
```

- [ ] **Step 6: Update the intro and workflow-integration wording**

In the intro paragraph, replace `stored in `docs/kanban/`.` (first occurrence, "one per card, stored in") with:
```
stored under per-status subfolders in `docs/kanban/` (`backlog/`, `todo/`,
`doing/`, `done/`, `archive/`).
```
In the `## Workflow integration` numbered list, step 4 (`On completion, move the card to `done``), append: ` (which `git mv`s it into `done/`)`.

- [ ] **Step 7: Verify the skill file is internally consistent**

Run:
```bash
grep -nE 'docs/kanban/(<slug>|<status>)' ~/.claude/skills/kanban-skill/SKILL.md
grep -n 'status.*parent folder' ~/.claude/skills/kanban-skill/SKILL.md
grep -n 'git mv' ~/.claude/skills/kanban-skill/SKILL.md
```
Expected: the filename line shows `<status>/<created>-<slug>.md` (no remaining bare `docs/kanban/<slug>.md`); the invariant bullet is present; `git mv` appears in both the move operation and workflow step 4.

- [ ] **Step 8: Verify the skill matches the migrated board**

Confirm the rules describe reality — spot-check that the live structure matches the rewritten filename rule:
```bash
ls /Users/matteo.paoli/private/aiusagebar/docs/kanban/doing/ /Users/matteo.paoli/private/aiusagebar/docs/kanban/todo/ 2>/dev/null
```
Expected: filenames of the form `YYYY-MM-DD-<slug>.md` inside status folders, matching the skill's documented format. (The skill file is outside the git repo — no commit step.)

---

## Notes for the executor

- Tasks are ordered: migration (Task 1) MUST land before the skill rewrite (Task 2) so Step 8's verification compares the skill against a real migrated board.
- The `spec:`/`plan:` relative paths inside cards are not touched by this work; they were already relative and remain as-is (out of scope).
- Card 57 (`kanban-folder-restructure`, the card tracking this very work) is migrated by Task 1 like any other card — it moves into `todo/` as `todo/2026-06-23-kanban-folder-restructure.md`.
