#!/usr/bin/env bash
set -euo pipefail
VERSION="$1"
echo "$VERSION" > VERSION
sed -i "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
# Also update workspace member versions
for toml in crates/*/Cargo.toml; do
    sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$toml"
done
cargo generate-lockfile
echo "Version bumped to $VERSION"
