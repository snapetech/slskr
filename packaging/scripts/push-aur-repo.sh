#!/usr/bin/env bash
set -euo pipefail

dir="${1:?usage: push-aur-repo.sh <dir> <pkg> <message>}"
pkg="${2:?usage: push-aur-repo.sh <dir> <pkg> <message>}"
message="${3:?usage: push-aur-repo.sh <dir> <pkg> <message>}"

cd "$dir"
git config user.email "slskr@proton.me"
git config user.name "slskr"
if git diff --cached --quiet && git diff --quiet; then
  echo "No AUR changes for ${pkg}"
  exit 0
fi
git commit -m "$message"
git push origin HEAD:master
