# Versioning Design

**Date:** 2026-06-10
**Status:** Approved

## Goals

1. Dev builds show `git describe` output embedded in binary (`0.1.0-3-gabcdef`)
2. Release builds show clean semver (`0.1.0`) when built on an exact tag
3. `Cargo.toml` is only modified during a release — never dirty during normal dev
4. Releases are explicit, human-triggered: bump + tag + changelog in one script
5. About screen reads a single `app_version()` function that handles both cases

## Components

### 1. vergen (build-time git info)

**Crate:** `vergen-git2 = "1"` added to `[build-dependencies]`.

**`build.rs`** (repo root):
```rust
fn main() {
    vergen_git2::Emitter::default()
        .add_instructions(&vergen_git2::Git2Builder::default()
            .describe(true, true, None)
            .build()
            .unwrap())
        .unwrap()
        .emit()
        .unwrap();
}
```

`describe(true, true, None)` maps to `git describe --tags --always`. With `--always`, the build never fails even when no tags exist yet — falls back to short commit hash.

Emitted env var: `VERGEN_GIT_DESCRIBE`

### 2. Version string helper

New module `src/version.rs`, declared in `main.rs` as `mod version;`:

```rust
pub fn app_version() -> String {
    let cargo = env!("CARGO_PKG_VERSION");
    let git = env!("VERGEN_GIT_DESCRIBE");
    if git.starts_with(cargo) {
        git.to_string()
    } else {
        format!("{cargo}+{git}")
    }
}
```

Behavior:
- Exact tag `v0.1.0` → `git describe` = `0.1.0` → returns `"0.1.0"`
- 3 commits after tag → `git describe` = `0.1.0-3-gabcdef` → returns `"0.1.0-3-gabcdef"`
- No tags in repo → `git describe` = `gabcdef` → returns `"0.1.0+gabcdef"`

### 3. Release script (`scripts/release.sh`)

Single entrypoint for all release operations. Accepts one argument: `major`, `minor`, or `patch`.

Steps executed in order:
1. Parse current version from `Cargo.toml`
2. Compute new version per bump type
3. Prompt confirmation before any mutation
4. `sed` update `Cargo.toml` version field
5. `git-cliff --tag "v$NEW" -o CHANGELOG.md`
6. `git add Cargo.toml CHANGELOG.md`
7. `git commit -m "chore(release): v$NEW"`
8. `git tag "v$NEW"`
9. Print push instructions — does NOT push automatically

Push remains an explicit manual step (`git push && git push --tags`).

### 4. git-cliff configuration (`cliff.toml`)

Placed in repo root. Conventional Commits parser. Non-conventional prefixes (`kanban:`, `docs(spec):`) are filtered out via `filter_unconventional = true`.

Commit groups:
- `feat` → Features
- `fix` → Bug Fixes
- `refactor` → Refactoring
- `test` → Tests
- `docs` → Documentation
- `chore` → Miscellaneous
- `chore(release)` → skipped (release commit itself never appears in changelog)

Output format: `CHANGELOG.md`, markdown, one section per version tag, date from tag timestamp.

## Constraints

- Script requires `git-cliff` on PATH — install via `cargo install git-cliff` or `brew install git-cliff`
- Script is macOS bash (`sed -i ''` syntax); not portable to GNU sed without modification
- `vergen-git2` links against `libgit2` — compile-time dependency, no git CLI required
- Release script must be run from repo root

## Out of scope

- CI/CD automation (GitHub Actions release pipeline)
- Publishing to crates.io
- Automated push after tag
- About screen UI (separate spec)
