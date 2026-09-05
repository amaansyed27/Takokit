#!/usr/bin/env bash
set -euo pipefail

APP=${1:?usage: test-macos-resident.sh APP INSTALL_ROOT TAKOKIT_HOME}
INSTALL_ROOT=${2:?}
TAKOKIT_HOME_ROOT=${3:?}

[ "$(uname -s)" = Darwin ] || { echo "macOS resident acceptance requires Darwin" >&2; exit 1; }
APP_BIN="$APP/Contents/MacOS/Takokit"
TAKO="$INSTALL_ROOT/bin/tako"
[ -x "$APP_BIN" ] || { echo "installed Takokit.app executable is missing: $APP_BIN" >&2; exit 1; }
[ -x "$TAKO" ] || { echo "installed Takokit CLI is missing: $TAKO" >&2; exit 1; }
command -v swiftc >/dev/null 2>&1 || { echo "swiftc is required for macOS resident acceptance" >&2; exit 1; }

WORK="$(mktemp -d "${TMPDIR:-/tmp}/takokit-macos-resident.XXXXXX")"
RESIDENT_PID=
DIRECT_PID=
cleanup() {
  if [ -n "$RESIDENT_PID" ] && kill -0 "$RESIDENT_PID" >/dev/null 2>&1; then
    kill "$RESIDENT_PID" >/dev/null 2>&1 || true
    wait "$RESIDENT_PID" >/dev/null 2>&1 || true
  fi
  if [ -n "$DIRECT_PID" ] && kill -0 "$DIRECT_PID" >/dev/null 2>&1; then
    kill "$DIRECT_PID" >/dev/null 2>&1 || true
    wait "$DIRECT_PID" >/dev/null 2>&1 || true
  fi
  "$TAKO" server stop >/dev/null 2>&1 || true
  rm -rf -- "$WORK"
}
trap cleanup EXIT HUP INT TERM

export TAKOKIT_INSTALL_ROOT="$INSTALL_ROOT"
export TAKOKIT_HOME="$TAKOKIT_HOME_ROOT"
export TAKOKIT_APP_PATH="$APP"

cat > "$WORK/terminate-running-app.swift" <<'SWIFT'
import AppKit
import Darwin
import Foundation

guard CommandLine.arguments.count == 2,
      let rawPID = Int32(CommandLine.arguments[1]),
      let app = NSRunningApplication(processIdentifier: pid_t(rawPID)) else {
    exit(2)
}
if !app.terminate() {
    exit(3)
}
SWIFT
swiftc "$WORK/terminate-running-app.swift" -framework AppKit -o "$WORK/terminate-running-app"

identity_mode() {
  curl -fsS --max-time 1 http://127.0.0.1:5050/api/v1/daemon/identity 2>/dev/null |
    python3 -c 'import json,sys; print(json.load(sys.stdin).get("mode", ""))' 2>/dev/null || true
}

require_identity() {
  expected=$1
  for _ in $(seq 1 100); do
    mode="$(identity_mode)"
    if [ "$mode" = "$expected" ]; then
      curl -fsS --max-time 1 http://127.0.0.1:5050/api/v1/daemon/identity |
        python3 -c 'import json,os,pathlib,sys; d=json.load(sys.stdin); root=pathlib.Path(os.environ["TAKOKIT_INSTALL_ROOT"]).resolve(); exe=pathlib.Path(d["executable"]).resolve(); assert exe.parent == root/"bin", (exe, root); assert d["host"] in ("127.0.0.1","localhost","::1"); assert int(d["port"]) == 5050'
      return 0
    fi
    sleep 0.1
  done
  echo "expected Takokit daemon identity mode '$expected', got '$(identity_mode)'" >&2
  return 1
}

launch_resident() {
  "$APP_BIN" --background >"$WORK/resident.stdout.log" 2>"$WORK/resident.stderr.log" &
  RESIDENT_PID=$!
  for _ in $(seq 1 50); do
    kill -0 "$RESIDENT_PID" >/dev/null 2>&1 && return 0
    sleep 0.1
  done
  echo "Takokit.app resident exited during startup" >&2
  cat "$WORK/resident.stderr.log" >&2 || true
  return 1
}

quit_resident() {
  [ -n "$RESIDENT_PID" ] || { echo "resident PID is missing" >&2; return 1; }
  "$WORK/terminate-running-app" "$RESIDENT_PID"
  for _ in $(seq 1 100); do
    if ! kill -0 "$RESIDENT_PID" >/dev/null 2>&1; then
      wait "$RESIDENT_PID" >/dev/null 2>&1 || true
      RESIDENT_PID=
      return 0
    fi
    sleep 0.1
  done
  echo "Takokit.app did not terminate after a normal application quit request" >&2
  return 1
}

# Installed resident owns a managed server and must stop only that owned server on Quit.
"$TAKO" server stop >/dev/null 2>&1 || true
launch_resident
require_identity managed
quit_resident
for _ in $(seq 1 100); do
  [ -z "$(identity_mode)" ] && break
  sleep 0.1
done
[ -z "$(identity_mode)" ] || { echo "Takokit.app Quit left its managed server running" >&2; exit 1; }

# A directly started developer server is not resident-owned and must survive resident Quit.
"$TAKO" serve >"$WORK/direct-server.log" 2>&1 &
DIRECT_PID=$!
require_identity direct
sleep 0.5
launch_resident
require_identity direct
quit_resident
kill -0 "$DIRECT_PID" >/dev/null 2>&1 || { echo "Takokit.app Quit killed a direct developer server" >&2; exit 1; }
require_identity direct
kill "$DIRECT_PID" >/dev/null 2>&1 || true
wait "$DIRECT_PID" >/dev/null 2>&1 || true
DIRECT_PID=
for _ in $(seq 1 100); do
  [ -z "$(identity_mode)" ] && break
  sleep 0.1
done
[ -z "$(identity_mode)" ] || { echo "direct developer server did not stop during test cleanup" >&2; exit 1; }

echo "macOS installed resident lifecycle acceptance passed"
