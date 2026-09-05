#!/usr/bin/env bash
set -euo pipefail

ARCHIVE=${1:?usage: test-pristine-unix-portable.sh ARCHIVE PLATFORM ARCH VERSION}
PLATFORM=${2:?}
ARCH=${3:?}
VERSION=${4:-0.3.0}
WORK="$(mktemp -d "${TMPDIR:-/tmp}/takokit-pristine.XXXXXX")"
SERVER_STARTED=0
cleanup() {
  if [ "$SERVER_STARTED" -eq 1 ]; then
    "$TAKO" server stop >/dev/null 2>&1 || true
  fi
  rm -rf -- "$WORK"
}
trap cleanup EXIT HUP INT TERM

export HOME="$WORK/Home pristine ü"
export TAKOKIT_HOME="$HOME/.takokit"
unset UV
mkdir -p "$HOME" "$WORK/extracted" "$WORK/workspace"

tar -xzf "$ARCHIVE" -C "$WORK/extracted"
ROOT="$WORK/extracted/Takokit-v$VERSION-$PLATFORM-$ARCH"
TAKO="$ROOT/bin/tako"
test -x "$TAKO"
test ! -e "$TAKOKIT_HOME"

# The packaged product must bootstrap itself. No system/Homebrew uv is injected.
test "$("$TAKO" version | sed -n '1p')" = "takokit $VERSION"
"$TAKO" --direct pull kokoro
MANAGED_UV="$TAKOKIT_HOME/tools/uv/uv"
if [ "$PLATFORM" = windows ]; then MANAGED_UV="$TAKOKIT_HOME/tools/uv/uv.exe"; fi
test -x "$MANAGED_UV"
test "$("$MANAGED_UV" --version | awk '{print $2}')" = "0.12.10"

audio_json="$WORK/kokoro.json"
"$TAKO" --direct --output json --workspace "$WORK/workspace" run kokoro \
  "Takokit pristine Kokoro acceptance on $PLATFORM $ARCH." > "$audio_json"
python3 - "$audio_json" <<'PY'
import json,pathlib,sys
value=json.load(open(sys.argv[1],encoding='utf-8'))
path=value.get('output_path') or value.get('data',{}).get('output_path')
assert path, value
p=pathlib.Path(path)
assert p.is_file() and p.stat().st_size > 44, p
print(p)
PY
KOKORO_WAV="$(python3 - "$audio_json" <<'PY'
import json,sys
v=json.load(open(sys.argv[1],encoding='utf-8'))
print(v.get('output_path') or v.get('data',{}).get('output_path'))
PY
)"

# Reuse the already-installed Kokoro runtime and model without bootstrap mutation.
UV_BEFORE="$(python3 - "$MANAGED_UV" <<'PY'
import hashlib,sys
print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())
PY
)"
"$TAKO" --direct --output json --workspace "$WORK/workspace" run kokoro \
  "Takokit Kokoro runtime reuse acceptance." > "$WORK/kokoro-reuse.json"
UV_AFTER="$(python3 - "$MANAGED_UV" <<'PY'
import hashlib,sys
print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())
PY
)"
test "$UV_BEFORE" = "$UV_AFTER"

"$TAKO" --direct pull whisper-tiny
"$TAKO" --direct --output json --workspace "$WORK/workspace" run whisper-tiny \
  --file "$KOKORO_WAV" > "$WORK/whisper.json"
python3 - "$WORK/whisper.json" <<'PY'
import json,sys
value=json.load(open(sys.argv[1],encoding='utf-8'))
text=value.get('text') or value.get('data',{}).get('text')
assert isinstance(text,str) and text.strip(), value
PY

"$TAKO" server start >/dev/null
SERVER_STARTED=1
curl -fsS http://127.0.0.1:5050/health >/dev/null
curl -fsS http://127.0.0.1:5050/gui/ >/dev/null
"$TAKO" --output human list models > "$WORK/models-human.txt"
! python3 - "$WORK/models-human.txt" <<'PY'
import json,sys
try:
    json.load(open(sys.argv[1],encoding='utf-8'))
except Exception:
    raise SystemExit(1)
raise SystemExit(0)
PY
"$TAKO" --output json list models > "$WORK/models.json"
python3 -m json.tool "$WORK/models.json" >/dev/null
"$TAKO" server stop >/dev/null
SERVER_STARTED=0

echo "pristine portable acceptance passed for $PLATFORM $ARCH"
