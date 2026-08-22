#!/usr/bin/env python3
"""Derive source-bound target capability evidence from a live interop TSV."""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import importlib.util
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
AUDIT_PATH = ROOT / "scripts/audit-parity-manifest.py"
SPEC = importlib.util.spec_from_file_location("audit_parity_manifest", AUDIT_PATH)
assert SPEC and SPEC.loader
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)


def read_tsv(path: Path) -> dict[str, dict[str, str]]:
    if not path.is_file():
        raise SystemExit(f"live interop TSV does not exist: {path}")
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        if reader.fieldnames != ["timestamp", "check", "status", "detail"]:
            raise SystemExit(f"{path} has invalid live interop TSV columns")
        rows: dict[str, dict[str, str]] = {}
        for row in reader:
            check = row.get("check", "")
            if not check or check in rows:
                raise SystemExit(f"{path} contains an invalid or duplicate check: {check}")
            rows[check] = row
        return rows


def source_artifacts(slskdn_root: Path, check_id: str) -> list[str]:
    paths = [slskdn_root / "src/slskd/Application.cs"]
    if check_id == "relay-gateway-bidirectional":
        paths.extend(
            [
                slskdn_root / "src/slskd/Mesh/ServiceFabric/Services/PodsMeshService.cs",
                slskdn_root / "src/slskd/Mesh/ServiceFabric/Services/PrivateGatewayMeshService.cs",
                slskdn_root / "src/slskd/API/Native/PodsController.cs",
            ]
        )
    elif check_id == "virtualsoulfind-bidirectional":
        paths.extend(
            [
                slskdn_root / "src/slskd/Bootstrap/VirtualSoulfindServiceCollectionExtensions.cs",
                slskdn_root / "src/slskd/Core/Options.cs",
                slskdn_root / "src/slskd/VirtualSoulfind/v2/API/VirtualSoulfindV2Controller.cs",
                slskdn_root / "src/slskd/VirtualSoulfind/v2/VirtualSoulfindV2Options.cs",
            ]
        )
    elif check_id in {
        "overlay-udp-bidirectional",
        "overlay-quic-control-bidirectional",
        "quic-data-bidirectional",
    }:
        paths.extend(
            [
                slskdn_root / "src/slskd/PodCore/PeerResolutionService.cs",
                slskdn_root / "src/slskd/PodCore/PodMessageRouter.cs",
                slskdn_root / "src/slskd/PodCore/PodServices.cs",
                slskdn_root / "src/slskd/Mesh/Dht/MeshDhtClient.cs",
                slskdn_root / "src/slskd/Mesh/ServiceFabric/Services/DhtMeshService.cs",
            ]
        )
    missing = [path for path in paths if not path.is_file()]
    if missing:
        raise SystemExit(
            "source-bound capability evidence names missing source artifacts: "
            + ", ".join(str(path) for path in missing)
        )
    return [str(path) for path in paths]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--slskdn-root", type=Path, required=True)
    parser.add_argument("--slskdn-tsv", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    rows = read_tsv(args.slskdn_tsv)
    contracts = AUDIT.frozen_slskdn_transport_not_applicable_contracts(
        args.slskdn_root
    )
    if not contracts:
        raise SystemExit("the supplied frozen slskdN source has no approved capability contracts")

    checks = []
    for check_id, targets in sorted(contracts.items()):
        for target, directions in sorted(targets.items()):
            reasons = {contract["reason"] for contract in directions.values()}
            if len(reasons) != 1:
                raise SystemExit(f"capability contract has inconsistent reasons: {check_id}/{target}")
            evidence_checks = sorted(
                {
                    check
                    for contract in directions.values()
                    for check in contract["evidenceChecks"]
                }
            )
            for evidence_check in evidence_checks:
                row = rows.get(evidence_check)
                if row is None:
                    raise SystemExit(f"capability evidence row is missing: {evidence_check}")
                expected_statuses = {
                    contract["evidenceStatus"]
                    for contract in directions.values()
                    if evidence_check in contract["evidenceChecks"]
                }
                expected_tokens = {
                    contract["evidenceDetailTokens"][evidence_check]
                    for contract in directions.values()
                    if evidence_check in contract["evidenceDetailTokens"]
                }
                if expected_statuses != {row.get("status")}:
                    raise SystemExit(f"capability evidence row has unexpected status: {evidence_check}")
                if not any(token in row.get("detail", "") for token in expected_tokens):
                    raise SystemExit(f"capability evidence row has unexpected detail: {evidence_check}")
            checks.append(
                {
                    "id": check_id,
                    "target": target,
                    "status": "not-applicable",
                    "directions": sorted(directions),
                    "reason": next(iter(reasons)),
                    "evidenceChecks": evidence_checks,
                    "evidenceArtifacts": [str(args.slskdn_tsv)]
                    + source_artifacts(args.slskdn_root, check_id),
                }
            )

    evidence = {
        "schemaVersion": 1,
        "evidenceMode": "live",
        "generatedAt": dt.datetime.now(dt.timezone.utc).isoformat(),
        "derivation": "scripts/derive-universal-transport-capability-evidence.py",
        "target": "slskdn",
        "targetRevision": "65a14a8b821de4df4ab7ef3ab3b156d7206837a3",
        "sourceRoot": str(args.slskdn_root),
        "checks": checks,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(f"transport capability evidence: {len(checks)} contracts; output={args.output}")


if __name__ == "__main__":
    main()
