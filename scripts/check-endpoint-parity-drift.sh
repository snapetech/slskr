#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

upstream_repo="${SLSKR_UPSTREAM_GIT_REPO:-$repo_root/../slskdn}"
slskd_ref="${SLSKR_SLSKD_REF:-16e5d86ec9a91120f3ef40b85cb22036566b788a}"
slskdn_ref="${SLSKR_SLSKDN_REF:-65a14a8b821de4df4ab7ef3ab3b156d7206837a3}"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/slskr-webui-endpoint-audit.XXXXXX")"
report="$work_dir/report.json"

cleanup() {
  rm -rf "$work_dir"
}
trap cleanup EXIT

mkdir -p "$work_dir/slskd" "$work_dir/slskdn"
git -C "$upstream_repo" archive "$slskd_ref" src/web | tar -x -C "$work_dir/slskd"
git -C "$upstream_repo" archive "$slskdn_ref" src/web | tar -x -C "$work_dir/slskdn"

if ! node scripts/audit-upstream-webui-endpoints.mjs \
  --slskd-root "$work_dir/slskd" \
  --slskdn-root "$work_dir/slskdn" \
  --slskr-web-root "$repo_root" \
  --fail-on-unresolved \
  --fail-on-missing \
  --json >"$report"; then
  node - "$report" <<'NODE'
const { readFileSync } = require('node:fs');
const report = JSON.parse(readFileSync(process.argv[2], 'utf8'));
for (const endpoint of report.comparison.missingFromSlskrWebCalls) {
  process.stderr.write(`missing WebUI call: ${endpoint}\n`);
}
for (const target of ['slskd', 'slskdn', 'slskr']) {
  for (const row of report[target].unresolved) {
    process.stderr.write(`unresolved ${target} call: ${row.file}:${row.line} ${row.method}\n`);
  }
}
NODE
  exit 1
fi

node - "$report" <<'NODE'
const { readFileSync } = require('node:fs');
const report = JSON.parse(readFileSync(process.argv[2], 'utf8'));
process.stdout.write(
  `endpoint parity drift check passed: ${report.slskd.endpoints.length} slskd + ` +
    `${report.slskdn.endpoints.length} slskdN calls; union ${report.comparison.targetUnionCount}\n`,
);
NODE
