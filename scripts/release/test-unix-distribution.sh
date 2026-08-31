#!/usr/bin/env bash
set -euo pipefail

ARCHIVE=${1:?usage: test-unix-distribution.sh ARCHIVE PLATFORM ARCH VERSION}
PLATFORM=${2:?}
ARCH=${3:?}
VERSION=${4:-0.3.0}
FIXTURE_ROOT=${5:-}
REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/takokit-acceptance.XXXXXX")"
SERVER_PID=
cleanup() {
  [ -z "$SERVER_PID" ] || kill "$SERVER_PID" >/dev/null 2>&1 || true
  rm -rf -- "$WORK"
}
trap cleanup EXIT HUP INT TERM

WEB="$WORK/web"
HOME_ROOT="$WORK/Home ü with spaces"
mkdir -p "$WEB" "$HOME_ROOT"
cp "$ARCHIVE" "$WEB/$(basename "$ARCHIVE")"
SHA="$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$ARCHIVE")"
PORT=18765
ARTIFACT_URL="http://127.0.0.1:$PORT/$(basename "$ARCHIVE")"
cat > "$WEB/metadata.json" <<EOF
{"version":"$VERSION","platform":"$PLATFORM","architecture":"$ARCH","signing_key_id":"takokit-release-v1","test_fixture":false,"artifact_name":"$(basename "$ARCHIVE")","artifact_url":"$ARTIFACT_URL","artifact_sha256":"$SHA"}
EOF
(cd "$WEB" && python3 -m http.server "$PORT" --bind 127.0.0.1 >/dev/null 2>&1) &
SERVER_PID=$!
for _ in $(seq 1 50); do curl -fsS "http://127.0.0.1:$PORT/metadata.json" >/dev/null 2>&1 && break; sleep 0.1; done

export HOME="$HOME_ROOT"
export SHELL=/bin/bash
export TAKOKIT_ALLOW_INSECURE_LOOPBACK=1
export TAKOKIT_METADATA_URL="http://127.0.0.1:$PORT/metadata.json"
export TAKOKIT_INSTALL_ROOT="$HOME_ROOT/.local/share/Takokit ü"
export TAKOKIT_BIN_DIR="$HOME_ROOT/.local/bin"
export TAKOKIT_APPLICATIONS_DIR="$HOME_ROOT/Applications"
export TAKOKIT_DESKTOP_DIR="$HOME_ROOT/.local/share/applications"
export TAKOKIT_ICON_DIR="$HOME_ROOT/.local/share/icons/hicolor/512x512/apps"

mkdir -p "$HOME/.takokit" "$HOME/workspace/.tako"
printf 'preserve-user' > "$HOME/.takokit/preserve.txt"
printf 'preserve-workspace' > "$HOME/workspace/.tako/preserve.txt"
sh "$REPO_ROOT/scripts/install.sh"
test -x "$TAKOKIT_INSTALL_ROOT/bin/tako"
test "$("$TAKOKIT_INSTALL_ROOT/bin/tako" version | sed -n '1p')" = "takokit $VERSION"
test -f "$TAKOKIT_INSTALL_ROOT/resources/gui/index.html"
test -f "$TAKOKIT_INSTALL_ROOT/resources/registry/index.json"
test -f "$TAKOKIT_INSTALL_ROOT/resources/runners/python/piper_adapter.py"
test -L "$TAKOKIT_BIN_DIR/tako"

if [ "$PLATFORM" = linux ]; then
  test -f "$TAKOKIT_DESKTOP_DIR/com.dawnlightlabs.takokit.desktop"
  grep -F 'Terminal=false' "$TAKOKIT_DESKTOP_DIR/com.dawnlightlabs.takokit.desktop" >/dev/null
else
  APP="$TAKOKIT_APPLICATIONS_DIR/Takokit.app"
  test -x "$APP/Contents/MacOS/Takokit"
  grep -F 'com.dawnlightlabs.takokit' "$APP/Contents/Info.plist" >/dev/null
  grep -F '<key>CFBundleShortVersionString</key><string>0.3.0</string>' "$APP/Contents/Info.plist" >/dev/null
fi

"$TAKOKIT_INSTALL_ROOT/bin/tako" server start >/dev/null
"$TAKOKIT_INSTALL_ROOT/bin/tako" server status >/dev/null
curl -fsS http://127.0.0.1:5050/health >/dev/null
curl -fsS http://127.0.0.1:5050/openapi.json >/dev/null
curl -fsS http://127.0.0.1:5050/gui/ >/dev/null
"$TAKOKIT_INSTALL_ROOT/bin/tako" server stop >/dev/null

if [ "${TAKOKIT_REAL_MODEL_SMOKE:-0}" = 1 ]; then
  "$TAKOKIT_INSTALL_ROOT/bin/tako" --direct pull kokoro
  "$TAKOKIT_INSTALL_ROOT/bin/tako" --direct samples create
  test -s "$HOME/.takokit/samples/hello.wav"
  "$TAKOKIT_INSTALL_ROOT/bin/tako" --direct pull whisper-tiny
  "$TAKOKIT_INSTALL_ROOT/bin/tako" --direct test whisper-tiny --file "$HOME/.takokit/samples/hello.wav" --run
fi

if [ -n "$FIXTURE_ROOT" ]; then
  FIXTURE_MANIFEST="$FIXTURE_ROOT/release-manifest.json"
  FIXTURE_SIGNATURE="$FIXTURE_ROOT/release-manifest.sig"
  "$TAKOKIT_INSTALL_ROOT/bin/tako" update apply --manifest "$FIXTURE_MANIFEST" --signature "$FIXTURE_SIGNATURE" --allow-test >/dev/null
  JOURNAL="$HOME/.takokit/runtime/update-journal.json"
  for _ in $(seq 1 600); do
    state=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("state",""))' "$JOURNAL" 2>/dev/null || true)
    [ "$state" = completed ] && break
    [ "$state" = rolled_back ] && { echo "Unix update unexpectedly rolled back" >&2; exit 1; }
    sleep 0.1
  done
  test "$("$TAKOKIT_INSTALL_ROOT/bin/tako" version | sed -n '1p')" = 'takokit 0.3.1'
  test -f "$HOME/.takokit/preserve.txt"

  # Restore the candidate, then prove an interrupted Unix update rolls back to it.
  sh "$REPO_ROOT/scripts/install.sh" >/dev/null
  rm -f -- "$JOURNAL"
  TAKOKIT_UPDATER_TEST_FAILPOINT=after_replace "$TAKOKIT_INSTALL_ROOT/bin/tako" update apply --manifest "$FIXTURE_MANIFEST" --signature "$FIXTURE_SIGNATURE" --allow-test >/dev/null
  for _ in $(seq 1 600); do
    state=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("state",""))' "$JOURNAL" 2>/dev/null || true)
    [ "$state" = rolled_back ] && break
    sleep 0.1
  done
  test "$state" = rolled_back
  test "$("$TAKOKIT_INSTALL_ROOT/bin/tako" version | sed -n '1p')" = "takokit $VERSION"
fi

# Reinstall is idempotent and must not duplicate the managed PATH block.
sh "$REPO_ROOT/scripts/install.sh"
test "$(grep -c '# >>> Takokit managed PATH >>>' "$HOME/.bashrc")" -le 1

# An interrupted replacement must restore the previously installed tree.
if TAKOKIT_INSTALL_TEST_FAILPOINT=after_backup sh "$REPO_ROOT/scripts/install.sh" >/dev/null 2>&1; then
  echo "interrupted install fixture unexpectedly succeeded" >&2; exit 1
fi
test -x "$TAKOKIT_INSTALL_ROOT/bin/tako"

sh "$TAKOKIT_INSTALL_ROOT/uninstall.sh"
test ! -e "$TAKOKIT_INSTALL_ROOT"
test ! -e "$TAKOKIT_BIN_DIR/tako"
test -f "$HOME/.takokit/preserve.txt"
test -f "$HOME/workspace/.tako/preserve.txt"

# Portable extraction has no integration side effects.
PORTABLE_HOME="$WORK/portable-home"
mkdir -p "$PORTABLE_HOME"
tar -xzf "$ARCHIVE" -C "$PORTABLE_HOME"
PORTABLE_ROOT="$PORTABLE_HOME/Takokit-v$VERSION-$PLATFORM-$ARCH"
test -x "$PORTABLE_ROOT/bin/tako"
test "$("$PORTABLE_ROOT/bin/tako" version | sed -n '1p')" = "takokit $VERSION"
test ! -e "$PORTABLE_HOME/.local"

echo "unix distribution acceptance passed for $PLATFORM $ARCH"
