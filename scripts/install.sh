#!/usr/bin/env sh
set -eu

REPO="gODtECH-Ctl-Create/StackPilot"
VERSION="${STACKPILOT_VERSION:-latest}"
INSTALL_DIR="${STACKPILOT_INSTALL_DIR:-$HOME/.local/bin}"

os_name="$(uname -s)"
arch_name="$(uname -m)"

case "$os_name" in
  Linux) os="linux" ;;
  Darwin) os="macos" ;;
  *)
    echo "StackPilot does not publish an installer binary for $os_name yet." >&2
    exit 1
    ;;
esac

case "$arch_name" in
  x86_64|amd64) arch="x86_64" ;;
  arm64|aarch64) arch="aarch64" ;;
  *)
    echo "Unsupported CPU architecture: $arch_name" >&2
    exit 1
    ;;
esac

if [ "$os" = "linux" ] && [ "$arch" = "aarch64" ]; then
  echo "Linux arm64 binaries are not published yet. Build StackPilot from source instead." >&2
  exit 1
fi

asset="stackpilot-${os}-${arch}.tar.gz"
if [ "$VERSION" = "latest" ]; then
  url="https://github.com/${REPO}/releases/latest/download/${asset}"
  release_label="latest release"
else
  case "$VERSION" in
    v*) tag="$VERSION" ;;
    *) tag="v$VERSION" ;;
  esac
  url="https://github.com/${REPO}/releases/download/${tag}/${asset}"
  release_label="$tag"
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

printf 'Installing StackPilot (%s/%s) from %s\n' "$os" "$arch" "$release_label"
if ! curl --fail --silent --show-error --location "$url" --output "$tmp_dir/$asset"; then
  cat >&2 <<EOF

StackPilot installation failed because the release asset '$asset' could not be downloaded.

Requested: $url

This usually means the selected GitHub release does not contain a binary for this platform yet.
Check the published release assets at:
https://github.com/${REPO}/releases

To install a specific release, set STACKPILOT_VERSION first, for example:
STACKPILOT_VERSION=0.1.2
EOF
  exit 1
fi

if ! tar -xzf "$tmp_dir/$asset" -C "$tmp_dir"; then
  echo "The downloaded StackPilot archive could not be extracted." >&2
  exit 1
fi

if [ ! -f "$tmp_dir/stackpilot" ]; then
  echo "The downloaded archive did not contain the stackpilot binary. The release package may be invalid." >&2
  exit 1
fi
if [ ! -f "$tmp_dir/recipes/base/recipe.toml" ]; then
  echo "The downloaded archive did not contain StackPilot recipes. The release package may be invalid." >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
install -m 0755 "$tmp_dir/stackpilot" "$INSTALL_DIR/stackpilot"
rm -rf "$INSTALL_DIR/recipes"
cp -R "$tmp_dir/recipes" "$INSTALL_DIR/recipes"

printf 'Installed StackPilot to %s/stackpilot\n' "$INSTALL_DIR"
printf 'Installed recipes to %s/recipes\n' "$INSTALL_DIR"
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *) printf 'Add %s to PATH to run stackpilot from any directory.\n' "$INSTALL_DIR" ;;
esac
