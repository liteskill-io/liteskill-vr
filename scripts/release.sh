#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: $0 <patch|minor|major>"
  exit 1
}

[[ $# -eq 1 ]] || usage

BUMP_TYPE="$1"
case "$BUMP_TYPE" in
  patch|minor|major) ;;
  *) usage ;;
esac

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Error: working directory is not clean. Commit or stash changes first."
  exit 1
fi

npm version "$BUMP_TYPE" --no-git-tag-version
VERSION=$(node -p "require('./package.json').version")

bash scripts/sync-version.sh

git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "release: v${VERSION}"
git tag -a "v${VERSION}" -m "v${VERSION}"

echo ""
echo "Created release commit and tag v${VERSION}"
echo "Push with: git push && git push origin v${VERSION}"
