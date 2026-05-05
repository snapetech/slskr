#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="${SLSKR_COMMONS_FIXTURE_MANIFEST:-$repo_root/fixtures/open-commons/manifest.tsv}"
dest="${1:-${SLSKR_COMMONS_FIXTURE_DIR:-$repo_root/target/open-commons-fixtures}}"
curl_user_agent="${SLSKR_COMMONS_FIXTURE_USER_AGENT:-slskR-test-fixtures/0.1}"

if [[ ! -f "$manifest" ]]; then
  echo "manifest not found: $manifest" >&2
  exit 2
fi

mkdir -p "$dest"

licenses="$dest/LICENSES.tsv"
printf 'filename\tlicense\tlicense_url\tsource_url\tattribution\n' > "$licenses"

while IFS=$'\t' read -r id filename media_type size_bytes sha256 license license_url source_url download_url attribution; do
  if [[ -z "${id:-}" || "${id:0:1}" == "#" ]]; then
    continue
  fi

  tmp="$dest/.${filename}.tmp"
  out="$dest/$filename"

  if [[ -f "$out" ]]; then
    actual_sha="$(sha256sum "$out" | awk '{print $1}')"
    actual_size="$(wc -c < "$out")"
    if [[ "$actual_sha" == "$sha256" && "$actual_size" == "$size_bytes" ]]; then
      printf '%s\t%s\t%s\t%s\t%s\n' "$filename" "$license" "$license_url" "$source_url" "$attribution" >> "$licenses"
      echo "ok $filename"
      continue
    fi
  fi

  rm -f "$tmp"
  curl \
    --fail \
    --location \
    --silent \
    --show-error \
    --retry 6 \
    --retry-delay 2 \
    --retry-all-errors \
    --connect-timeout 20 \
    --max-time 120 \
    --user-agent "$curl_user_agent" \
    --output "$tmp" \
    "$download_url"

  actual_size="$(wc -c < "$tmp")"
  if [[ "$actual_size" != "$size_bytes" ]]; then
    echo "size mismatch for $filename: expected $size_bytes, got $actual_size" >&2
    rm -f "$tmp"
    exit 1
  fi

  actual_sha="$(sha256sum "$tmp" | awk '{print $1}')"
  if [[ "$actual_sha" != "$sha256" ]]; then
    echo "sha256 mismatch for $filename: expected $sha256, got $actual_sha" >&2
    rm -f "$tmp"
    exit 1
  fi

  mv "$tmp" "$out"
  printf '%s\t%s\t%s\t%s\t%s\n' "$filename" "$license" "$license_url" "$source_url" "$attribution" >> "$licenses"
  echo "fetched $filename"
done < "$manifest"

echo "fixtures ready: $dest"
