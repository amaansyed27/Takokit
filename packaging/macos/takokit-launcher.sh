#!/bin/sh
set -eu
exec "${TAKOKIT_INSTALL_ROOT:-$HOME/.local/share/takokit}/bin/tako" gui
