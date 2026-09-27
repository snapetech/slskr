#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixtures_root="${SLSKR_FIXTURES_ROOT:-$repo_root/test-data/slskr-test-fixtures}"
manifest="$fixtures_root/meta/manifest.json"

if [[ ! -f "$manifest" ]]; then
  printf 'fixture manifest check failed: missing %s\n' "$manifest" >&2
  exit 1
fi

python3 - "$fixtures_root" "$manifest" <<'PY'
import hashlib
import json
import re
import sys
from pathlib import Path

fixtures_root = Path(sys.argv[1]).resolve()
manifest_path = Path(sys.argv[2]).resolve()

try:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as error:
    raise SystemExit(f"fixture manifest check failed: {error}")

entries = manifest.get("files")
if not isinstance(entries, list) or not entries:
    raise SystemExit("fixture manifest check failed: files must be a non-empty array")

seen = set()
for entry in entries:
    if not isinstance(entry, dict):
        raise SystemExit("fixture manifest check failed: every files entry must be an object")
    relative = entry.get("path")
    digest = entry.get("sha256")
    expected_bytes = entry.get("bytes")
    if not isinstance(relative, str) or not relative:
        raise SystemExit("fixture manifest check failed: file path is missing")
    relative_path = Path(relative)
    if relative in seen:
        raise SystemExit(f"fixture manifest check failed: duplicate path {relative}")
    seen.add(relative)
    if relative_path.is_absolute() or ".." in relative_path.parts:
        raise SystemExit(f"fixture manifest check failed: unsafe path {relative}")
    if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-fA-F]{64}", digest):
        raise SystemExit(f"fixture manifest check failed: invalid SHA-256 for {relative}")
    if isinstance(expected_bytes, bool) or not isinstance(expected_bytes, int) or expected_bytes < 0:
        raise SystemExit(f"fixture manifest check failed: invalid byte count for {relative}")

    path = fixtures_root / relative_path
    if not path.is_file():
        raise SystemExit(f"fixture manifest check failed: missing file {relative}")
    actual_bytes = path.stat().st_size
    if actual_bytes != expected_bytes:
        raise SystemExit(
            f"fixture manifest check failed: byte count mismatch for {relative}: "
            f"expected {expected_bytes}, got {actual_bytes}"
        )
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(chunk)
    actual_digest = hasher.hexdigest()
    if actual_digest.lower() != digest.lower():
        raise SystemExit(
            f"fixture manifest check failed: SHA-256 mismatch for {relative}: "
            f"expected {digest}, got {actual_digest}"
        )

print(f"fixture manifest check passed: {len(entries)} files")
PY
