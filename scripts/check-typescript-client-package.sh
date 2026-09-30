#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

npm --prefix client-ts run build
if ! git diff --exit-code -- client-ts/dist; then
  printf 'TypeScript package check failed: tracked dist is not reproducible from source\n' >&2
  exit 1
fi

manifest="$(mktemp)"
trap 'rm -f "$manifest"' EXIT
(cd client-ts && npm pack --dry-run --json --ignore-scripts > "$manifest")

node - "$manifest" <<'NODE'
const fs = require('fs');
const manifest = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const packageData = Array.isArray(manifest) ? manifest[0] : manifest[Object.keys(manifest)[0]];
const files = (packageData?.files ?? []).map((entry) => entry.path);
const invalid = files.filter((path) => (
  path !== 'README.md' && path !== 'package.json' && !path.startsWith('dist/')
));
if (invalid.length > 0) {
  console.error(`TypeScript package check failed: unexpected packed files: ${invalid.join(', ')}`);
  process.exit(1);
}
if (!files.some((path) => path === 'dist/index.js') || !files.some((path) => path === 'dist/index.d.ts')) {
  console.error('TypeScript package check failed: package entrypoints are missing');
  process.exit(1);
}
console.log(`TypeScript package check passed: ${files.length} allowlisted files`);
NODE
