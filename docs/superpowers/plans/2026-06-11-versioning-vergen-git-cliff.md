# Versioning: vergen + git-cliff + release script — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Embed git-describe version at compile time; add a release script that bumps, tags, and generates CHANGELOG.md.

**Architecture:** `vergen-git2` in `build.rs` emits `VERGEN_GIT_DESCRIBE` at compile time. `src/version.rs` exposes `app_version()` via a pure testable helper. `cliff.toml` + `scripts/release.sh` handle changelog generation and tagging.

**Tech Stack:** Rust (`vergen-git2 = "1"` build-dep), `git-cliff` (must be on PATH for release), macOS bash.

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `Cargo.toml` | add `[build-dependencies]` block |
| Modify | `build.rs` | add vergen-git2 emit alongside existing macOS target |
| Create | `src/version.rs` | `format_version()` (pure, testable) + `app_version()` |
| Modify | `src/main.rs` | add `mod version;` |
| Create | `cliff.toml` | git-cliff Conventional Commits config |
| Create | `scripts/release.sh` | bump + tag + changelog, print push instructions |

---

### Task 1: Add vergen-git2 build dependency + update build.rs

**Files:**
- Modify: `Cargo.toml`
- Modify: `build.rs`

- [ ] **Step 1: Add build dependency to Cargo.toml**

Open `Cargo.toml`. After the `[dependencies]` block, add:

```toml
[build-dependencies]
vergen-git2 = "1"
```

- [ ] **Step 2: Update build.rs to emit VERGEN_GIT_DESCRIBE**

Replace the contents of `build.rs` with:

```rust
fn main() {
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=11.0");

    vergen_git2::Emitter::default()
        .add_instructions(
            &vergen_git2::Git2Builder::default()
                .describe(true, true, None)
                .build()
                .unwrap(),
        )
        .unwrap()
        .emit()
        .unwrap();
}
```

`describe(true, true, None)` = `git describe --tags --always`. `--always` means it falls back to the short commit hash when no tags exist, so the build never fails.

- [ ] **Step 3: Verify it compiles**

```bash
cargo check
```

Expected: no errors. `VERGEN_GIT_DESCRIBE` is now available as a compile-time env var.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock build.rs
git commit -m "build: add vergen-git2, emit VERGEN_GIT_DESCRIBE at compile time"
```

---

### Task 2: Create src/version.rs (TDD)

**Files:**
- Create: `src/version.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/version.rs with failing tests first**

```rust
pub fn format_version(cargo: &str, git: &str) -> String {
    todo!()
}

pub fn app_version() -> &'static str {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_tag_returns_git_string() {
        assert_eq!(format_version("0.1.0", "0.1.0"), "0.1.0");
    }

    #[test]
    fn commits_after_tag_returns_git_string() {
        assert_eq!(
            format_version("0.1.0", "0.1.0-3-gabcdef"),
            "0.1.0-3-gabcdef"
        );
    }

    #[test]
    fn no_tags_returns_cargo_plus_hash() {
        assert_eq!(
            format_version("0.1.0", "gabcdef"),
            "0.1.0+gabcdef"
        );
    }
}
```

- [ ] **Step 2: Add mod declaration to src/main.rs**

In `src/main.rs`, add near the top alongside other `mod` declarations:

```rust
mod version;
```

- [ ] **Step 3: Run tests — verify they fail**

```bash
cargo test version
```

Expected: three test failures with `not yet implemented`.

- [ ] **Step 4: Implement format_version and app_version**

Replace the `todo!()` bodies in `src/version.rs`:

```rust
pub fn format_version(cargo: &str, git: &str) -> String {
    if git.starts_with(cargo) {
        git.to_string()
    } else {
        format!("{cargo}+{git}")
    }
}

pub fn app_version() -> String {
    format_version(env!("CARGO_PKG_VERSION"), env!("VERGEN_GIT_DESCRIBE"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_tag_returns_git_string() {
        assert_eq!(format_version("0.1.0", "0.1.0"), "0.1.0");
    }

    #[test]
    fn commits_after_tag_returns_git_string() {
        assert_eq!(
            format_version("0.1.0", "0.1.0-3-gabcdef"),
            "0.1.0-3-gabcdef"
        );
    }

    #[test]
    fn no_tags_returns_cargo_plus_hash() {
        assert_eq!(
            format_version("0.1.0", "gabcdef"),
            "0.1.0+gabcdef"
        );
    }
}
```

- [ ] **Step 5: Run tests — verify they pass**

```bash
cargo test version
```

Expected:
```
test version::tests::exact_tag_returns_git_string ... ok
test version::tests::commits_after_tag_returns_git_string ... ok
test version::tests::no_tags_returns_cargo_plus_hash ... ok
```

- [ ] **Step 6: Commit**

```bash
git add src/version.rs src/main.rs
git commit -m "feat(version): add app_version() with vergen git-describe embedding"
```

---

### Task 3: Add cliff.toml

**Files:**
- Create: `cliff.toml`

- [ ] **Step 1: Create cliff.toml**

```toml
[changelog]
header = "# Changelog\n\n"
body = """
{% if version %}\
## [{{ version | trim_start_matches(pat="v") }}] - {{ timestamp | date(format="%Y-%m-%d") }}
{% else %}\
## [Unreleased]
{% endif %}\
{% for group, commits in commits | group_by(attribute="group") %}
### {{ group }}
{% for commit in commits %}
- {{ commit.message | upper_first }}\
{% endfor %}
{% endfor %}\n
"""
footer = ""
trim = true

[git]
conventional_commits = true
filter_unconventional = true
commit_parsers = [
  { message = "^feat", group = "Features" },
  { message = "^fix", group = "Bug Fixes" },
  { message = "^refactor", group = "Refactoring" },
  { message = "^test", group = "Tests" },
  { message = "^docs", group = "Documentation" },
  { message = "^chore\\(release\\)", skip = true },
  { message = "^chore", group = "Miscellaneous" },
]
filter_commits = true
tag_pattern = "v[0-9].*"
```

- [ ] **Step 2: Verify cliff.toml parses (requires git-cliff on PATH)**

```bash
git-cliff --config cliff.toml --unreleased 2>&1 | head -20
```

If `git-cliff` is not installed: `brew install git-cliff` or `cargo install git-cliff`. Expected: prints an unreleased changelog section (or empty if no conventional commits since last tag).

- [ ] **Step 3: Commit**

```bash
git add cliff.toml
git commit -m "chore: add cliff.toml for conventional commits changelog"
```

---

### Task 4: Write scripts/release.sh

**Files:**
- Create: `scripts/release.sh`

- [ ] **Step 1: Create the release script**

```bash
#!/usr/bin/env bash
set -euo pipefail

BUMP="${1:-}"
if [[ "$BUMP" != "major" && "$BUMP" != "minor" && "$BUMP" != "patch" ]]; then
    echo "Usage: $0 major|minor|patch" >&2
    exit 1
fi

# Must run from repo root
REPO_ROOT="$(git rev-parse --show-toplevel)"
if [[ "$(pwd)" != "$REPO_ROOT" ]]; then
    echo "Run from repo root: $REPO_ROOT" >&2
    exit 1
fi

# Parse current version from Cargo.toml
CURRENT="$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')"
MAJOR="$(echo "$CURRENT" | cut -d. -f1)"
MINOR="$(echo "$CURRENT" | cut -d. -f2)"
PATCH="$(echo "$CURRENT" | cut -d. -f3)"

case "$BUMP" in
    major) NEW="$((MAJOR + 1)).0.0" ;;
    minor) NEW="${MAJOR}.$((MINOR + 1)).0" ;;
    patch) NEW="${MAJOR}.${MINOR}.$((PATCH + 1))" ;;
esac

echo "Bumping $CURRENT → $NEW"
read -r -p "Continue? [y/N] " CONFIRM
if [[ "$CONFIRM" != "y" && "$CONFIRM" != "Y" ]]; then
    echo "Aborted." >&2
    exit 1
fi

# Bump Cargo.toml (macOS sed syntax)
sed -i '' "s/^version = \"$CURRENT\"/version = \"$NEW\"/" Cargo.toml

# Regenerate CHANGELOG.md
git-cliff --config cliff.toml --tag "v$NEW" -o CHANGELOG.md

# Commit and tag
git add Cargo.toml CHANGELOG.md
git commit -m "chore(release): v$NEW"
git tag "v$NEW"

echo ""
echo "Done. To publish:"
echo "  git push && git push --tags"
```

- [ ] **Step 2: Make it executable**

```bash
chmod +x scripts/release.sh
```

- [ ] **Step 3: Dry-run sanity check (no mutation)**

```bash
bash -n scripts/release.sh
```

Expected: no output (syntax OK).

- [ ] **Step 4: Commit**

```bash
git add scripts/release.sh
git commit -m "chore: add release.sh for bump + tag + changelog"
```

---

### Task 5: Update kanban card

- [ ] **Step 1: Move card #2 to doing**

In `docs/kanban/versioning-vergen-git-cliff.md`, set `status: doing`, update `updated: 2026-06-11`, link plan in frontmatter (`plan: superpowers/plans/2026-06-11-versioning-vergen-git-cliff.md`), append Narrative entry.

- [ ] **Step 2: On completion, move card to done**

Set `status: done`, append closing Narrative entry.

---

## Self-Review

**Spec coverage:**
- [x] Dev builds show `git describe` output — Task 1 + 2
- [x] Release builds show clean semver on exact tag — `format_version` handles `git.starts_with(cargo)` case
- [x] `Cargo.toml` only modified at release — only `release.sh` touches it
- [x] Releases are explicit + human-triggered — `release.sh` with confirmation prompt
- [x] `app_version()` single function — Task 2
- [x] `cliff.toml` with Conventional Commits — Task 3
- [x] `scripts/release.sh` — Task 4
- [x] `chore(release)` commits skipped in changelog — `cliff.toml` `skip = true`
- [x] No auto-push — script prints instructions only

**Placeholder scan:** none found.

**Type consistency:** `format_version` defined in Task 2 Step 1, used in `app_version()` same file. No cross-task type drift.
