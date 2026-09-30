"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

import json
import tempfile
import time
from pathlib import Path
from parity_audit_shared import (
    PROTOCOL_DIFFERENTIAL_TEST_PREFIX,
)
from parity_audit_process import (
    bounded_slskr_test_command,
    fresh_json_evidence_paths,
    run_logged,
)


def protocol_behaviors_ledger(
    root: Path, reuse_evidence: bool = False
) -> dict[tuple[str, str, str], bool]:
    """Run every protocol-behaviors bulk differential test (the base
    slskr-protocol codec, slskr-client extension/overlay tests, and the
    slskr virtual-Soulfind bridge test, named
    `protocol_behaviors_differential_*` by convention) and union their
    evidence ledgers, keyed by (target, subject, case) where subject is
    `{family}:{name}:{value}` matching protocol_entries()'s own subject
    format. Each such test independently re-verifies a real full-message
    encode/decode round-trip (not just a discriminant or inventory lookup)
    against the codec that owns that protocol family.
    """
    evidence_dir = Path(tempfile.gettempdir()) / "slskr-parity-evidence" / "protocol-behaviors"
    if reuse_evidence:
        if not evidence_dir.is_dir():
            raise RuntimeError(f"reusable protocol evidence is missing: {evidence_dir}")
        ledger: dict[tuple[str, str, str], bool] = {}
        for ledger_path in sorted(evidence_dir.glob("*.json")):
            for row in json.loads(ledger_path.read_text(encoding="utf-8")):
                ledger[(row["target"], row["subject"], row["case"])] = bool(row["pass"])
        return ledger

    evidence_started_ns = time.time_ns()
    run_logged(
        ["cargo", "test", "-p", "slskr-protocol", "--", PROTOCOL_DIFFERENTIAL_TEST_PREFIX],
        cwd=root,
    )
    run_logged(
        [
            "cargo",
            "test",
            "-p",
            "slskr-client",
            "--test",
            "protocol_behaviors_differential",
            "--",
            PROTOCOL_DIFFERENTIAL_TEST_PREFIX,
        ],
        cwd=root,
    )
    run_logged(
        bounded_slskr_test_command(
            "bounded-protocol-tests", PROTOCOL_DIFFERENTIAL_TEST_PREFIX
        ),
        cwd=root,
    )
    ledger: dict[tuple[str, str, str], bool] = {}
    for ledger_path in fresh_json_evidence_paths(evidence_dir, evidence_started_ns):
        rows = json.loads(ledger_path.read_text(encoding="utf-8"))
        for row in rows:
            ledger[(row["target"], row["subject"], row["case"])] = bool(row["pass"])
    return ledger
