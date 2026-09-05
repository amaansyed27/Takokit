#!/usr/bin/env bash
set -euo pipefail

APP=${1:?app bundle output path required}
VERSION=${2:?version required}
BUILD_ID=${3:?build id required}
STATUS_FILE=${4:?signing status path required}

[ "$(uname -s)" = Darwin ] || { echo "macOS app build requires Darwin" >&2; exit 1; }
case "$(uname -m)" in
  arm64|aarch64) SWIFT_ARCH=arm64 ;;
  x86_64|amd64) SWIFT_ARCH=x86_64 ;;
  *) echo "unsupported macOS architecture: $(uname -m)" >&2; exit 1 ;;
esac
command -v swiftc >/dev/null 2>&1 || { echo "swiftc is required to build Takokit.app" >&2; exit 1; }
command -v iconutil >/dev/null 2>&1 || { echo "iconutil is required to build Takokit.app" >&2; exit 1; }
command -v sips >/dev/null 2>&1 || { echo "sips is required to build Takokit.app" >&2; exit 1; }

rm -rf -- "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
sed "s/@VERSION@/$VERSION/g" packaging/macos/Info.plist > "$APP/Contents/Info.plist"

swiftc packaging/macos/TakokitResident.swift \
  -O \
  -target "${SWIFT_ARCH}-apple-macos12.0" \
  -framework AppKit \
  -o "$APP/Contents/MacOS/Takokit"
chmod 0755 "$APP/Contents/MacOS/Takokit"

ICONSET="$(mktemp -d "${TMPDIR:-/tmp}/takokit-icon.XXXXXX")/Takokit.iconset"
mkdir -p "$ICONSET"
cp assets/transparent-png/16.png "$ICONSET/icon_16x16.png"
cp assets/transparent-png/32.png "$ICONSET/icon_16x16@2x.png"
cp assets/transparent-png/32.png "$ICONSET/icon_32x32.png"
sips -z 64 64 assets/transparent-png/128.png --out "$ICONSET/icon_32x32@2x.png" >/dev/null
cp assets/transparent-png/128.png "$ICONSET/icon_128x128.png"
cp assets/transparent-png/256.png "$ICONSET/icon_128x128@2x.png"
cp assets/transparent-png/256.png "$ICONSET/icon_256x256.png"
cp assets/transparent-png/512.png "$ICONSET/icon_256x256@2x.png"
cp assets/transparent-png/512.png "$ICONSET/icon_512x512.png"
cp assets/transparent-png/1024.png "$ICONSET/icon_512x512@2x.png"
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Takokit.icns"
rm -rf -- "$(dirname -- "$ICONSET")"
cp assets/transparent-png/32.png "$APP/Contents/Resources/TakokitStatus.png"
printf '%s\n' "$BUILD_ID" > "$APP/Contents/Resources/build-id.txt"

DEVELOPER_ID=false
NOTARIZED=false
STATUS="ad-hoc signed"
if [ -n "${TAKOKIT_APPLE_SIGNING_IDENTITY:-}" ]; then
  codesign --force --deep --options runtime --timestamp \
    --sign "$TAKOKIT_APPLE_SIGNING_IDENTITY" "$APP"
  DEVELOPER_ID=true
  STATUS="Developer ID signed"

  if [ -n "${TAKOKIT_APPLE_ID:-}" ] && [ -n "${TAKOKIT_APPLE_TEAM_ID:-}" ] && [ -n "${TAKOKIT_APPLE_APP_PASSWORD:-}" ]; then
    NOTARY_ZIP="$(mktemp "${TMPDIR:-/tmp}/Takokit-notary.XXXXXX.zip")"
    rm -f -- "$NOTARY_ZIP"
    ditto -c -k --keepParent "$APP" "$NOTARY_ZIP"
    xcrun notarytool submit "$NOTARY_ZIP" \
      --apple-id "$TAKOKIT_APPLE_ID" \
      --team-id "$TAKOKIT_APPLE_TEAM_ID" \
      --password "$TAKOKIT_APPLE_APP_PASSWORD" \
      --wait
    rm -f -- "$NOTARY_ZIP"
    xcrun stapler staple "$APP"
    xcrun stapler validate "$APP"
    NOTARIZED=true
    STATUS="Developer ID signed and notarized"
  fi
else
  codesign --force --deep --sign - "$APP"
fi
codesign --verify --deep --strict "$APP"

mkdir -p "$(dirname -- "$STATUS_FILE")"
printf '{"artifact":"Takokit.app","status":"%s","developer_id":%s,"notarized":%s}\n' \
  "$STATUS" "$DEVELOPER_ID" "$NOTARIZED" > "$STATUS_FILE"
