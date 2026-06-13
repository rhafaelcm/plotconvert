#!/usr/bin/env bash
set -euo pipefail

REPO="rhafaelcm/plotconvert"
ASSET="plotconvert-linux-x86_64"
INSTALL_PATH="/usr/bin/plotconvert"
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"

if ! command -v curl >/dev/null 2>&1; then
  echo "error: curl is required" >&2
  exit 1
fi

if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
  echo "error: run as root to install to ${INSTALL_PATH} (e.g. sudo $0)" >&2
  exit 1
fi

tmp_file="$(mktemp)"
trap 'rm -f "$tmp_file"' EXIT

echo "Downloading latest ${ASSET}..."
curl -fsSL -o "$tmp_file" "$DOWNLOAD_URL"

if [[ ! -s "$tmp_file" ]]; then
  echo "error: downloaded file is empty" >&2
  exit 1
fi

install -m 755 "$tmp_file" "$INSTALL_PATH"

echo "Installed to ${INSTALL_PATH}"
"${INSTALL_PATH}" --version
