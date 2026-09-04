#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python3 - "$repo_root" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
lockfiles = (
    pathlib.Path("web/package-lock.json"),
    pathlib.Path("dashboard/package-lock.json"),
    pathlib.Path("client-ts/package-lock.json"),
)
allowed_deprecated = {}
errors = []
deprecated = []

for lockfile in lockfiles:
    path = root / lockfile
    try:
        document = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        errors.append(f"{lockfile}: cannot read package lock: {error}")
        continue

    for package_path, package in document.get("packages", {}).items():
        reason = package.get("deprecated")
        if reason is None:
            continue
        deprecated.append((lockfile, package_path, package.get("version", "unknown"), reason))
        if not package.get("dev", False):
            errors.append(f"{lockfile}:{package_path}: deprecated runtime dependency")
        if package_path not in allowed_deprecated.get(lockfile, set()):
            errors.append(f"{lockfile}:{package_path}: undocumented deprecated dependency")

manifest = json.loads((root / "web/package.json").read_text())
dev_dependencies = manifest.get("devDependencies", {})
if "eslint-config-canonical" in dev_dependencies:
    errors.append("web/package.json: unused eslint-config-canonical must not return")
for package_name in ("@vitest/eslint-plugin", "eslint-plugin-react"):
    if package_name not in dev_dependencies:
        errors.append(
            f"web/package.json: eslint config imports {package_name} without a direct dependency"
        )

client_manifest = json.loads((root / "client-ts/package.json").read_text())
if client_manifest.get("overrides", {}).get("glob") != "^13.0.6":
    errors.append("client-ts/package.json: Jest glob must stay on the supported 13.x line")

documentation = (root / "docs/dev/npm-dependency-hygiene.md").read_text()
if "client-ts/package.json` overrides Jest's test-exclude glob to `^13.0.6`" not in documentation:
    errors.append("docs/dev/npm-dependency-hygiene.md: Jest glob override is not documented")

if errors:
    print("npm dependency hygiene check failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    raise SystemExit(1)

if deprecated:
    for lockfile, package_path, version, reason in deprecated:
        print(
            f"documented dev-only deprecated dependency: "
            f"{lockfile}:{package_path}@{version} ({reason})"
        )
else:
    print("no deprecated package entries remain in the frontend lockfiles")
print("npm dependency hygiene check passed")
PY
