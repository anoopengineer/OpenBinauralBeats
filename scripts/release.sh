#!/usr/bin/env bash
# Cuts a release: bumps the version in Cargo.toml, runs the checks, commits and tags.
# Pushing the tag triggers .github/workflows/release.yml. Usage: scripts/release.sh 0.2.0
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${1:-}"
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "usage: $0 <major.minor.patch>" >&2
  exit 1
fi
TAG="v$VERSION"
[[ -z "$(git status --porcelain)" ]] || { echo "working tree is not clean" >&2; exit 1; }
[[ "$(git branch --show-current)" == "main" ]] || { echo "releases are cut from main" >&2; exit 1; }
git rev-parse -q --verify "refs/tags/$TAG" >/dev/null && { echo "tag $TAG already exists" >&2; exit 1; }

CURRENT=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
if [[ "$CURRENT" != "$VERSION" ]]; then
  # First `version = ` line only (the [package] one).
  perl -0pi -e "s/^version = \"[^\"]*\"/version = \"$VERSION\"/m" Cargo.toml
fi

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
python3 scripts/check-site.py

if [[ -n "$(git status --porcelain)" ]]; then
  git commit -am "Release $TAG"
fi
git tag -a "$TAG" -m "OpenBinauralBeats $TAG"

echo
echo "Tagged $TAG. Publish it with:"
echo "  git push origin main $TAG"
