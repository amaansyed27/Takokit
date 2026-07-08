#!/usr/bin/env sh
set -eu

echo "Takokit release downloads are not published yet."
echo "This installer is scaffolded for future release distribution and will not download binaries today."
echo

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"

case "$os" in
  linux) target_os="linux" ;;
  darwin) target_os="macos" ;;
  msys*|mingw*|cygwin*) target_os="windows" ;;
  *) target_os="$os" ;;
esac

case "$arch" in
  x86_64|amd64) target_arch="x64" ;;
  arm64|aarch64) target_arch="arm64" ;;
  *) target_arch="$arch" ;;
esac

artifact="takokit-${target_os}-${target_arch}.tar.gz"
release_base_url="https://github.com/amaansyed27/Takokit/releases/latest/download"
future_url="${release_base_url}/${artifact}"

echo "Detected target: ${target_os}-${target_arch}"
echo "Future artifact: ${artifact}"
echo "Future URL: ${future_url}"
echo
echo "Future installer flow:"
echo "  1. Download ${artifact} from GitHub Releases."
echo "  2. Verify a published SHA256 checksum."
echo "  3. Install the takokit binary into ~/.local/bin or /usr/local/bin."
echo "  4. Run: takokit doctor"
echo
echo "No installation was performed."
