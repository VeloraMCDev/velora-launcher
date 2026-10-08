#!/usr/bin/env bash
# Install job-local dependencies on Linux Actions runners.
set -euo pipefail

mode="${1:-}"
case "$mode" in
  rust|image|java|release) ;;
  *) echo "Usage: $0 {rust|image|java|release}" >&2; exit 2 ;;
esac
[[ "$(uname -s)" == Linux ]] || { echo 'This setup tool requires Linux.' >&2; exit 1; }

add_path() {
  export PATH="$1:$PATH"
  for path_file in "${GITHUB_PATH:-}" "${GITEA_PATH:-}"; do
    [[ -z "$path_file" ]] || printf '%s\n' "$1" >> "$path_file"
  done
}

install_packages() {
  local apt=()
  local missing=()
  command -v apt-get >/dev/null || { echo 'Automatic system package installation requires apt-get (Ubuntu/Debian runner).' >&2; exit 1; }
  for package in "$@"; do
    dpkg-query -W -f='${Status}' "$package" 2>/dev/null | grep -q 'install ok installed' || missing+=("$package")
  done
  [[ ${#missing[@]} -gt 0 ]] || return 0
  if [[ "$(id -u)" != 0 ]]; then
    command -v sudo >/dev/null || { echo 'System packages require root or passwordless sudo.' >&2; exit 1; }
    sudo -n true || { echo 'Passwordless sudo is required for system packages.' >&2; exit 1; }
    apt=(sudo -n)
  fi
  "${apt[@]}" apt-get update
  DEBIAN_FRONTEND=noninteractive "${apt[@]}" apt-get install -y --no-install-recommends "${missing[@]}"
}

require() {
  command -v "$1" >/dev/null || { echo "Missing $1 after dependency setup." >&2; exit 1; }
}

case "$mode" in
  rust)
    install_packages build-essential curl file libayatana-appindicator3-dev librsvg2-dev libssl-dev libwebkit2gtk-4.1-dev libxdo-dev pkg-config wget
    if ! command -v rustup >/dev/null; then
      installer="$(mktemp)"
      trap 'rm -f "$installer"' EXIT
      curl -fsSL https://sh.rustup.rs -o "$installer"
      sh "$installer" -y --profile minimal --default-toolchain stable
    fi
    add_path "${CARGO_HOME:-$HOME/.cargo}/bin"
    rustup toolchain install stable --profile minimal --component rustfmt --component clippy
    rustup default stable
    require cargo
    require rustc
    require node
    require npm
    [[ "$(node -p 'process.versions.node.split(".")[0]')" -ge 22 ]] || { echo 'Node 22+ is required.' >&2; exit 1; }
    pkg-config --exists webkit2gtk-4.1 ayatana-appindicator3-0.1 librsvg-2.0
    ;;
  image)
    require docker
    docker info >/dev/null || { echo 'Docker daemon is unavailable. Mount the host socket or run a Docker-capable host runner.' >&2; exit 1; }
    docker buildx version >/dev/null || { echo 'Docker Buildx is missing. Install the Docker buildx plugin on the host runner.' >&2; exit 1; }
    ;;
  java)
    require java
    [[ -n "${2:-}" ]] || { echo 'Java major version is required.' >&2; exit 2; }
    java -version 2>&1 | grep -q "${2}\." || { echo "Java $2 is required for this integration build." >&2; exit 1; }
    require bash
    ;;
  release)
    if ! command -v curl >/dev/null || ! command -v jq >/dev/null; then install_packages curl jq; fi
    require curl
    require jq
    ;;
esac
echo "Linux $mode dependencies are ready."
