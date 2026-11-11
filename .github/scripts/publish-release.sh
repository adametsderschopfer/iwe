#!/usr/bin/env bash
set -euo pipefail

if [[ ! "$RELEASE_TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Use a stable release tag in the form vMAJOR.MINOR.PATCH." >&2
  exit 1
fi
version=${RELEASE_TAG#v}
assets=(
  "dist/iwe-code-$version-macos-arm64.dmg" "dist/iwe-code-$version-macos-x64.dmg"
  "dist/iwe-code-$version-windows-x64.zip" "dist/iwe-code-$version-windows-x86.zip"
  "dist/iwe-code-$version-windows-x64-setup.exe" "dist/iwe-code-$version-windows-x86-setup.exe"
  "dist/iwe-code-$version-linux-x64.tar.gz" "dist/iwe-code-$version-linux-x86.tar.gz"
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
python3 .github/scripts/release-notes.py --version "$version" --output dist/release-notes.md
test -s dist/updates.json
test -s dist/updates.sig
gh release create "$RELEASE_TAG" "${assets[@]}" dist/SHA256SUMS dist/updates.json dist/updates.sig \
  --verify-tag --notes-file dist/release-notes.md --title "IWE Code $version"
