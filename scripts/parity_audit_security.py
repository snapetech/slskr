"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

import json
import re
import tempfile
import time
from parity_not_applicable import security_not_applicable_cases
from pathlib import Path
from typing import Any
from parity_audit_shared import (
    SECURITY_CONTROL_CASES,
    SECURITY_CONTROL_DIFFERENTIAL_TEST_PREFIX,
)
from parity_audit_process import (
    bounded_slskr_test_command,
    fresh_json_evidence_paths,
    run_logged,
)


def security_components(root: Path) -> list[str]:
    source_root = root / "src/slskd"
    security_name = re.compile(
        r"Security|Auth|RateLimit|Csp|Csrf|Cors|Token|Certificate|Blacklist|"
        r"Blocklist|Ban|Permission|Policy",
        flags=re.IGNORECASE,
    )
    return [
        str(path.relative_to(source_root))
        for path in sorted(source_root.rglob("*.cs"))
        if security_name.search(str(path.relative_to(source_root)))
    ]




def security_control_ledger(
    root: Path, reuse_evidence: bool = False
) -> dict[tuple[str, str, str], bool]:
    """Run explicit security-control differentials and union their evidence.

    Security components are intentionally not promoted from source-name
    matching. A row is complete only when the focused Rust differential emits
    a passing case for the exact frozen target/component/case tuple.
    """
    evidence_dir = Path(tempfile.gettempdir()) / "slskr-parity-evidence" / "security-controls"
    if reuse_evidence:
        if not evidence_dir.is_dir():
            raise RuntimeError(f"reusable security-control evidence is missing: {evidence_dir}")
        ledger: dict[tuple[str, str, str], bool] = {}
        for ledger_path in sorted(evidence_dir.glob("*.json")):
            for row in json.loads(ledger_path.read_text(encoding="utf-8")):
                ledger[(row["target"], row["subject"], row["case"])] = bool(row["pass"])
        return ledger

    evidence_started_ns = time.time_ns()
    run_logged(
        bounded_slskr_test_command(
            "bounded-security-control-tests", SECURITY_CONTROL_DIFFERENTIAL_TEST_PREFIX
        ),
        cwd=root,
    )
    ledger: dict[tuple[str, str, str], bool] = {}
    for ledger_path in fresh_json_evidence_paths(evidence_dir, evidence_started_ns):
        rows = json.loads(ledger_path.read_text(encoding="utf-8"))
        for row in rows:
            ledger[(row["target"], row["subject"], row["case"])] = bool(row["pass"])
    return ledger


def security_component_entries(
    target: str,
    components: list[str],
    security_ledger: dict[tuple[str, str, str], bool] | None = None,
    frozen_root: Path | None = None,
) -> list[dict[str, Any]]:
    entries = []
    for source in components:
        subject = source.removesuffix(".cs")
        family = source.split("/", 1)[0].lower()
        not_applicable_cases = security_not_applicable_cases(frozen_root, source)
        for case in SECURITY_CONTROL_CASES:
            proven = (
                security_ledger.get((target, subject, case))
                if security_ledger is not None
                else None
            )
            not_applicable_reason = not_applicable_cases.get(case)
            entries.append(
                {
                    "id": f"security-component:{target}:{subject}:{case}",
                    "workstream": "security-controls",
                    "featureFamily": family,
                    "targets": [target],
                    "surface": "security-control-case",
                    "subject": subject,
                    "case": case,
                    "status": "complete" if proven or not_applicable_reason else "needs-proof",
                    "coverage": {
                        "frozenSecurityComponentInventory": "complete",
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
                    "evidence": source,
                }
            )
    return entries
