#!/usr/bin/env bash
set -euo pipefail

BUMP="${1:-}"
if [[ "$BUMP" != "major" && "$BUMP" != "minor" && "$BUMP" != "patch" ]]; then
    echo "Usage: $0 major|minor|patch" >&2
    exit 1
fi

command -v git-cliff > /dev/null || { echo "Error: git-cliff not found. Install: brew install git-cliff" >&2; exit 1; }

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
