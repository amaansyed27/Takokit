#!/usr/bin/env bash
set -euo pipefail

VERSION="${TAKOKIT_VERSION:-0.3.0}"
OUTPUT_ROOT="${TAKOKIT_OUTPUT_ROOT:-}"
SKIP_BUILD=0
ALLOW_DIRTY=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --output) OUTPUT_ROOT="$2"; shift 2 ;;
    --skip-build) SKIP_BUILD=1; shift ;;
    --allow-dirty) ALLOW_DIRTY=1; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
cd "$REPO_ROOT"
[ "$VERSION" = "0.3.0" ] || { echo "Slice 6 candidates are locked to 0.3.0" >&2; exit 1; }

case "$(uname -s)" in
  Linux) PLATFORM=linux ;;
  Darwin) PLATFORM=macos ;;
  *) echo "unsupported packaging host: $(uname -s)" >&2; exit 1 ;;
esac
case "$(uname -m)" in
  x86_64|amd64) ARCH=x86_64 ;;
  arm64|aarch64) ARCH=arm64 ;;
  *) echo "unsupported packaging architecture: $(uname -m)" >&2; exit 1 ;;
esac
if [ "$PLATFORM" = linux ] && [ "$ARCH" != x86_64 ]; then
  echo "Linux $ARCH is not an advertised Slice 6 target" >&2
  exit 1
fi

COMMIT_SHA="$(git rev-parse HEAD)"
if [ "$ALLOW_DIRTY" -ne 1 ] && [ -n "$(git status --porcelain)" ]; then
  echo "refusing to package a dirty source tree; commit first or pass --allow-dirty" >&2
  exit 1
fi
COMMIT_EPOCH="$(git show -s --format=%ct HEAD)"
BUILD_TIMESTAMP="$(python3 -c 'import datetime,sys; print(datetime.datetime.fromtimestamp(int(sys.argv[1]), datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"))' "$COMMIT_EPOCH")"
BUILD_ID="${TAKOKIT_BUILD_ID:-${PLATFORM}-${ARCH}-v${VERSION}-${COMMIT_SHA}}"
OUTPUT_ROOT="${OUTPUT_ROOT:-$REPO_ROOT/dist/$PLATFORM-$ARCH}"

if [ "$SKIP_BUILD" -ne 1 ]; then
  cargo build --release --locked --bin tako --bin Takokit --bin takokit-server --bin takokit-updater
  cargo build --release --locked -p takokit-release --bin takokit-release-tool
  (cd apps/gui && npm ci && npm run build)
fi

for required in target/release/tako target/release/Takokit target/release/takokit-server target/release/takokit-updater target/release/takokit-release-tool apps/gui/dist/index.html registry/index.json LICENSE; do
  [ -f "$required" ] || { echo "missing release input: $required" >&2; exit 1; }
done
[ -n "${TAKOKIT_WHISPERCPP_BUNDLE:-}" ] && [ -d "$TAKOKIT_WHISPERCPP_BUNDLE" ] || {
  echo "TAKOKIT_WHISPERCPP_BUNDLE must name the pinned native whisper.cpp runtime directory" >&2
  exit 1
}
[ -x "$TAKOKIT_WHISPERCPP_BUNDLE/whisper-cli" ] || { echo "whisper.cpp bundle has no executable whisper-cli" >&2; exit 1; }

rm -rf -- "$OUTPUT_ROOT"
mkdir -p "$OUTPUT_ROOT"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/takokit-build.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT HUP INT TERM
BASE="$WORK/base"
mkdir -p "$BASE/bin" "$BASE/resources/licenses" "$BASE/resources/icons"
install -m 0755 target/release/tako "$BASE/bin/tako"
install -m 0755 target/release/Takokit "$BASE/bin/Takokit"
install -m 0755 target/release/takokit-server "$BASE/bin/takokit-server"
install -m 0755 target/release/takokit-updater "$BASE/bin/takokit-updater"
cp -R apps/gui/dist "$BASE/resources/gui"
cp -R registry "$BASE/resources/registry"
cp -R runners "$BASE/resources/runners"
cp -RL "$TAKOKIT_WHISPERCPP_BUNDLE" "$BASE/resources/runners/whispercpp-runtime"
install -m 0644 assets/transparent-png/512.png "$BASE/resources/icons/takokit.png"
install -m 0644 LICENSE "$BASE/resources/licenses/LICENSE.txt"
python3 scripts/release/generate_dependency_notices.py --output "$BASE/resources/licenses/THIRD_PARTY_NOTICES.md"
install -m 0755 scripts/uninstall.sh "$BASE/uninstall.sh"

mkdir -p "$BASE/integrations/linux" "$BASE/integrations/macos/Takokit.app/Contents/MacOS" "$BASE/integrations/macos/Takokit.app/Contents/Resources"
sed "s|@TAKOKIT_EXEC@|__TAKOKIT_EXEC__|g" packaging/linux/Takokit.desktop > "$BASE/integrations/linux/Takokit.desktop"
install -m 0644 assets/transparent-png/512.png "$BASE/integrations/linux/takokit.png"
sed "s/@VERSION@/$VERSION/g" packaging/macos/Info.plist > "$BASE/integrations/macos/Takokit.app/Contents/Info.plist"
install -m 0755 packaging/macos/takokit-launcher.sh "$BASE/integrations/macos/Takokit.app/Contents/MacOS/Takokit"
install -m 0644 assets/transparent-png/512.png "$BASE/integrations/macos/Takokit.app/Contents/Resources/Takokit.png"
if [ "$PLATFORM" = macos ]; then
  codesign --force --deep --sign - "$BASE/integrations/macos/Takokit.app"
  codesign --verify --deep --strict "$BASE/integrations/macos/Takokit.app"
  cat > "$OUTPUT_ROOT/apple-signing-status.json" <<EOF
{"artifact":"Takokit.app","status":"ad-hoc signed","developer_id":false,"notarized":false}
EOF
fi

python3 - "$BASE/build-provenance.json" "$VERSION" "$COMMIT_SHA" "$BUILD_ID" "$BUILD_TIMESTAMP" "$PLATFORM" "$ARCH" <<'PY'
import json,sys
path,version,sha,build_id,timestamp,platform,arch=sys.argv[1:]
with open(path,"w",encoding="utf-8") as f:
  json.dump({"product":"Takokit","version":version,"commit_sha":sha,"build_id":build_id,"build_timestamp":timestamp,"source_tree_dirty":False,"os":platform,"architecture":arch,"registry_schema_version":1,"storage_schema_version":1},f,indent=2)
  f.write("\n")
PY

materialize() {
  destination="$1"; mode="$2"
  cp -R "$BASE" "$destination"
  stable="https://github.com/amaansyed27/Takokit/releases/latest/download/release-manifest-${PLATFORM}-${ARCH}.json"
  preview="https://github.com/amaansyed27/Takokit/releases/download/preview/release-manifest-${PLATFORM}-${ARCH}.json"
  python3 - "$destination/distribution.json" "$VERSION" "$mode" "$stable" "$preview" <<'PY'
import json,sys
path,version,mode,stable,preview=sys.argv[1:]
with open(path,"w",encoding="utf-8") as f:
  json.dump({"product":"Takokit","version":version,"mode":mode,"install_root":None,"update_manifest_url":stable,"update_manifest_urls":{"stable":stable,"preview":preview},"default_channel":"stable"},f,indent=2)
  f.write("\n")
PY
  python3 - "$destination/release-metadata.json" "$VERSION" "$COMMIT_SHA" "$BUILD_ID" "$BUILD_TIMESTAMP" "$mode" "$PLATFORM" "$ARCH" <<'PY'
import json,sys
path,version,sha,build_id,timestamp,mode,platform,arch=sys.argv[1:]
with open(path,"w",encoding="utf-8") as f:
  json.dump({"product":"Takokit","version":version,"commit_sha":sha,"build_id":build_id,"build_timestamp":timestamp,"distribution_mode":mode,"platform":platform,"architecture":arch,"portable":mode=="portable"},f,indent=2)
  f.write("\n")
PY
}

INSTALLED="$WORK/installed"
PORTABLE="$WORK/portable"
materialize "$INSTALLED" installed
materialize "$PORTABLE" portable
PACKAGE_NAME="Takokit-v${VERSION}-${PLATFORM}-${ARCH}"
python3 scripts/release/create-tar.py --source "$PORTABLE" --output "$OUTPUT_ROOT/$PACKAGE_NAME.tar.gz" --root "$PACKAGE_NAME" --epoch "$COMMIT_EPOCH"
python3 scripts/release/create-tar.py --source "$INSTALLED" --output "$OUTPUT_ROOT/$PACKAGE_NAME-update.tar.gz" --epoch "$COMMIT_EPOCH"

PORTABLE_PATH="$OUTPUT_ROOT/$PACKAGE_NAME.tar.gz"
UPDATE_PATH="$OUTPUT_ROOT/$PACKAGE_NAME-update.tar.gz"
SIGNING_KEY_ID=takokit-test-fixture-v1
CHANNEL=test
TEST_FIXTURE=true
if [ -n "${TAKOKIT_RELEASE_SIGNING_KEY_HEX:-}" ]; then
  SIGNING_KEY_ID=takokit-release-v1
  CHANNEL=stable
  TEST_FIXTURE=false
fi
MANIFEST="$OUTPUT_ROOT/release-manifest-${PLATFORM}-${ARCH}.json"
python3 - "$MANIFEST" "$VERSION" "$COMMIT_SHA" "$BUILD_ID" "$BUILD_TIMESTAMP" "$PLATFORM" "$ARCH" "$SIGNING_KEY_ID" "$CHANNEL" "$TEST_FIXTURE" "$PORTABLE_PATH" "$UPDATE_PATH" <<'PY'
import hashlib,json,os,sys
path,version,sha,build_id,timestamp,platform,arch,key,channel,test_fixture,portable,update=sys.argv[1:]
def artifact(role,p):
  data=open(p,"rb").read()
  return {"role":role,"name":os.path.basename(p),"size":len(data),"sha256":hashlib.sha256(data).hexdigest()}
value={"product":"Takokit","version":version,"channel":channel,"commit_sha":sha,"build_id":build_id,"build_timestamp":timestamp,"os":platform,"architecture":arch,"registry_schema_version":1,"storage_schema":{"current":1,"minimum_readable":1,"maximum_readable":1},"minimum_compatible_version":"0.2.0","signing_key_id":key,"test_fixture":test_fixture=="true","artifacts":[artifact("portable",portable),artifact("update_bundle",update)]}
with open(path,"w",encoding="utf-8") as f: json.dump(value,f,indent=2); f.write("\n")
PY
SIG="$OUTPUT_ROOT/release-manifest-${PLATFORM}-${ARCH}.sig"
if [ "$TEST_FIXTURE" = true ]; then
  target/release/takokit-release-tool sign "$MANIFEST" "$SIG" --test
  target/release/takokit-release-tool verify "$MANIFEST" "$SIG" --allow-test
else
  target/release/takokit-release-tool sign "$MANIFEST" "$SIG"
  target/release/takokit-release-tool verify "$MANIFEST" "$SIG"
fi
python3 - "$OUTPUT_ROOT/SHA256SUMS.txt" "$PORTABLE_PATH" "$UPDATE_PATH" "$MANIFEST" "$SIG" <<'PY'
import hashlib,os,sys
output,*paths=sys.argv[1:]
with open(output,"w",encoding="utf-8") as f:
  for path in paths:
    f.write(f"{hashlib.sha256(open(path,'rb').read()).hexdigest()}  {os.path.basename(path)}\n")
PY
cp "$BASE/build-provenance.json" "$OUTPUT_ROOT/build-provenance-${PLATFORM}-${ARCH}.json"
python3 - "$OUTPUT_ROOT/build-summary.json" "$VERSION" "$COMMIT_SHA" "$PLATFORM" "$ARCH" "$PORTABLE_PATH" "$UPDATE_PATH" "$MANIFEST" "$SIG" "$TEST_FIXTURE" <<'PY'
import json,sys
path,version,sha,platform,arch,portable,update,manifest,sig,test=sys.argv[1:]
with open(path,"w",encoding="utf-8") as f: json.dump({"version":version,"commit_sha":sha,"platform":platform,"architecture":arch,"portable":portable,"update_bundle":update,"manifest":manifest,"signature":sig,"test_fixture":test=="true"},f,indent=2); f.write("\n")
PY
echo "built $PACKAGE_NAME from $COMMIT_SHA in $OUTPUT_ROOT"
