"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

import csv
from pathlib import Path
from typing import Any
from parity_audit_shared import (
    LIVE_INTEROP_EXPECTED_FAILURE_DETAIL_TOKENS,
    LIVE_INTEROP_PROOF_REQUIREMENTS,
    LIVE_INTEROP_REQUIRED_DETAIL_TOKENS,
)
from parity_audit_live_catalog import (
    live_interop_not_applicable_reason,
)


def live_interop_ledger(
    paths: Path | list[Path],
    expected_failure_checks: frozenset[str] = frozenset(),
) -> dict[tuple[str, str, str], tuple[str, ...]]:
    """Read an explicit, all-green live interop TSV and match only known cases.

    The audit does not run a credentialed live matrix implicitly. Callers must
    opt in with ``--live-interop-evidence`` and point at the exact artifact they
    want to certify. Unknown checks are ignored; known checks are only promoted
    when every requirement for the manifest case is present with ``status=ok``.
    """
    evidence_paths = [paths] if isinstance(paths, Path) else paths
    if not evidence_paths:
        raise SystemExit("live interop evidence requires at least one TSV path")

    observed: dict[str, tuple[str, str]] = {}
    for path in evidence_paths:
        if not path.is_file():
            raise SystemExit(f"live interop evidence file does not exist: {path}")
        with path.open("r", encoding="utf-8", newline="") as handle:
            reader = csv.DictReader(handle, delimiter="\t")
            if reader.fieldnames != ["timestamp", "check", "status", "detail"]:
                raise SystemExit(
                    "live interop evidence must have TSV columns: "
                    "timestamp, check, status, detail"
                )
            for row in reader:
                check = row.get("check", "")
                status = row.get("status", "")
                if not check:
                    raise SystemExit("live interop evidence contains a row without a check name")
                if check in observed:
                    raise SystemExit(f"live interop evidence contains duplicate check: {check}")
                if status not in {"ok", "fail"}:
                    raise SystemExit(f"live interop evidence has invalid status for {check}: {status}")
                observed[check] = (status, row.get("detail", ""))

    def check_is_proven(check: str) -> bool:
        status, detail = observed.get(check, ("", ""))
        if status == "ok":
            return (
                LIVE_INTEROP_REQUIRED_DETAIL_TOKENS.get(check, "") in detail
            )
        expected_failure_token = (
            LIVE_INTEROP_EXPECTED_FAILURE_DETAIL_TOKENS.get(check)
            if check in expected_failure_checks
            else None
        )
        return (
            status == "fail"
            and expected_failure_token is not None
            and expected_failure_token in detail
        )

    return {
        key: requirements
        for key, requirements in LIVE_INTEROP_PROOF_REQUIREMENTS.items()
        if all(check_is_proven(check) for check in requirements)
    }


def live_interop_entries(
    features: list[tuple[str, str]],
    proof_ledger: dict[tuple[str, str, str], tuple[str, ...]] | None = None,
    evidence_paths: list[Path] | None = None,
    expected_failure_checks: frozenset[str] = frozenset(),
    typed_differential_proof: bool = False,
) -> list[dict[str, Any]]:
    entries = []
    for target, feature in features:
        for case in (
            "slskr-initiates-to-target",
            "target-initiates-to-slskr",
            "reconnect-retry-and-resume",
            "malformed-denied-timeout-and-cancel",
            "restart-and-persisted-state",
        ):
            proof_checks = (
                proof_ledger.get((target, feature, case), ())
                if proof_ledger is not None
                else ()
            )
            proven = bool(proof_checks)
            negative_proof = any(
                check in expected_failure_checks for check in proof_checks
            )
            not_applicable_reason = (
                None
                if proven
                else live_interop_not_applicable_reason(target, feature, case)
            )
            entries.append(
                {
                    "id": f"live-interop:{target}:{feature}:{case}",
                    "workstream": "live-interop",
                    "featureFamily": feature,
                    "targets": [target],
                    "surface": "live-interop-case",
                    "subject": feature,
                    "case": case,
                    "status": "complete" if proven or not_applicable_reason else "needs-proof",
                    "coverage": {
                        "targetFeatureInventory": "complete",
                        "liveBehavioralProof": (
                            "complete"
                            if proven
                            else "not-applicable"
                            if not_applicable_reason
                            else "open"
                        ),
                        "typedDifferentialProof": (
                            "complete"
                            if not proven
                            and not_applicable_reason
                            and typed_differential_proof
                            else "not-applicable"
                            if proven or not_applicable_reason is None
                            else "open"
                        ),
                    },
                    **(
                        {
                            "proofMode": "negative-target-contract"
                            if negative_proof
                            else "positive-peer-transaction"
                        }
                        if proven
                        else {}
                    ),
                    **(
                        {"notApplicableReason": not_applicable_reason}
                        if not_applicable_reason
                        else {}
                    ),
                    "evidence": [
                        "docs/live-interop-test-matrix.md",
                        "scripts/run-live-interop-matrix.sh",
                        "scripts/run-slskdn-cross-client-interop.sh",
                    ]
                    + ([str(path) for path in evidence_paths] if proven and evidence_paths else []),
                    "proofChecks": list(proof_checks),
                }
            )
    return entries
