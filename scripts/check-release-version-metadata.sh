#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

if ! rg -n '^\| BUG-015 .* \| Verified \|$' "$ledger" >/dev/null; then
  printf 'release version metadata check failed: BUG-015 must stay verified in council ledger\n' >&2
  status=1
fi

if ! rg -n -F 'internal/unpublished Cargo crates intentionally remain at `0.0.0`' docs/release.md >/dev/null; then
  printf 'release version metadata check failed: docs/release.md must document internal 0.0.0 crate policy\n' >&2
  status=1
fi

if ! rg -n -F 'SLSKR_RELEASE_VERSION' scripts/build-release-archive.sh .github/workflows/release.yml >/dev/null; then
  printf 'release version metadata check failed: release archive builds must use SLSKR_RELEASE_VERSION\n' >&2
  status=1
fi

if ! rg -n -F 'slskr-$safe_version-$target' scripts/build-release-archive.sh >/dev/null; then
  printf 'release version metadata check failed: release archive root must include sanitized release version and target\n' >&2
  status=1
fi

python3 - <<'PY'
import pathlib
import re
import sys

root = pathlib.Path.cwd()
crate_versions = {}
for manifest in sorted((root / "crates").glob("*/Cargo.toml")):
    text = manifest.read_text(encoding="utf-8")
    name = re.search(r'^name = "([^"]+)"', text, re.M)
    version = re.search(r'^version = "([^"]+)"', text, re.M)
    if not name or not version:
        raise SystemExit(f"{manifest}: missing package name or version")
    crate_versions[name.group(1)] = version.group(1)

if set(crate_versions.values()) != {"0.0.0"}:
    raise SystemExit(f"expected all internal crate versions to remain 0.0.0, got {crate_versions}")

for manifest in sorted((root / "crates").glob("*/Cargo.toml")):
    text = manifest.read_text(encoding="utf-8")
    for dep, version in re.findall(r'(slskr(?:-[a-z]+)?) = \{ version = "([^"]+)", path = "[^"]+" \}', text):
        if version != crate_versions.get(dep):
            raise SystemExit(f"{manifest}: internal dependency {dep} version {version} does not match package version {crate_versions.get(dep)}")

release_doc = (root / "docs/release.md").read_text(encoding="utf-8")
if "release-v<semver>" not in release_doc:
    raise SystemExit("docs/release.md must keep release-v<semver> as the artifact version source")
PY

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'release version metadata check passed\n'
