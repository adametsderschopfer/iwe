#!/usr/bin/env bash
set -euo pipefail

version=$(cargo metadata --locked --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "iwe") | .version')
if [[ "${RELEASE_TAG#v}" != "$version" ]]; then
  echo "Release tag must match Cargo.toml: v$version or $version." >&2
  exit 1
fi
echo "RELEASE_VERSION=$version" >> "$GITHUB_ENV"
