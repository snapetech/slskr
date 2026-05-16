#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

artifact_dir="${1:-target/dist}"

sha256_digest() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{ print $1 }'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{ print $1 }'
  elif command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 -r "$file" | awk '{ print $1 }'
  else
    echo "no SHA-256 command found; install sha256sum, shasum, or openssl" >&2
    return 1
  fi
}

verify_sha256_file() {
  local file="$1"
  local checksum_file="$file.sha256"
  if [[ ! -f "$checksum_file" ]]; then
    printf '%s  %s\n' "$(sha256_digest "$file")" "$file"
    return
  fi

  local expected
  local actual
  expected="$(awk 'NR == 1 { print $1 }' "$checksum_file")"
  actual="$(sha256_digest "$file")"
  if [[ "$expected" != "$actual" ]]; then
    echo "$file: FAILED" >&2
    echo "expected $expected" >&2
    echo "actual   $actual" >&2
    return 1
  fi
  echo "$file: OK"
}

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
  verify_sha256_file "$artifact"

  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  case "$artifact" in
    *.tar.gz) python - "$artifact" "$tmp" <<'PY'
import pathlib
import sys
import tarfile

destination = pathlib.Path(sys.argv[2]).resolve()

with tarfile.open(sys.argv[1], "r:gz") as tf:
    for member in tf.getmembers():
        member_path = pathlib.PurePosixPath(member.name)
        if member_path.is_absolute() or ".." in member_path.parts:
            raise SystemExit(f"unsafe tar entry path: {member.name}")
        if member.issym() or member.islnk():
            raise SystemExit(f"tar links are not allowed: {member.name}")
        if not (member.isfile() or member.isdir()):
            raise SystemExit(f"tar special files are not allowed: {member.name}")
        target = (destination / pathlib.Path(*member_path.parts)).resolve()
        if target != destination and destination not in target.parents:
            raise SystemExit(f"tar entry escapes destination: {member.name}")
    tf.extractall(destination)
PY
      ;;
    *.zip) python - "$artifact" "$tmp" <<'PY'
import pathlib
import sys
import zipfile

destination = pathlib.Path(sys.argv[2]).resolve()

with zipfile.ZipFile(sys.argv[1]) as zf:
    for member in zf.infolist():
        member_path = pathlib.PurePosixPath(member.filename)
        if member_path.is_absolute() or ".." in member_path.parts:
            raise SystemExit(f"unsafe zip entry path: {member.filename}")
        target = (destination / pathlib.Path(*member_path.parts)).resolve()
        if target != destination and destination not in target.parents:
            raise SystemExit(f"zip entry escapes destination: {member.filename}")
    zf.extractall(destination)
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
  test -d "$root/web/build/assets"
  find "$root/web/build/assets" -type f -name '*.js' | grep -q .
  find "$root/web/build/assets" -type f -name '*.css' | grep -q .
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
