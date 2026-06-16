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

# Pre-flight: branch
BRANCH="$(git rev-parse --abbrev-ref HEAD)"
[[ "$BRANCH" == "master" ]] || {
    echo "Error: must be on master (current: $BRANCH)" >&2
    exit 1
}

# Pre-flight: working tree clean
git diff --quiet && git diff --cached --quiet || {
    echo "Error: working tree not clean. Commit or stash first." >&2
    exit 1
}

# Pre-flight: sync with origin
git fetch --quiet origin master
LOCAL="$(git rev-parse HEAD)"
REMOTE="$(git rev-parse origin/master)"
[[ "$LOCAL" == "$REMOTE" ]] || {
    echo "Error: local master not in sync with origin/master." >&2
    echo "  local:  $LOCAL" >&2
    echo "  remote: $REMOTE" >&2
    exit 1
}

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

# Pre-flight: tag must not exist locally or on origin
if git rev-parse "v$NEW" >/dev/null 2>&1; then
    echo "Error: tag v$NEW already exists locally." >&2
    exit 1
fi
if git ls-remote --tags --exit-code origin "v$NEW" >/dev/null 2>&1; then
    echo "Error: tag v$NEW already exists on origin." >&2
    exit 1
fi

echo "Running quality gate (clippy + tests)..."
cargo clippy -- -D warnings
cargo test
echo "Quality gate passed."

echo "Bumping $CURRENT → $NEW"
read -r -p "Continue? [y/N] " CONFIRM
if [[ "$CONFIRM" != "y" && "$CONFIRM" != "Y" ]]; then
    echo "Aborted." >&2
    exit 1
fi

# Rollback working tree if any step fails after this point
cleanup_on_error() {
    local rc=$?
    echo "" >&2
    echo "Error: release failed (exit $rc). Rolling back working tree..." >&2
    git checkout -- Cargo.toml CHANGELOG.md 2>/dev/null || true
    echo "  (if a commit was already made: git reset --hard origin/master)" >&2
    exit "$rc"
}
trap cleanup_on_error ERR

# Bump Cargo.toml (macOS sed syntax)
sed -i '' "s/^version = \"$CURRENT\"/version = \"$NEW\"/" Cargo.toml

# Sync Cargo.lock to new version
cargo check -q

# Regenerate CHANGELOG.md
git-cliff --config cliff.toml --tag "v$NEW" -o CHANGELOG.md

# Commit and tag
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore(release): v$NEW"
git tag -a "v$NEW" -m "Release v$NEW"

echo ""
read -r -p "Push to origin now? [y/N] " PUSH
if [[ "$PUSH" == "y" || "$PUSH" == "Y" ]]; then
    git push origin master
    git push origin "v$NEW"
    echo "Pushed v$NEW to origin."
else
    echo "Skipped push. To publish later:"
    echo "  git push origin master && git push origin v$NEW"
    echo "  cargo build --release"
    echo "  codesign --force -s - target/release/aiusagebar"
    echo "  mkdir -p dist && cp target/release/aiusagebar dist/aiusagebar-macos-arm64-v$NEW"
    echo "  gh release create v$NEW --title v$NEW --notes-file <notes-file> dist/aiusagebar-macos-arm64-v$NEW"
    exit 0
fi

trap 'echo "" >&2; echo "Post-push step failed. Tag v$NEW is live on origin." >&2; echo "Run the build/sign/release steps manually." >&2' ERR

echo "Building release binary..."
cargo build --release

echo "Signing binary (ad-hoc)..."
codesign --force -s - target/release/aiusagebar
codesign --verify --verbose target/release/aiusagebar

echo "Packaging..."
mkdir -p dist
cp target/release/aiusagebar "dist/aiusagebar-macos-arm64-v${NEW}"

echo "Extracting release notes from CHANGELOG.md..."
NOTES=$(mktemp)
awk -v tag="## [v${NEW}]" 'index($0,tag)==1{p=1;next} /^## \[/{p=0} p' CHANGELOG.md > "$NOTES"

echo "Creating GitHub release..."
gh release create "v${NEW}" \
    "dist/aiusagebar-macos-arm64-v${NEW}" \
    --title "v${NEW}" \
    --notes-file "$NOTES"
rm -f "$NOTES"

echo ""
echo "Released: https://github.com/mttpla/aiusagebar/releases/tag/v${NEW}"
