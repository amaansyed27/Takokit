#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/takokit-install-security.XXXXXX")"
SERVER_PID=
cleanup() { [ -z "$SERVER_PID" ] || kill "$SERVER_PID" >/dev/null 2>&1 || true; rm -rf -- "$WORK"; }
trap cleanup EXIT HUP INT TERM
mkdir -p "$WORK/web" "$WORK/home"
PORT=18766
(cd "$WORK/web" && python3 -m http.server "$PORT" --bind 127.0.0.1 >/dev/null 2>&1) &
SERVER_PID=$!
for _ in $(seq 1 50); do curl -fsS "http://127.0.0.1:$PORT/" >/dev/null 2>&1 && break; sleep 0.1; done

export HOME="$WORK/home"
export TAKOKIT_ALLOW_INSECURE_LOOPBACK=1
export TAKOKIT_INSTALL_ROOT="$HOME/.local/share/takokit"
export TAKOKIT_BIN_DIR="$HOME/.local/bin"
expect_failure() {
  name=$1
  if sh "$REPO_ROOT/scripts/install.sh" >"$WORK/$name.out" 2>&1; then
    echo "$name unexpectedly succeeded" >&2; exit 1
  fi
  test ! -e "$TAKOKIT_INSTALL_ROOT"
}

printf '{broken' > "$WORK/web/metadata.json"
export TAKOKIT_METADATA_URL="http://127.0.0.1:$PORT/metadata.json"
expect_failure malformed_metadata

cat > "$WORK/web/metadata.json" <<EOF
{"version":"0.3.0","platform":"linux","architecture":"x86_64","signing_key_id":"takokit-release-v1","test_fixture":false,"artifact_name":"Takokit-v0.3.0-linux-x86_64.tar.gz","artifact_url":"http://127.0.0.1:$PORT/missing.tar.gz"}
EOF
expect_failure missing_checksum

cat > "$WORK/web/metadata.json" <<EOF
{"version":"0.3.0","platform":"linux","architecture":"x86_64","signing_key_id":"takokit-release-v1","test_fixture":false,"artifact_name":"Takokit-v0.3.0-linux-x86_64.tar.gz","artifact_url":"http://127.0.0.1:$PORT/missing.tar.gz","artifact_sha256":"$(printf 'a%.0s' {1..64})"}
EOF
expect_failure http_failure

printf 'not an archive' > "$WORK/web/Takokit-v0.3.0-linux-x86_64.tar.gz"
BAD_SHA=$(python3 -c 'print("b"*64)')
cat > "$WORK/web/metadata.json" <<EOF
{"version":"0.3.0","platform":"linux","architecture":"x86_64","signing_key_id":"takokit-release-v1","test_fixture":false,"artifact_name":"Takokit-v0.3.0-linux-x86_64.tar.gz","artifact_url":"http://127.0.0.1:$PORT/Takokit-v0.3.0-linux-x86_64.tar.gz","artifact_sha256":"$BAD_SHA"}
EOF
expect_failure bad_sha

cat > "$WORK/web/metadata.json" <<EOF
{"version":"0.3.0","platform":"linux","architecture":"x86_64","signing_key_id":"takokit-release-v1","test_fixture":false,"artifact_name":"Takokit-v0.3.0-linux-x86_64.tar.gz","artifact_url":"file:///tmp/unsafe.tar.gz","artifact_sha256":"$(printf 'a%.0s' {1..64})"}
EOF
expect_failure unsafe_url

python3 - "$WORK/web/Takokit-v0.3.0-linux-x86_64.tar.gz" <<'PY'
import io,tarfile,sys
with tarfile.open(sys.argv[1],"w:gz") as t:
  for name,kind in [("../escape", "file"),("Takokit-v0.3.0-linux-x86_64/link", "link")]:
    i=tarfile.TarInfo(name); i.size=1
    if kind=="link": i.type=tarfile.SYMTYPE; i.linkname="/tmp/escape"; i.size=0; t.addfile(i)
    else: t.addfile(i,io.BytesIO(b"x"))
PY
TRAVERSAL_SHA=$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$WORK/web/Takokit-v0.3.0-linux-x86_64.tar.gz")
cat > "$WORK/web/metadata.json" <<EOF
{"version":"0.3.0","platform":"linux","architecture":"x86_64","signing_key_id":"takokit-release-v1","test_fixture":false,"artifact_name":"Takokit-v0.3.0-linux-x86_64.tar.gz","artifact_url":"http://127.0.0.1:$PORT/Takokit-v0.3.0-linux-x86_64.tar.gz","artifact_sha256":"$TRAVERSAL_SHA"}
EOF
expect_failure archive_traversal_and_symlink

FAKE="$WORK/fake-bin"
mkdir -p "$FAKE"
cat > "$FAKE/uname" <<'EOF'
#!/bin/sh
if [ "${1:-}" = -s ]; then echo Plan9; else echo x86_64; fi
EOF
chmod +x "$FAKE/uname"
if PATH="$FAKE:$PATH" sh "$REPO_ROOT/scripts/install.sh" >"$WORK/unsupported-os.out" 2>&1; then
  echo "unsupported OS fixture unexpectedly succeeded" >&2; exit 1
fi

test ! -e "$WORK/escape"
echo "install.sh security fixtures passed"
