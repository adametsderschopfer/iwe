#!/usr/bin/env bash
set -euo pipefail

version=${RELEASE_TAG#v}
assets=(
  "dist/iwe-$version-macos-arm64.dmg"
  "dist/iwe-$version-macos-x64.dmg"
  "dist/iwe-$version-windows-x64.zip"
  "dist/iwe-$version-windows-x86.zip"
  "dist/iwe-$version-linux-x64.tar.gz"
  "dist/iwe-$version-linux-x86.tar.gz"
)
for platform in macos-arm64 macos-x64 windows-x64 windows-x86 linux-x64 linux-x86; do
  assets+=("dist/iwe-$version-$platform-update.tar.gz")
done
for asset in "${assets[@]}"; do
  test -s "$asset"
done
(
  cd dist
  sha256sum -- iwe-* > SHA256SUMS
)
args=(--verify-tag --generate-notes --title "IWE $version")
version_without_build=${version%%+*}
if [[ "$version_without_build" == *-* ]]; then
  args+=(--prerelease)
fi
test -s dist/updates.json
test -s dist/updates.sig
gh release create "$RELEASE_TAG" "${assets[@]}" dist/SHA256SUMS dist/updates.json dist/updates.sig "${args[@]}"
