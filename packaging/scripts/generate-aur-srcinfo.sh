#!/usr/bin/env bash
set -euo pipefail

pkgbuild="${1:-PKGBUILD}"

if command -v makepkg >/dev/null 2>&1; then
  (cd "$(dirname "$pkgbuild")" && makepkg --printsrcinfo -p "$(basename "$pkgbuild")")
  exit 0
fi

pkgdir="$(cd "$(dirname "$pkgbuild")" && pwd)"
cleanup_owner() {
  if command -v sudo >/dev/null 2>&1; then
    sudo chown -R "$(id -u):$(id -g)" "$pkgdir"
  elif [[ "$(id -u)" -eq 0 ]]; then
    chown -R "$(id -u):$(id -g)" "$pkgdir"
  fi
}
trap cleanup_owner EXIT

docker run --rm -v "${pkgdir}:/pkg" -w /pkg archlinux:base-devel bash -c "
  pacman -Sy --noconfirm pacman-contrib >/dev/null
  useradd -m builder
  chown -R builder:builder /pkg
  su builder -c 'makepkg --printsrcinfo -p $(printf '%q' "$(basename "$pkgbuild")")'
"
