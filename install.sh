#!/usr/bin/env bash

set -euo pipefail

REPO_URL="https://github.com/Mohamed-Badry/cliphist-cosmic"
BINARY_NAME="cliphist-cosmic"
TARGET="x86_64-unknown-linux-gnu"
TMP_DIR=""

cleanup() {
  if [ -n "${TMP_DIR:-}" ]; then
    rm -rf -- "$TMP_DIR"
  fi
}

trap cleanup EXIT

main() {
  require_linux
  require_x86_64
  require_command curl
  require_command tar

  local version
  version="$(resolve_version "${1:-}")"

  local install_dir
  install_dir="$(default_install_dir)"
  mkdir -p "$install_dir"

  local archive_name
  archive_name="${BINARY_NAME}-${version}-${TARGET}.tar.gz"

  local download_url
  download_url="${REPO_URL}/releases/download/${version}/${archive_name}"

  TMP_DIR="$(mktemp -d)"

  local archive_path
  archive_path="${TMP_DIR}/${archive_name}"

  echo "Downloading ${download_url}"
  curl -fsSL "$download_url" -o "$archive_path"

  tar -xzf "$archive_path" -C "$TMP_DIR"
  install -m 755 "${TMP_DIR}/${BINARY_NAME}" "${install_dir}/${BINARY_NAME}"

  echo "Installed ${BINARY_NAME} ${version} to ${install_dir}/${BINARY_NAME}"

  if ! command -v cliphist >/dev/null 2>&1; then
    echo "Note: cliphist is not installed. The picker expects cliphist to provide clipboard history."
  fi

  if ! command -v wl-copy >/dev/null 2>&1; then
    echo "Note: wl-copy is not installed. Install wl-clipboard for copy support."
  fi

  if ! path_contains "$install_dir"; then
    echo "Note: ${install_dir} is not on PATH in this shell."
  fi
}

resolve_version() {
  local requested_version="${1:-}"
  if [ -n "$requested_version" ]; then
    normalize_version "$requested_version"
    return
  fi

  local latest_url
  latest_url="$(curl -fsSL -o /dev/null -w '%{url_effective}' "${REPO_URL}/releases/latest")"
  normalize_version "${latest_url##*/}"
}

normalize_version() {
  local version="$1"
  case "$version" in
    v*) printf '%s\n' "$version" ;;
    *) printf 'v%s\n' "$version" ;;
  esac
}

default_install_dir() {
  if [ -n "${INSTALL_DIR:-}" ]; then
    printf '%s\n' "${INSTALL_DIR}"
    return
  fi

  if [ "$(id -u)" -eq 0 ]; then
    printf '%s\n' "/usr/local/bin"
  else
    printf '%s\n' "${HOME}/.local/bin"
  fi
}

path_contains() {
  case ":${PATH:-}:" in
    *":$1:"*) return 0 ;;
    *) return 1 ;;
  esac
}

require_linux() {
  local os
  os="$(uname -s)"
  if [ "$os" != "Linux" ]; then
    echo "This installer currently supports Linux only." >&2
    exit 1
  fi
}

require_x86_64() {
  local arch
  arch="$(uname -m)"
  if [ "$arch" != "x86_64" ] && [ "$arch" != "amd64" ]; then
    echo "This installer currently supports x86_64 Linux only." >&2
    exit 1
  fi
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

main "$@"
