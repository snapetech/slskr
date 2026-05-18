#!/usr/bin/env bash
set -euo pipefail

pkgbuild="${1:-PKGBUILD}"

if command -v makepkg >/dev/null 2>&1; then
  (cd "$(dirname "$pkgbuild")" && makepkg --printsrcinfo -p "$(basename "$pkgbuild")")
  exit 0
fi

docker run --rm -v "$(cd "$(dirname "$pkgbuild")" && pwd):/pkg" -w /pkg archlinux:base-devel bash -c "
  pacman -Sy --noconfirm pacman-contrib >/dev/null
  useradd -m builder
  chown -R builder:builder /pkg
  su builder -c 'makepkg --printsrcinfo -p $(printf '%q' "$(basename "$pkgbuild")")'
"
