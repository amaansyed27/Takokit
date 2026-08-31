#!/bin/sh
set -eu

INSTALL_ROOT="${TAKOKIT_INSTALL_ROOT:-$HOME/.local/share/takokit}"
BIN_DIR="${TAKOKIT_BIN_DIR:-$HOME/.local/bin}"
APPLICATIONS_DIR="${TAKOKIT_APPLICATIONS_DIR:-$HOME/Applications}"
DESKTOP_DIR="${TAKOKIT_DESKTOP_DIR:-$HOME/.local/share/applications}"
ICON_DIR="${TAKOKIT_ICON_DIR:-$HOME/.local/share/icons/hicolor/512x512/apps}"
PURGE=0
CONFIRM=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --purge-data) PURGE=1; shift ;;
    --confirm-purge) CONFIRM=$2; shift 2 ;;
    *) echo "unknown uninstall argument: $1" >&2; exit 2 ;;
  esac
done

if [ -x "$INSTALL_ROOT/bin/tako" ]; then "$INSTALL_ROOT/bin/tako" stop >/dev/null 2>&1 || true; fi
for link in "$BIN_DIR/tako" "$BIN_DIR/takokit-server"; do
  if [ -L "$link" ]; then
    target=$(readlink "$link" || true)
    case "$target" in "$INSTALL_ROOT"/*) rm -f -- "$link" ;; esac
  fi
done
desktop="$DESKTOP_DIR/com.dawnlightlabs.takokit.desktop"
if [ -f "$desktop" ] && grep -F 'X-Takokit-Owned=true' "$desktop" >/dev/null 2>&1; then rm -f -- "$desktop"; fi
rm -f -- "$ICON_DIR/takokit.png"
app="$APPLICATIONS_DIR/Takokit.app"
if [ -f "$app/Contents/Info.plist" ] && grep -F 'com.dawnlightlabs.takokit' "$app/Contents/Info.plist" >/dev/null 2>&1; then rm -rf -- "$app"; fi

for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
  [ -f "$rc" ] || continue
  tmp="${rc}.takokit.$$"
  awk '/# >>> Takokit managed PATH >>>/{skip=1;next}/# <<< Takokit managed PATH <<</{skip=0;next}!skip{print}' "$rc" > "$tmp"
  mv -- "$tmp" "$rc"
done
fish="$HOME/.config/fish/conf.d/takokit.fish"
if [ -f "$fish" ] && grep -F '# Takokit managed PATH' "$fish" >/dev/null 2>&1; then rm -f -- "$fish"; fi

expected_default="$HOME/.local/share/takokit"
if [ "$INSTALL_ROOT" != "$expected_default" ] && [ -z "${TAKOKIT_INSTALL_ROOT:-}" ]; then
  echo "refusing unexpected install root: $INSTALL_ROOT" >&2
  exit 1
fi
rm -rf -- "$INSTALL_ROOT"

if [ "$PURGE" -eq 1 ]; then
  [ "$CONFIRM" = "$HOME/.takokit" ] || { echo "data purge requires --confirm-purge '$HOME/.takokit'" >&2; exit 1; }
  rm -rf -- "$HOME/.takokit"
  echo "Takokit application and user runtime data removed. Workspace .tako directories were preserved."
else
  echo "Takokit application removed. User data at $HOME/.takokit and all workspace .tako directories were preserved."
fi
