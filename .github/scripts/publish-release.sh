#!/usr/bin/env bash
set -euo pipefail

family=all
version=$RELEASE_TAG
case "$RELEASE_TAG" in
  macos-*|windows-*|linux-*) family=${RELEASE_TAG%%-*}; version=${RELEASE_TAG#*-} ;;
esac
version=${version#v}
if [[ "${RELEASE_PLATFORM:-$family}" != "$family" || ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Release platform and stable version must match the tag." >&2
  exit 1
fi
assets=()
if [[ "$family" == all || "$family" == macos ]]; then
  assets+=("dist/iwe-code-$version-macos-arm64.dmg" "dist/iwe-code-$version-macos-x64.dmg")
fi
if [[ "$family" == all || "$family" == windows ]]; then
  assets+=("dist/iwe-code-$version-windows-x64.zip" "dist/iwe-code-$version-windows-x86.zip"
    "dist/iwe-code-$version-windows-x64-setup.exe" "dist/iwe-code-$version-windows-x86-setup.exe")
fi
if [[ "$family" == all || "$family" == linux ]]; then
  assets+=("dist/iwe-code-$version-linux-x64.tar.gz" "dist/iwe-code-$version-linux-x86.tar.gz")
fi
for platform in macos-arm64 macos-x64 windows-x64 windows-x86 linux-x64 linux-x86; do
  if [[ "$family" != all && "$platform" != "$family-"* ]]; then continue; fi
  assets+=("dist/iwe-$version-$platform-update.tar.gz")
done
for asset in "${assets[@]}"; do
  test -s "$asset"
done
(
  cd dist
  sha256sum -- iwe-* > SHA256SUMS
)
title="IWE Code $version"
if [[ "$family" != all ]]; then title="$title ($family)"; fi
args=(--verify-tag --generate-notes --title "$title")
if [[ "$family" != all ]]; then
  # Legacy clients must keep seeing the last shared migration release.
  args+=(--latest=false)
fi
test -s dist/updates.json
test -s dist/updates.sig
gh release create "$RELEASE_TAG" "${assets[@]}" dist/SHA256SUMS dist/updates.json dist/updates.sig "${args[@]}"
