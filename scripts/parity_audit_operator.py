"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

import collections
import json
from parity_not_applicable import operator_not_applicable_cases
from pathlib import Path
from typing import Any


def operator_families(root: Path) -> dict[str, list[str]]:
    families: dict[str, set[str]] = collections.defaultdict(set)

    dockerfile = root / "Dockerfile"
    if dockerfile.is_file():
        families["container-root"].add("Dockerfile")

    workflow_root = root / ".github/workflows"
    if workflow_root.is_dir():
        for path in sorted(workflow_root.glob("*.y*ml")):
            families[f"github-workflow-{path.stem}"].add(str(path.relative_to(root)))

    packaging_root = root / "packaging"
    if packaging_root.is_dir():
        for path in sorted(packaging_root.rglob("*")):
            if path.is_file():
                relative = path.relative_to(root)
                families[f"packaging-{relative.parts[1]}"].add(str(relative))

    systemd_root = root / "etc/systemd"
    if systemd_root.is_dir():
        for child in sorted(systemd_root.rglob("*")):
            if child.is_file():
                families["systemd-hardened"].add(str(child.relative_to(root)))

    nix_file = root / "flake.nix"
    if nix_file.is_file():
        families["nix-root"].add("flake.nix")

    vpn_root = root / "src/slskdN.VpnAgent"
    if vpn_root.is_dir():
        for child in sorted(vpn_root.rglob("*")):
            if child.is_file() and (
                child.name == "install.sh" or "systemd" in child.relative_to(vpn_root).parts
            ):
                families["vpn-agent"].add(str(child.relative_to(root)))

    return {family: sorted(paths) for family, paths in sorted(families.items())}


def operator_packaging_ledger(path: Path) -> dict[tuple[str, str, str], bool]:
    """Read explicit operator-packaging evidence emitted by the artifact audit.

    The artifact audit intentionally emits failed rows as well as passing rows;
    this loader rejects malformed or duplicate evidence and only promotes an
    exact target/family/case tuple when its row is explicitly green.
    """
    rows = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(rows, list):
        raise ValueError(f"operator evidence must be a JSON array: {path}")
    ledger: dict[tuple[str, str, str], bool] = {}
    for row in rows:
        if not isinstance(row, dict):
            raise ValueError(f"operator evidence row must be an object: {path}")
        key = (row.get("target"), row.get("subject"), row.get("case"))
        if any(not isinstance(value, str) for value in key):
            raise ValueError(f"operator evidence row has invalid identity: {row!r}")
        if key in ledger:
            raise ValueError(f"duplicate operator evidence row: {key!r}")
        if not isinstance(row.get("pass"), bool):
            raise ValueError(f"operator evidence row has invalid pass value: {row!r}")
        ledger[key] = row["pass"]
    return ledger




def operator_entries(
    target: str,
    families: dict[str, list[str]],
    operator_ledger: dict[tuple[str, str, str], bool] | None = None,
    frozen_root: Path | None = None,
) -> list[dict[str, Any]]:
    entries = []
    for family, sources in families.items():
        not_applicable_cases = operator_not_applicable_cases(
            frozen_root, family, sources
        )
        for case in (
            "build-render-and-artifact-contents",
            "fresh-install-and-upgrade",
            "start-stop-signal-and-restart",
            "configuration-user-permissions-and-secrets",
            "network-ports-storage-and-health",
            "failure-rollback-uninstall-and-logs",
        ):
            proven = (
                operator_ledger.get((target, family, case))
                if operator_ledger is not None
                else None
            )
            not_applicable_reason = not_applicable_cases.get(case)
            entries.append(
                {
                    "id": f"operator:{target}:{family}:{case}",
                    "workstream": "operator-packaging",
                    "featureFamily": family,
                    "targets": [target],
                    "surface": "operator-family-case",
                    "subject": family,
                    "case": case,
                    "status": "complete"
                    if proven or not_applicable_reason
                    else "needs-proof",
                    "coverage": {
                        "frozenOperatorArtifactInventory": "complete",
                        "behavioralDifferentialOrNotApplicableProof": (
                            "complete"
                            if proven
                            else "not-applicable"
                            if not_applicable_reason
                            else "open"
                        ),
                    },
                    **(
                        {"notApplicableReason": not_applicable_reason}
                        if not_applicable_reason
                        else {}
                    ),
                    "evidence": sources,
                }
            )
    return entries
