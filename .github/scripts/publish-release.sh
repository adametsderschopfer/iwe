#!/usr/bin/env bash
set -euo pipefail

version=${RELEASE_TAG#v}
assets=(
  "dist/iwe-code-$version-macos-arm64.dmg"
  "dist/iwe-code-$version-macos-x64.dmg"
  "dist/iwe-code-$version-windows-x64.zip"
  "dist/iwe-code-$version-windows-x86.zip"
  "dist/iwe-code-$version-windows-x64-setup.exe"
  "dist/iwe-code-$version-windows-x86-setup.exe"
  "dist/iwe-code-$version-linux-x64.tar.gz"
  "dist/iwe-code-$version-linux-x86.tar.gz"
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
args=(--verify-tag --generate-notes --title "IWE Code $version")
version_without_build=${version%%+*}
if [[ "$version_without_build" == *-* ]]; then
  args+=(--prerelease)
fi
test -s dist/updates.json
test -s dist/updates.sig
gh release create "$RELEASE_TAG" "${assets[@]}" dist/SHA256SUMS dist/updates.json dist/updates.sig "${args[@]}"
