#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/slskr-fixture-manifest.XXXXXX")"
trap 'rm -rf "$tmp_root"' EXIT

python3 - "$repo_root/test-data/slskr-test-fixtures" "$tmp_root" <<'PY'
import json, pathlib, shutil, sys
source, target = map(pathlib.Path, sys.argv[1:])
manifest = source / 'meta' / 'manifest.json'
(target / 'meta').mkdir()
shutil.copyfile(manifest, target / 'meta' / 'manifest.json')
for entry in json.loads(manifest.read_text())['files']:
    destination = target / entry['path']
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source / entry['path'], destination)
PY
SLSKR_FIXTURES_ROOT="$tmp_root" "$repo_root/scripts/check-fixture-manifest.sh" >/dev/null

printf 'corrupt fixture\n' >> "$tmp_root/book/treasure_island_pg120.txt"
if SLSKR_FIXTURES_ROOT="$tmp_root" "$repo_root/scripts/check-fixture-manifest.sh" >/dev/null 2>&1; then
  printf 'fixture manifest test failed: corrupt fixture was not rejected\n' >&2
  exit 1
fi

printf 'fixture manifest tests passed\n'
