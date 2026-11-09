#!/usr/bin/env bash
set -euo pipefail

: "${BUILD_TARGET:?}" "${ASSET_PLATFORM:?}" "${RELEASE_VERSION:?}" "${RUNNER_OS:?}"
mkdir -p dist
output=$(cd dist && pwd)
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
name="iwe-${RELEASE_VERSION}-${ASSET_PLATFORM}"
binary="target/$BUILD_TARGET/release/iwe"

case "$RUNNER_OS" in
  macOS)
    bundle="$stage/IWE.app/Contents"
    mkdir -p "$bundle/MacOS" "$bundle/Resources"
    cp assets/icon.icns "$bundle/Resources/icon.icns"
    cp "$binary" "$bundle/MacOS/iwe"
    chmod 755 "$bundle/MacOS/iwe"
    version=${RELEASE_VERSION%%[-+]*}
    cat > "$bundle/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>iwe</string>
  <key>CFBundleIdentifier</key><string>com.adametsderschopfer.iwe</string>
  <key>CFBundleName</key><string>IWE</string>
  <key>CFBundleDisplayName</key><string>IWE</string>
  <key>CFBundleIconFile</key><string>icon.icns</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
    plutil -lint "$bundle/Info.plist"
    codesign --force --sign - "$stage/IWE.app"
    codesign --verify --strict "$stage/IWE.app"
    COPYFILE_DISABLE=1 tar -czf "$output/$name-update.tar.gz" -C "$stage" IWE.app
    ln -s /Applications "$stage/Applications"
    hdiutil create -volname IWE -srcfolder "$stage" -ov -format UDZO "$output/$name.dmg"
    ;;
  Windows)
    cp "$binary.exe" "$stage/iwe.exe"
    cp README.md "$stage/README.md"
    tar -czf "$output/$name-update.tar.gz" -C "$stage" iwe.exe
    (cd "$stage" && 7z a -tzip "$output/$name.zip" iwe.exe README.md)
    ;;
  Linux)
    cp "$binary" "$stage/iwe"
    chmod 755 "$stage/iwe"
    cp README.md "$stage/README.md"
    cp assets/iwe.desktop "$stage/iwe.desktop"
    cp assets/icon.png "$stage/iwe.png"
    tar -czf "$output/$name-update.tar.gz" -C "$stage" iwe
    COPYFILE_DISABLE=1 tar -czf "$output/$name.tar.gz" -C "$stage" iwe README.md iwe.desktop iwe.png
    ;;
  *) echo "Unsupported release platform: $RUNNER_OS" >&2; exit 1 ;;
esac
