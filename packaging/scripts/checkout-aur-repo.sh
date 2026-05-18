#!/usr/bin/env bash
set -euo pipefail

pkg="${1:?usage: checkout-aur-repo.sh <pkg> <dir>}"
dir="${2:?usage: checkout-aur-repo.sh <pkg> <dir>}"
rm -rf "$dir"
git clone "ssh://aur@aur.archlinux.org/${pkg}.git" "$dir"
