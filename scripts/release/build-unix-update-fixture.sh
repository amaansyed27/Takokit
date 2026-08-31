#!/usr/bin/env bash
set -euo pipefail

PLATFORM=${1:?usage: build-unix-update-fixture.sh PLATFORM ARCH OUTPUT_ROOT}
ARCH=${2:?}
OUTPUT_ROOT=${3:?}
REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
OUTPUT_ROOT="$(python3 -c 'import os,sys; print(os.path.abspath(sys.argv[1]))' "$OUTPUT_ROOT")"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/takokit-update-fixture.XXXXXX")"
SOURCE="$WORK/source"
TARGET="$WORK/target"
TREE="$WORK/tree"
cleanup() {
  cd "$REPO_ROOT"
  if [ -d "$SOURCE/.git" ] || [ -f "$SOURCE/.git" ]; then git worktree remove --force "$SOURCE" >/dev/null 2>&1 || true; fi
  rm -rf -- "$WORK"
}
trap cleanup EXIT HUP INT TERM
mkdir -p "$OUTPUT_ROOT/test-update"
cd "$REPO_ROOT"
git worktree add --detach "$SOURCE" HEAD
python3 - "$SOURCE/Cargo.toml" <<'PY'
import pathlib,re,sys
p=pathlib.Path(sys.argv[1]); text=p.read_text()
text,count=re.subn(r'(?m)^version\s*=\s*"0\.3\.0"\s*$', 'version = "0.3.1"', text, count=1)
assert count==1
p.write_text(text)
PY
(cd "$SOURCE" && CARGO_TARGET_DIR="$TARGET" TAKOKIT_BUILD_ID="test-update-${PLATFORM}-${ARCH}-0.3.1" cargo build --release --bin tako --bin Takokit --bin takokit-server --bin takokit-updater)
ARCHIVE="$OUTPUT_ROOT/Takokit-v0.3.0-${PLATFORM}-${ARCH}.tar.gz"
mkdir -p "$WORK/base"
tar -xzf "$ARCHIVE" -C "$WORK/base"
cp -R "$WORK/base/Takokit-v0.3.0-${PLATFORM}-${ARCH}" "$TREE"
for binary in tako Takokit takokit-server takokit-updater; do install -m 0755 "$TARGET/release/$binary" "$TREE/bin/$binary"; done
python3 - "$TREE/distribution.json" <<'PY'
import json,pathlib,sys
p=pathlib.Path(sys.argv[1]); value=json.loads(p.read_text()); value.update(version="0.3.1",mode="installed"); p.write_text(json.dumps(value,indent=2)+"\n")
PY
BUNDLE="$OUTPUT_ROOT/test-update/Takokit-v0.3.1-${PLATFORM}-${ARCH}-update.tar.gz"
EPOCH="$(git show -s --format=%ct HEAD)"
python3 scripts/release/create-tar.py --source "$TREE" --output "$BUNDLE" --epoch "$EPOCH"
MANIFEST="$OUTPUT_ROOT/test-update/release-manifest.json"
python3 - "$MANIFEST" "$BUNDLE" "$PLATFORM" "$ARCH" "$(git rev-parse HEAD)" <<'PY'
import hashlib,json,os,sys
path,bundle,platform,arch,sha=sys.argv[1:]; data=open(bundle,'rb').read()
value={"product":"Takokit","version":"0.3.1","channel":"test","commit_sha":sha,"build_id":f"test-update-{platform}-{arch}-0.3.1","build_timestamp":"2026-08-31T00:00:00Z","os":platform,"architecture":arch,"registry_schema_version":1,"storage_schema":{"current":1,"minimum_readable":1,"maximum_readable":1},"minimum_compatible_version":"0.3.0","signing_key_id":"takokit-test-fixture-v1","test_fixture":True,"artifacts":[{"role":"update_bundle","name":os.path.basename(bundle),"size":len(data),"sha256":hashlib.sha256(data).hexdigest()}]}
open(path,'w',encoding='utf-8').write(json.dumps(value,indent=2)+'\n')
PY
"$REPO_ROOT/target/release/takokit-release-tool" sign "$MANIFEST" "$OUTPUT_ROOT/test-update/release-manifest.sig" --test
"$REPO_ROOT/target/release/takokit-release-tool" verify "$MANIFEST" "$OUTPUT_ROOT/test-update/release-manifest.sig" --allow-test
