#!/usr/bin/env bash
#
# Release Crimson.  Usage:
#
#     ./release.sh 1.1.0
#
# Does everything in the right order, refuses to continue if anything
# is off, and runs the checks locally BEFORE pushing so you find
# problems in 30 seconds instead of 8 minutes of CI.
#
set -euo pipefail

die() { printf '\n\033[31merror:\033[0m %s\n\n' "$*" >&2; exit 1; }

# Undo the version bump so a failed release leaves the repo exactly as found.
rollback() {
  git checkout -- Cargo.toml 2>/dev/null || true
  git checkout -- Cargo.lock 2>/dev/null || true
}
step() { printf '\n\033[35m==>\033[0m \033[1m%s\033[0m\n' "$*"; }

VERSION="${1:-}"
[ -n "$VERSION" ] || die "No version given.  Usage: ./release.sh 1.1.0"

# Reject a leading v so both ./release.sh 1.1.0 and v1.1.0 work.
VERSION="${VERSION#v}"

[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] \
  || die "Version must look like 1.2.3 (got '$VERSION')"

cd "$(dirname "$0")"

# ---------------------------------------------------------------- checks
step "Checking repository state"

[ -z "$(git status --porcelain)" ] \
  || die "You have uncommitted changes. Commit or stash them first, then re-run."

BRANCH=$(git rev-parse --abbrev-ref HEAD)
[ "$BRANCH" = "main" ] \
  || die "You are on branch '$BRANCH', not main. Switch to main first."

git rev-parse "v$VERSION" >/dev/null 2>&1 \
  && die "Tag v$VERSION already exists. Pick a new version number."

echo "  Fetching latest from GitHub..."
git fetch --quiet origin main --tags

LOCAL=$(git rev-parse HEAD)
REMOTE=$(git rev-parse origin/main)
[ "$LOCAL" = "$REMOTE" ] \
  || die "Your main has diverged from GitHub. Run 'git pull --rebase' first."

CURRENT=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
echo "  Current version: $CURRENT"
echo "  New version:     $VERSION"
echo "  Branch:          $BRANCH (in sync with GitHub)"

# ---------------------------------------------------------------- bump
step "Bumping Cargo.toml"
# Only the first `version = "..."` line, which is the package version.
awk -v v="$VERSION" '
  !done && /^version[[:space:]]*=/ { sub(/"[^"]*"/, "\"" v "\""); done=1 }
  { print }
' Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml

grep -m1 '^version' Cargo.toml | sed 's/^/  now: /'

# Refresh Cargo.lock so the version bump is recorded there too.
cargo check --quiet 2>/dev/null || true

# ---------------------------------------------------------------- verify
step "Running the same checks CI will run (this is the slow part)"

echo "  clippy..."
if ! cargo clippy --release --quiet -- -D warnings; then
  rollback
  die "clippy failed. Nothing was committed or tagged. Fix the warnings and re-run."
fi

echo "  release build..."
if ! cargo build --release --quiet; then
  rollback
  die "Build failed. Nothing was committed or tagged."
fi

echo "  ok"

# ---------------------------------------------------------------- ship
step "Committing and tagging"
git add Cargo.toml Cargo.lock 2>/dev/null || git add Cargo.toml
git commit --quiet -m "Release v$VERSION"
git tag -a "v$VERSION" -m "Crimson v$VERSION"

step "Pushing to GitHub"
git push --quiet origin main
git push --quiet origin "v$VERSION"

REPO=$(git remote get-url origin | sed -E 's#.*github\.com[:/]##; s#\.git$##')

cat <<EOF

  Released v$VERSION

  GitHub Actions is now building all three platforms and will publish
  the release automatically when it finishes (about 5-10 minutes).

  Watch:    https://github.com/$REPO/actions
  Release:  https://github.com/$REPO/releases

  Your website needs no changes. The download buttons point at
  /releases/latest/ and will pick this up on their own.

EOF
