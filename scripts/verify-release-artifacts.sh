#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

artifact_dir="${1:-target/dist}"

if [[ ! -d "$artifact_dir" ]]; then
  echo "missing artifact dir: $artifact_dir" >&2
  exit 2
fi

shopt -s nullglob
artifacts=("$artifact_dir"/slskr-*.tar.gz "$artifact_dir"/slskr-*.zip)
if ((${#artifacts[@]} == 0)); then
  echo "no slskr release archives found in $artifact_dir" >&2
  exit 1
fi

for artifact in "${artifacts[@]}"; do
  echo "==> $artifact"
  if [[ -f "$artifact.sha256" ]]; then
    sha256sum -c "$artifact.sha256"
  else
    sha256sum "$artifact"
  fi

  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  case "$artifact" in
    *.tar.gz) tar -C "$tmp" -xzf "$artifact" ;;
    *.zip) python - "$artifact" "$tmp" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1]) as zf:
    zf.extractall(sys.argv[2])
PY
      ;;
  esac

  root="$(find "$tmp" -mindepth 1 -maxdepth 1 -type d | head -1)"
  test -n "$root"
  test -f "$root/README.md"
  test -f "$root/LICENSE"
  test -f "$root/NOTICE"
  test -f "$root/COMPLIANCE.md"
  test -f "$root/docs/slskr.config.example.toml"
  test -f "$root/web/build/index.html"
  if [[ -f "$root/slskr" ]]; then
    chmod +x "$root/slskr"
    "$root/slskr" version
  elif [[ -f "$root/slskr.exe" ]]; then
    echo "Windows executable present: $root/slskr.exe"
  else
    echo "archive does not contain slskr executable" >&2
    exit 1
  fi
  rm -rf "$tmp"
  trap - EXIT
done

echo "release artifacts verified"
