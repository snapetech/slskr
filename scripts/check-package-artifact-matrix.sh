#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

for id in BUG-012 BUG-013 BUG-015; do
  if ! rg -n "^\| ${id} \|" "$ledger" >/dev/null; then
    printf 'package artifact matrix check failed: %s is missing from council ledger\n' "$id" >&2
    status=1
  fi
done

for target in \
  x86_64-unknown-linux-gnu \
  x86_64-unknown-linux-musl \
  aarch64-unknown-linux-gnu \
  x86_64-apple-darwin \
  aarch64-apple-darwin \
  x86_64-pc-windows-msvc; do
  if ! rg -n -F "$target" .github/workflows/release.yml scripts/build-release-archive.sh >/dev/null; then
    printf 'package artifact matrix check failed: release target missing: %s\n' "$target" >&2
    status=1
  fi
done

for expected in 'SHA256SUMS.txt' 'sha256sum' 'verify-release-artifacts.sh' 'cargo package' 'verify-cargo-package-contents.sh'; do
  if ! rg -n -F "$expected" .github/workflows scripts >/dev/null; then
    printf 'package artifact matrix check failed: expected packaging token missing: %s\n' "$expected" >&2
    status=1
  fi
done

for expected in \
  'scripts/generate-release-manifests.sh' \
  'slskr-cyclonedx.json' \
  'slskr-dependency-manifest.json' \
  'release/*.json'; do
  if ! rg -n -F "$expected" .github/workflows scripts docs >/dev/null; then
    printf 'package artifact matrix check failed: expected SBOM token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if ! rg -n '^\| BUG-012 .* \| Verified \|$' "$ledger" >/dev/null; then
  printf 'package artifact matrix check failed: BUG-012 must stay verified in council ledger\n' >&2
  status=1
fi

if ! rg -n '^\| BUG-013 .* \| Verified \|$' "$ledger" >/dev/null; then
  printf 'package artifact matrix check failed: BUG-013 must stay verified in council ledger\n' >&2
  status=1
fi

if ! rg -n 'version = "0\.0\.0"' crates/*/Cargo.toml >/dev/null; then
  printf 'package artifact matrix check warning: crate versions no longer show 0.0.0; update BUG-015 status if release metadata is aligned\n'
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'package artifact matrix check passed\n'
