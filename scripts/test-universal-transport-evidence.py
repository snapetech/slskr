#!/usr/bin/env python3
"""Small bounded regression test for transport-evidence derivation."""

from __future__ import annotations

import csv
import json
import importlib.util
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
MODULE_PATH = ROOT / "scripts/derive-universal-transport-evidence.py"
SPEC = importlib.util.spec_from_file_location("derive_transport_evidence", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def write_tsv(path: Path, rows: list[str]) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(("timestamp", "check", "status", "detail"))
        for check in rows:
            writer.writerow(("2026-08-17T00:00:00Z", check, "ok", "exact live transaction"))


def write_rows(path: Path, rows: list[tuple[str, str, str]]) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(("timestamp", "check", "status", "detail"))
        for check, status, detail in rows:
            writer.writerow(("2026-08-17T00:00:00Z", check, status, detail))


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="slskr-transport-evidence-test-") as directory:
        root = Path(directory)
        slskd = root / "slskd.tsv"
        slskdn = root / "slskdn.tsv"
        slskdn_supplement = root / "slskdn-supplement.tsv"
        lifecycle = root / "lifecycle.json"
        lifecycle_artifact = root / "lifecycle-case.json"
        capability = root / "capability.json"
        lifecycle_artifact.write_text("{}\n", encoding="utf-8")
        write_tsv(
            slskd,
            [
                "protocol-slskr-message-dispatch-slskd",
                "protocol-slskd-message-dispatch",
                "slskr-to-slskd-download",
                "slskd-to-slskr-download",
                "protocol-slskr-distributed-peer-slskd",
            ],
        )
        write_tsv(
            slskdn,
            [
                "protocol-slskr-message-dispatch",
                "protocol-slskdn-message-dispatch",
                "slskr-to-slskdn-download",
                "slskdn-to-slskr-download",
                "protocol-slskr-distributed-peer-slskdn",
                "protocol-slskdn-distributed-peer-slskr",
                "protocol-slskr-obfuscated-peer-slskdn",
                "protocol-slskdn-obfuscated-peer-slskr",
                "protocol-slskr-overlay-udp-slskdn",
                "protocol-slskr-overlay-quic-control-slskdn",
                "protocol-slskr-quic-data-slskdn",
            ],
        )
        write_rows(
            slskdn,
            [
                ("protocol-slskr-message-dispatch", "ok", "exact live transaction"),
                ("protocol-slskdn-message-dispatch", "ok", "exact live transaction"),
                ("slskr-to-slskdn-download", "ok", "exact live transaction"),
                ("slskdn-to-slskr-download", "ok", "exact live transaction"),
                ("protocol-slskr-distributed-peer-slskdn", "ok", "exact live transaction"),
                ("protocol-slskdn-distributed-peer-slskr", "ok", "exact live transaction"),
                ("protocol-slskr-obfuscated-peer-slskdn", "ok", "exact live transaction"),
                ("protocol-slskdn-obfuscated-peer-slskr", "ok", "exact live transaction"),
                ("protocol-slskr-overlay-udp-slskdn", "ok", "exact live transaction"),
                (
                    "protocol-slskr-overlay-quic-control-slskdn",
                    "ok",
                    "exact live transaction",
                ),
                ("protocol-slskr-quic-data-slskdn", "ok", "exact live transaction"),
                ("protocol-ksdn-probe-dispatch", "ok", "exact live transaction"),
                ("protocol-ksdn-slskr-receives-ack", "ok", "exact live transaction"),
                ("protocol-ksdn-slskr-verifies-slskdn-descriptor", "ok", "exact live transaction"),
                ("protocol-ksdn-slskdn-receives-hello", "ok", "exact live transaction"),
                ("protocol-ksdn-slskdn-persists-slskr-descriptor", "ok", "exact live transaction"),
                (
                    "protocol-slskdn-overlay-udp-slskr",
                    "ok",
                    "expected-target-negative endpoint-resolution-unavailable",
                ),
                (
                    "protocol-slskdn-overlay-quic-control-slskr",
                    "ok",
                    "expected-target-negative endpoint-resolution-unavailable",
                ),
                (
                    "protocol-slskdn-quic-data-slskr",
                    "ok",
                    "expected-target-negative endpoint-resolution-unavailable",
                ),
                ("protocol-slskr-gateway-open-slskdn", "fail", "Service 'private-gateway' not found"),
                ("protocol-slskr-gateway-send-slskdn", "fail", "gateway tunnel was not opened"),
                ("protocol-slskr-gateway-receive-slskdn", "fail", "echo payload unavailable"),
                ("protocol-slskr-gateway-close-slskdn", "fail", "gateway tunnel was not opened"),
                ("protocol-slskr-gateway-pod-join-slskdn", "fail", "Service 'pods' not found"),
                ("runtime-slskdn-virtualsoulfind-v2-create", "ok", "status=503 body=VirtualSoulfind v2 is disabled"),
            ],
        )
        write_rows(
            slskdn_supplement,
            [
                (
                    "protocol-ksdn-mesh-sync-reconnect-retry",
                    "fail",
                    'expected-target-negative status=400 body={"error":"Failed to sync with peer"} target_attempts=400,400 replacement_attempts=400,400',
                )
            ],
        )
        scenarios = list(MODULE.LIFECYCLE_SCENARIOS)
        cases = [
            {
                "target": target,
                "scenario": scenario,
                "status": "pass",
                "evidenceArtifacts": [str(lifecycle_artifact)],
            }
            for target in MODULE.LIFECYCLE_TARGETS
            for scenario in scenarios
        ]
        lifecycle.write_text(
            json.dumps(
                {
                    "id": MODULE.LIFECYCLE_CHECK,
                    "evidenceMode": "live",
                    "status": "pass",
                    "targets": list(MODULE.LIFECYCLE_TARGETS),
                    "scenarios": scenarios,
                    "targetScenarios": {target: scenarios for target in MODULE.LIFECYCLE_TARGETS},
                    "evidenceArtifacts": [str(lifecycle_artifact)],
                    "cases": cases,
                }
            )
            + "\n",
            encoding="utf-8",
        )
        capability.write_text(
            json.dumps(
                {
                    "schemaVersion": 1,
                    "evidenceMode": "live",
                    "checks": [
                        {
                            "id": "relay-gateway-bidirectional",
                            "target": "slskdn",
                            "status": "not-applicable",
                            "directions": ["slskr-to-target", "target-to-slskr"],
                            "reason": "frozen-target-private-gateway-service-not-registered",
                            "evidenceChecks": [
                                "protocol-slskr-gateway-open-slskdn",
                                "protocol-slskr-gateway-send-slskdn",
                                "protocol-slskr-gateway-receive-slskdn",
                                "protocol-slskr-gateway-close-slskdn",
                                "protocol-slskr-gateway-pod-join-slskdn",
                            ],
                            "evidenceArtifacts": [str(slskdn)],
                        },
                        {
                            "id": "virtualsoulfind-bidirectional",
                            "target": "slskdn",
                            "status": "not-applicable",
                            "directions": ["slskr-to-target", "target-to-slskr"],
                            "reason": "frozen-target-virtualsoulfind-v2-controller-options-unbound",
                            "evidenceChecks": ["runtime-slskdn-virtualsoulfind-v2-create"],
                            "evidenceArtifacts": [str(slskdn)],
                        },
                        {
                            "id": "overlay-udp-bidirectional",
                            "target": "slskdn",
                            "status": "not-applicable",
                            "directions": ["target-to-slskr"],
                            "reason": "frozen-target-overlay-peer-resolution-unwired",
                            "evidenceChecks": ["protocol-slskdn-overlay-udp-slskr"],
                            "evidenceArtifacts": [str(slskdn)],
                        },
                        {
                            "id": "overlay-quic-control-bidirectional",
                            "target": "slskdn",
                            "status": "not-applicable",
                            "directions": ["target-to-slskr"],
                            "reason": "frozen-target-overlay-peer-resolution-unwired",
                            "evidenceChecks": ["protocol-slskdn-overlay-quic-control-slskr"],
                            "evidenceArtifacts": [str(slskdn)],
                        },
                        {
                            "id": "quic-data-bidirectional",
                            "target": "slskdn",
                            "status": "not-applicable",
                            "directions": ["target-to-slskr"],
                            "reason": "frozen-target-overlay-peer-resolution-unwired",
                            "evidenceChecks": ["protocol-slskdn-quic-data-slskr"],
                            "evidenceArtifacts": [str(slskdn)],
                        },
                    ],
                }
            )
            + "\n",
            encoding="utf-8",
        )
        evidence = MODULE.derive(
            {"slskd": slskd, "slskdn": slskdn},
            lifecycle,
            capability,
            {"slskdn": [slskdn_supplement]},
        )
        checks = {check["id"]: check for check in evidence["checks"]}
        assert checks["soulseek-peer-bidirectional"]["status"] == "pass"
        assert checks["file-stream-transfer-bidirectional"]["status"] == "pass"
        assert checks["distributed-dht-bidirectional"]["status"] == "fail"
        assert checks["distributed-dht-bidirectional"]["targetDirections"]["slskd"] == [
            "slskr-to-target"
        ]
        assert checks["distributed-dht-bidirectional"]["targetDirections"]["slskdn"] == [
            "slskr-to-target",
            "target-to-slskr",
        ]
        assert checks["obfuscated-peer-bidirectional"]["status"] == "pass"
        assert checks["overlay-udp-bidirectional"]["status"] == "pass"
        assert checks["overlay-quic-control-bidirectional"]["status"] == "pass"
        assert checks["quic-data-bidirectional"]["status"] == "pass"
        assert checks["mesh-sync-bidirectional"]["status"] == "pass"
        assert checks["mesh-sync-bidirectional"]["lifecycleStatus"] == "pass"
        assert checks["relay-gateway-bidirectional"]["status"] == "pass"
        assert checks["relay-gateway-bidirectional"]["notApplicableDirections"]["slskdn"] == [
            "slskr-to-target",
            "target-to-slskr",
        ]
        assert checks["virtualsoulfind-bidirectional"]["status"] == "pass"
        assert checks[MODULE.LIFECYCLE_CHECK]["status"] == "pass"


if __name__ == "__main__":
    main()
