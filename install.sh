#!/usr/bin/env sh
# Install ct from the latest GitHub release.
# Usage: curl -sSL https://raw.githubusercontent.com/apetcu/vibelens/main/install.sh | sh
# Or: curl -sSL https://raw.githubusercontent.com/apetcu/vibelens/main/install.sh | sh -s -- -d /usr/local/bin

set -e

REPO="apetcu/vibelens"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"

while getopts "d:h" opt; do
  case $opt in
    d) BIN_DIR="$OPTARG" ;;
    h) echo "Usage: install.sh [-d DEST_DIR]"; echo "  -d  Install directory (default: \$HOME/.local/bin)"; exit 0 ;;
    *) exit 1 ;;
  esac
done

case "$(uname -s)" in
  Linux)
    case "$(uname -m)" in
      x86_64) ASSET="ct-linux-x86_64" ;;
      *) echo "Unsupported arch: $(uname -m). We only provide x86_64 for Linux. Build from source or use a release asset."; exit 1 ;;
    esac ;;
  Darwin)
    case "$(uname -m)" in
      arm64) ASSET="ct-macos-aarch64" ;;
      x86_64) ASSET="ct-macos-x86_64" ;;
      *) echo "Unsupported arch: $(uname -m). Build from source or use a release asset."; exit 1 ;;
    esac ;;
  *)
    echo "Unsupported OS: $(uname -s). Use the Windows install script or download from Releases."; exit 1 ;;
esac

TAG=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
[ -z "$TAG" ] && echo "Could not get latest release tag." && exit 1

URL="https://github.com/$REPO/releases/download/$TAG/$ASSET"
echo "Installing ct $TAG to $BIN_DIR ..."
mkdir -p "$BIN_DIR"
curl -sSL -o "$BIN_DIR/ct" "$URL"
chmod +x "$BIN_DIR/ct"
echo "Installed: $BIN_DIR/ct"
command -v ct >/dev/null 2>&1 || echo "Add $BIN_DIR to your PATH to run \`ct\` from the shell."
