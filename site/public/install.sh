#!/bin/sh
set -eu

# This public entrypoint stays small; the release tag pins the audited installer body.
SCRIPT_URL="https://raw.githubusercontent.com/amaansyed27/Takokit/v0.3.0/scripts/install.sh"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL --proto '=https' --tlsv1.2 "$SCRIPT_URL" | sh
  exit $?
fi
if command -v wget >/dev/null 2>&1; then
  wget -qO- "$SCRIPT_URL" | sh
  exit $?
fi
echo "Takokit install requires curl or wget." >&2
exit 1
