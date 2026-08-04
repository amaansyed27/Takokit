#!/bin/sh
set -eu

REPOSITORY="https://github.com/amaansyed27/Takokit.git"
INSTALL_DIR="${TAKOKIT_INSTALL_DIR:-$HOME/.local/bin}"

case "$(uname -s)" in
  Darwin|Linux) ;;
  *)
    echo "Takokit supports this installer on macOS and Linux." >&2
    exit 1
    ;;
esac

for command_name in git cargo npm; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "Missing required command: $command_name" >&2
    echo "Install Git, Rust stable, Node.js LTS, and npm, then run this command again." >&2
    exit 1
  fi
done

work_dir="$(mktemp -d 2>/dev/null || mktemp -d -t takokit-install)"
cleanup() {
  rm -rf "$work_dir"
}
trap cleanup EXIT INT TERM

echo "Downloading Takokit source..."
git clone --depth 1 "$REPOSITORY" "$work_dir/Takokit"

cd "$work_dir/Takokit/apps/gui"
npm ci
npm run build

cd "$work_dir/Takokit"
cargo build --release --locked

binary="$work_dir/Takokit/target/release/tako"
if [ ! -f "$binary" ]; then
  binary="$work_dir/Takokit/target/release/takokit"
fi
if [ ! -f "$binary" ]; then
  echo "The Takokit binary was not produced by the build." >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
install -m 0755 "$binary" "$INSTALL_DIR/tako"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    echo "Add $INSTALL_DIR to PATH before opening a new terminal." >&2
    ;;
esac

echo "Takokit installed at $INSTALL_DIR/tako"
"$INSTALL_DIR/tako" version
