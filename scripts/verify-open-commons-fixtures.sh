#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="${SLSKR_COMMONS_FIXTURE_MANIFEST:-$repo_root/fixtures/open-commons/manifest.tsv}"
dest="${1:-${SLSKR_COMMONS_FIXTURE_DIR:-$repo_root/target/open-commons-fixtures}}"

"$repo_root/scripts/fetch-open-commons-fixtures.sh" "$dest" >/dev/null

if [[ ! -f "$dest/LICENSES.tsv" ]]; then
  echo "missing fixture license summary: $dest/LICENSES.tsv" >&2
  exit 1
fi

checked=0
while IFS=$'\t' read -r id filename media_type size_bytes sha256 license license_url source_url download_url attribution; do
  if [[ -z "${id:-}" || "${id:0:1}" == "#" ]]; then
    continue
  fi

  path="$dest/$filename"
  if [[ ! -f "$path" ]]; then
    echo "missing fixture: $path" >&2
    exit 1
  fi

  actual_size="$(wc -c < "$path")"
  if [[ "$actual_size" != "$size_bytes" ]]; then
    echo "size mismatch for $filename: expected $size_bytes, got $actual_size" >&2
    exit 1
  fi

  actual_sha="$(sha256sum "$path" | awk '{print $1}')"
  if [[ "$actual_sha" != "$sha256" ]]; then
    echo "sha256 mismatch for $filename: expected $sha256, got $actual_sha" >&2
    exit 1
  fi

  if ! grep -Fq "$filename" "$dest/LICENSES.tsv"; then
    echo "license summary missing fixture: $filename" >&2
    exit 1
  fi

  checked=$((checked + 1))
done < "$manifest"

echo "verified $checked open commons fixtures in $dest"
