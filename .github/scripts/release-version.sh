#!/usr/bin/env bash
set -euo pipefail

version=$(cargo metadata --locked --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "iwe") | .version')
tag_version=$RELEASE_TAG
case "$tag_version" in
  macos-*|windows-*|linux-*) tag_version=${tag_version#*-} ;;
esac
if [[ "${tag_version#v}" != "$version" ]]; then
  echo "Release tag must match Cargo.toml, with an optional operating-system prefix." >&2
  exit 1
fi
echo "RELEASE_VERSION=$version" >> "$GITHUB_ENV"
