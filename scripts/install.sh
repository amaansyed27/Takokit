#!/bin/sh
set -eu

SITE_ORIGIN="${TAKOKIT_SITE_ORIGIN:-https://takokit.dawnlightlabs.com}"
INSTALL_ROOT="${TAKOKIT_INSTALL_ROOT:-$HOME/.local/share/takokit}"
BIN_DIR="${TAKOKIT_BIN_DIR:-$HOME/.local/bin}"
APPLICATIONS_DIR="${TAKOKIT_APPLICATIONS_DIR:-$HOME/Applications}"
APP_SUPPORT_DIR="${TAKOKIT_APP_SUPPORT_DIR:-$HOME/Library/Application Support/Takokit}"
DESKTOP_DIR="${TAKOKIT_DESKTOP_DIR:-$HOME/.local/share/applications}"
ICON_DIR="${TAKOKIT_ICON_DIR:-$HOME/.local/share/icons/hicolor/512x512/apps}"
PATH_RC=
NEW_ROOT=
BACKUP_ROOT=
INSTALLED=0

fail() { printf '%s\n' "Takokit install failed: $*" >&2; exit 1; }
cleanup() {
  status=$?
  [ -z "${TMP_ROOT:-}" ] || rm -rf -- "$TMP_ROOT"
  if [ "$status" -ne 0 ] && [ "$INSTALLED" -eq 0 ]; then
    [ -z "$NEW_ROOT" ] || rm -rf -- "$NEW_ROOT"
    if [ -n "$BACKUP_ROOT" ] && [ -d "$BACKUP_ROOT" ] && [ ! -e "$INSTALL_ROOT" ]; then
      mv -- "$BACKUP_ROOT" "$INSTALL_ROOT" || true
    fi
  fi
  exit "$status"
}
trap cleanup EXIT HUP INT TERM

case "$(uname -s 2>/dev/null || true)" in
  Linux) PLATFORM=linux ;;
  Darwin) PLATFORM=macos ;;
  *) fail "unsupported operating system" ;;
esac
case "$(uname -m 2>/dev/null || true)" in
  x86_64|amd64) ARCH=x86_64 ;;
  arm64|aarch64) ARCH=arm64 ;;
  *) fail "unsupported architecture" ;;
esac
case "$PLATFORM-$ARCH" in
  linux-x86_64|macos-arm64|macos-x86_64) ;;
  *) fail "$PLATFORM $ARCH is not a published Takokit target" ;;
esac

METADATA_URL="${TAKOKIT_METADATA_URL:-$SITE_ORIGIN/v1/releases/stable/${PLATFORM}-${ARCH}.json}"
is_allowed_url() {
  case "$1" in
    https://*) return 0 ;;
    http://127.0.0.1:*|http://localhost:*) [ "${TAKOKIT_ALLOW_INSECURE_LOOPBACK:-0}" = 1 ] ; return ;;
    *) return 1 ;;
  esac
}
is_allowed_url "$METADATA_URL" || fail "metadata URL must use HTTPS"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/takokit-install.XXXXXX")" || fail "could not create staging directory"
METADATA="$TMP_ROOT/metadata.json"
fetch() {
  url=$1; output=$2
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL --proto '=https' --tlsv1.2 "$url" -o "$output" 2>/dev/null || {
      if [ "${TAKOKIT_ALLOW_INSECURE_LOOPBACK:-0}" = 1 ]; then curl -fsSL "$url" -o "$output"; else return 1; fi
    }
  elif command -v wget >/dev/null 2>&1; then
    wget -q "$url" -O "$output"
  else
    fail "curl or wget is required"
  fi
}
fetch "$METADATA_URL" "$METADATA" || fail "could not download stable release metadata"
[ -s "$METADATA" ] || fail "stable release metadata is empty"

json_string() {
  key=$1
  tr -d '\r\n' < "$METADATA" | sed -n "s/.*\"$key\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p"
}
json_bool() {
  key=$1
  tr -d '\r\n ' < "$METADATA" | sed -n \
    -e "s/.*\"$key\":\(true\).*/\1/p" \
    -e "s/.*\"$key\":\(false\).*/\1/p"
}
VERSION="$(json_string version)"
META_PLATFORM="$(json_string platform)"
META_ARCH="$(json_string architecture)"
KEY_ID="$(json_string signing_key_id)"
TEST_FIXTURE="$(json_bool test_fixture)"
ARTIFACT_NAME="$(json_string artifact_name)"
ARTIFACT_URL="$(json_string artifact_url)"
ARTIFACT_SHA="$(json_string artifact_sha256)"
[ -n "$VERSION" ] && [ -n "$ARTIFACT_NAME" ] && [ -n "$ARTIFACT_URL" ] && [ -n "$ARTIFACT_SHA" ] || fail "stable release metadata is malformed or missing required fields"
[ "$META_PLATFORM" = "$PLATFORM" ] || fail "metadata platform mismatch"
[ "$META_ARCH" = "$ARCH" ] || fail "metadata architecture mismatch"
[ "$KEY_ID" = takokit-release-v1 ] || fail "stable release metadata lacks the production signing identity"
[ "$TEST_FIXTURE" = false ] || fail "stable release metadata points to a test fixture"
case "$VERSION" in *[!0-9A-Za-z.+-]*|'') fail "invalid release version" ;; esac
case "$ARTIFACT_NAME" in "Takokit-v$VERSION-$PLATFORM-$ARCH.tar.gz") ;; *) fail "unexpected artifact name" ;; esac
case "$ARTIFACT_SHA" in *[!0-9a-fA-F]*|'') fail "missing or invalid artifact SHA-256" ;; esac
[ "${#ARTIFACT_SHA}" -eq 64 ] || fail "missing or invalid artifact SHA-256"
is_allowed_url "$ARTIFACT_URL" || fail "artifact URL must use HTTPS"

ARCHIVE="$TMP_ROOT/$ARTIFACT_NAME"
fetch "$ARTIFACT_URL" "$ARCHIVE" || fail "artifact download failed"
if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL_SHA="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  ACTUAL_SHA="$(shasum -a 256 "$ARCHIVE" | awk '{print $1}')"
else
  fail "sha256sum or shasum is required"
fi
[ "$(printf '%s' "$ACTUAL_SHA" | tr 'A-F' 'a-f')" = "$(printf '%s' "$ARTIFACT_SHA" | tr 'A-F' 'a-f')" ] || fail "artifact checksum mismatch; no binary was executed"

tar -tzf "$ARCHIVE" > "$TMP_ROOT/archive.list" || fail "artifact archive is invalid"
while IFS= read -r entry; do
  case "$entry" in
    /*|../*|*/../*|*/..|..|*\\*) fail "archive contains an unsafe path: $entry" ;;
  esac
done < "$TMP_ROOT/archive.list"
if tar -tvzf "$ARCHIVE" | awk 'substr($1,1,1)=="l" || substr($1,1,1)=="h" {found=1} END{exit !found}'; then
  fail "archive contains an unsafe symbolic or hard link"
fi

EXTRACTED="$TMP_ROOT/extracted"
mkdir -p "$EXTRACTED"
tar -xzf "$ARCHIVE" -C "$EXTRACTED" || fail "artifact extraction failed"
PACKAGE_ROOT="$EXTRACTED/Takokit-v$VERSION-$PLATFORM-$ARCH"
[ -x "$PACKAGE_ROOT/bin/tako" ] || fail "verified package is missing executable bin/tako"
FIRST_LINE="$($PACKAGE_ROOT/bin/tako version 2>/dev/null | sed -n '1p')" || fail "verified package binary did not run"
[ "$FIRST_LINE" = "takokit $VERSION" ] || fail "verified package version mismatch"

parent=$(dirname -- "$INSTALL_ROOT")
mkdir -p "$parent" "$BIN_DIR"
NEW_ROOT="${INSTALL_ROOT}.new.$$"
BACKUP_ROOT="${INSTALL_ROOT}.rollback.$$"
rm -rf -- "$NEW_ROOT" "$BACKUP_ROOT"
mv -- "$PACKAGE_ROOT" "$NEW_ROOT"
sed 's/"mode": "portable"/"mode": "installed"/' "$NEW_ROOT/distribution.json" > "$TMP_ROOT/distribution.json"
mv -- "$TMP_ROOT/distribution.json" "$NEW_ROOT/distribution.json"
if [ -e "$INSTALL_ROOT" ]; then mv -- "$INSTALL_ROOT" "$BACKUP_ROOT"; fi
if [ "${TAKOKIT_INSTALL_TEST_FAILPOINT:-}" = after_backup ]; then
  fail "test failpoint after backing up the existing installation"
fi
mv -- "$NEW_ROOT" "$INSTALL_ROOT"
NEW_ROOT=

ln -sfn "$INSTALL_ROOT/bin/tako" "$BIN_DIR/tako"
ln -sfn "$INSTALL_ROOT/bin/takokit-server" "$BIN_DIR/takokit-server"

if [ "$PLATFORM" = linux ]; then
  mkdir -p "$DESKTOP_DIR" "$ICON_DIR"
  escaped=$(printf '%s' "$INSTALL_ROOT/bin/tako" | sed 's/[&|]/\\&/g')
  sed "s|__TAKOKIT_EXEC__|$escaped|" "$INSTALL_ROOT/integrations/linux/Takokit.desktop" > "$DESKTOP_DIR/com.dawnlightlabs.takokit.desktop"
  printf '%s\n' 'X-Takokit-Owned=true' >> "$DESKTOP_DIR/com.dawnlightlabs.takokit.desktop"
  install -m 0644 "$INSTALL_ROOT/integrations/linux/takokit.png" "$ICON_DIR/takokit.png"
else
  APP_SOURCE="$INSTALL_ROOT/integrations/macos/Takokit.app"
  [ -x "$APP_SOURCE/Contents/MacOS/Takokit" ] || fail "verified package is missing native Takokit.app"
  mkdir -p "$APPLICATIONS_DIR" "$APP_SUPPORT_DIR"
  rm -rf -- "$APPLICATIONS_DIR/Takokit.app"
  cp -R "$APP_SOURCE" "$APPLICATIONS_DIR/Takokit.app"
  if command -v codesign >/dev/null 2>&1; then
    codesign --verify --deep --strict "$APPLICATIONS_DIR/Takokit.app" || fail "installed Takokit.app failed code-signature verification"
  fi
  printf '%s\n' "$INSTALL_ROOT" > "$TMP_ROOT/install-root.txt"
  mv -- "$TMP_ROOT/install-root.txt" "$APP_SUPPORT_DIR/install-root.txt"
  printf '%s\n' "$APPLICATIONS_DIR/Takokit.app" > "$TMP_ROOT/application-path.txt"
  mv -- "$TMP_ROOT/application-path.txt" "$APP_SUPPORT_DIR/application-path.txt"
fi

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *)
    case "${SHELL:-}" in
      */zsh) PATH_RC="$HOME/.zshrc" ;;
      */fish)
        PATH_RC="$HOME/.config/fish/conf.d/takokit.fish"
        mkdir -p "$(dirname -- "$PATH_RC")"
        printf '%s\n' '# Takokit managed PATH' "fish_add_path \"$BIN_DIR\"" > "$PATH_RC"
        ;;
      *) PATH_RC="$HOME/.bashrc" ;;
    esac
    case "${SHELL:-}" in
      */fish) ;;
      *)
        touch "$PATH_RC"
        if ! grep -F '# >>> Takokit managed PATH >>>' "$PATH_RC" >/dev/null 2>&1; then
          printf '\n%s\n%s\n%s\n' '# >>> Takokit managed PATH >>>' "export PATH=\"$BIN_DIR:\$PATH\"" '# <<< Takokit managed PATH <<<' >> "$PATH_RC"
        fi
        ;;
    esac
    ;;
esac

printf '%s\n' "{\"product\":\"Takokit\",\"version\":\"$VERSION\",\"platform\":\"$PLATFORM\",\"architecture\":\"$ARCH\",\"install_root\":\"$INSTALL_ROOT\",\"bin_dir\":\"$BIN_DIR\",\"path_rc\":\"$PATH_RC\"}" > "$INSTALL_ROOT/install-receipt.json"
rm -rf -- "$BACKUP_ROOT"
BACKUP_ROOT=
INSTALLED=1
"$INSTALL_ROOT/bin/tako" version >/dev/null || fail "installed tako verification failed"
printf '%s\n' "Takokit $VERSION installed for $PLATFORM $ARCH." "CLI: $BIN_DIR/tako" "Run: tako gui"
