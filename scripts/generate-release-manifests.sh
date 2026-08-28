#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

out_dir="${1:-target/dist}"
release_version="${2:-${SLSKR_RELEASE_VERSION:-unknown}}"
mkdir -p "$out_dir"

OUT_DIR="$out_dir" RELEASE_VERSION="$release_version" python3 - <<'PY'
import ast
import json
import os
import pathlib
import re
import tomllib

root = pathlib.Path.cwd()
out_dir = pathlib.Path(os.environ["OUT_DIR"])
release_version = os.environ["RELEASE_VERSION"]

components = []


def add_component(ecosystem, name, version, path=None, source=None, license_value=None):
    if not name or not version:
        return
    purl_type = {
        "cargo": "cargo",
        "npm": "npm",
        "go": "golang",
        "python": "pypi",
    }.get(ecosystem, ecosystem)
    component = {
        "type": "library",
        "group": ecosystem,
        "name": name,
        "version": version,
        "purl": f"pkg:{purl_type}/{name}@{version}",
    }
    if path:
        component["evidence"] = {"identity": {"field": "manifest", "confidence": 1.0, "methods": [{"technique": "manifest-analysis", "value": path}]}}
    if source:
        component["externalReferences"] = [{"type": "distribution", "url": source}]
    if license_value:
        component["licenses"] = [{"license": {"id": license_value}}]
    components.append(component)


def cargo_components():
    lock_path = root / "Cargo.lock"
    if not lock_path.exists():
        return
    data = tomllib.loads(lock_path.read_text(encoding="utf-8"))
    for package in data.get("package", []):
        add_component(
            "cargo",
            package.get("name"),
            package.get("version"),
            "Cargo.lock",
            package.get("source"),
        )


def npm_components(lockfile):
    path = root / lockfile
    if not path.exists():
        return
    data = json.loads(path.read_text(encoding="utf-8"))
    for package_path, package in sorted(data.get("packages", {}).items()):
        if not package_path or "node_modules/" not in package_path:
            continue
        name = package.get("name") or package_path.rsplit("node_modules/", 1)[-1]
        add_component(
            "npm",
            name,
            package.get("version"),
            lockfile,
            package.get("resolved"),
            package.get("license") if isinstance(package.get("license"), str) else None,
        )


def go_components():
    path = root / "client-go/go.sum"
    if not path.exists():
        return
    seen = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        parts = line.split()
        if len(parts) < 2 or parts[1].endswith("/go.mod"):
            continue
        key = (parts[0], parts[1])
        if key in seen:
            continue
        seen.add(key)
        add_component("go", parts[0], parts[1], "client-go/go.sum")


def python_components():
    path = root / "client-python/setup.py"
    if not path.exists():
        return
    text = path.read_text(encoding="utf-8")
    version_match = re.search(r'name="([^"]+)".*?version="([^"]+)"', text, re.S)
    if version_match:
        add_component("python", version_match.group(1), version_match.group(2), "client-python/setup.py")
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call):
            continue
        for keyword in node.keywords:
            if keyword.arg != "install_requires" or not isinstance(keyword.value, ast.List):
                continue
            for item in keyword.value.elts:
                if not isinstance(item, ast.Constant) or not isinstance(item.value, str):
                    continue
                requirement = item.value
                name = re.split(r"[<>=!~ ]", requirement, maxsplit=1)[0]
                version = requirement[len(name):].lstrip("<>=!~ ") or "unspecified"
                add_component("python", name, version, "client-python/setup.py")


cargo_components()
for lockfile in ["package-lock.json", "web/package-lock.json", "dashboard/package-lock.json", "client-ts/package-lock.json"]:
    npm_components(lockfile)
go_components()
python_components()

components.sort(key=lambda item: (item["group"], item["name"].lower(), item["version"]))

bom = {
    "bomFormat": "CycloneDX",
    "specVersion": "1.5",
    "serialNumber": "urn:uuid:00000000-0000-0000-0000-000000000000",
    "version": 1,
    "metadata": {
        "component": {
            "type": "application",
            "name": "slskr",
            "version": release_version,
        },
        "tools": [
            {
                "vendor": "slskr",
                "name": "scripts/generate-release-manifests.sh",
            }
        ],
    },
    "components": components,
}

grouped = {}
for component in components:
    grouped.setdefault(component["group"], []).append(
        {
            "name": component["name"],
            "version": component["version"],
            "purl": component["purl"],
        }
    )

manifest = {
    "name": "slskr",
    "version": release_version,
    "source": "checked-in lockfiles and package metadata",
    "component_count": len(components),
    "ecosystems": grouped,
}

(out_dir / "slskr-cyclonedx.json").write_text(json.dumps(bom, indent=2, sort_keys=True) + "\n", encoding="utf-8")
(out_dir / "slskr-dependency-manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

printf '%s\n' "$out_dir/slskr-cyclonedx.json"
printf '%s\n' "$out_dir/slskr-dependency-manifest.json"
