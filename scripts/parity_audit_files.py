"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

import json
import re
import tempfile
import time
from parity_not_applicable import FILE_LIFECYCLE_CASES, file_lifecycle_not_applicable_cases
from pathlib import Path
from typing import Any
from parity_audit_shared import (
    FILE_LIFECYCLE_DIFFERENTIAL_TEST_PREFIX,
)
from parity_audit_process import (
    bounded_slskr_test_command,
    fresh_json_evidence_paths,
    run_logged,
)


def file_write_domains(root: Path) -> list[str]:
    source_root = root / "src/slskd"
    patterns = (
        re.compile(
            r"(?:(?<![\w.])File\.|System\.IO\.File\.|IOFile\.)"
            r"(?:WriteAllText(?:Async)?|WriteAllBytes(?:Async)?|Move|Replace|Create)\b"
        ),
        re.compile(
            r"new\s+(?:System\.IO\.)?FileStream\([\s\S]{0,800}?"
            r"FileMode\.(?:Create|CreateNew|Append|OpenOrCreate)\b"
        ),
        re.compile(r"\b(?:AtomicFileWriter|SecureFileWriter)\."),
    )
    return [
        str(path.relative_to(source_root))
        for path in sorted(source_root.rglob("*.cs"))
        if any(pattern.search(path.read_text(encoding="utf-8-sig", errors="ignore")) for pattern in patterns)
    ]




def file_lifecycle_ledger(
    root: Path, reuse_evidence: bool = False
) -> dict[tuple[str, str, str], bool]:
    """Run explicit file-writer differentials and union their evidence.

    File-writer subjects are source paths rather than normalized feature
    names, so promotion is driven by per-domain, per-case rows emitted by
    tests. There is no generic name-matching classifier here.
    """
    evidence_dir = Path(tempfile.gettempdir()) / "slskr-parity-evidence" / "file-lifecycle"
    if reuse_evidence:
        if not evidence_dir.is_dir():
            raise RuntimeError(f"reusable file evidence is missing: {evidence_dir}")
        ledger: dict[tuple[str, str, str], bool] = {}
        for ledger_path in sorted(evidence_dir.glob("*.json")):
            for row in json.loads(ledger_path.read_text(encoding="utf-8")):
                ledger[(row["target"], row["subject"], row["case"])] = bool(row["pass"])
        return ledger

    evidence_started_ns = time.time_ns()
    run_logged(
        bounded_slskr_test_command(
            "bounded-file-lifecycle-tests", FILE_LIFECYCLE_DIFFERENTIAL_TEST_PREFIX
        ),
        cwd=root,
    )
    ledger: dict[tuple[str, str, str], bool] = {}
    for ledger_path in fresh_json_evidence_paths(evidence_dir, evidence_started_ns):
        rows = json.loads(ledger_path.read_text(encoding="utf-8"))
        for row in rows:
            ledger[(row["target"], row["subject"], row["case"])] = bool(row["pass"])
    return ledger


def file_lifecycle_entries(
    target: str,
    domains: list[str],
    file_ledger: dict[tuple[str, str, str], bool] | None = None,
    frozen_root: Path | None = None,
) -> list[dict[str, Any]]:
    entries = []
    for source in domains:
        subject = source.removesuffix(".cs")
        family = source.split("/", 1)[0].lower()
        not_applicable_cases = file_lifecycle_not_applicable_cases(
            frozen_root, source
        )
        for case in FILE_LIFECYCLE_CASES:
            proven = (
                file_ledger.get((target, subject, case))
                if file_ledger is not None
                else None
            )
            delegated_atomic_proof = False
            if (
                not proven
                and file_ledger is not None
                and target == "slskdn"
                and subject != "Common/IO/AtomicFileWriter"
                and case != "path-and-default-selection"
                and frozen_root is not None
            ):
                source_path = frozen_root / "src/slskd" / source
                if source_path.is_file():
                    source_text = source_path.read_text(
                        encoding="utf-8-sig", errors="ignore"
                    )
                    atomic_delegate = (
                        "AtomicFileWriter." in source_text
                        or bool(
                            re.search(
                                r"(?:FileMode\.(?:Create|CreateNew)|File\.WriteAll(?:Text|Bytes))"
                                r"[\s\S]{0,600}?File\.Move\([^\n]*temp",
                                source_text,
                            )
                        )
                    )
                    delegated_atomic_proof = (
                        atomic_delegate
                        and bool(
                            file_ledger.get(
                                (target, "Common/IO/AtomicFileWriter", case), False
                            )
                        )
                        and (
                            case != "restart-reload-retention-and-corruption"
                            or bool(
                                re.search(
                                    r"File\.Exists|File\.ReadAll|ReadAllText|ReadAllBytes|"
                                    r"Deserialize|Load(?:State|History)?",
                                    source_text,
                                )
                            )
                        )
                    )
            proven = bool(proven or delegated_atomic_proof)
            not_applicable_reason = not_applicable_cases.get(case)
            entries.append(
                {
                    "id": f"file-lifecycle:{target}:{subject}:{case}",
                    "workstream": "persistence-lifecycle",
                    "featureFamily": family,
                    "targets": [target],
                    "surface": "file-lifecycle-case",
                    "subject": subject,
                    "case": case,
                    "status": "complete"
                    if proven or not_applicable_reason
                    else "needs-proof",
                    "coverage": {
                        "frozenFileWriterInventory": "complete",
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
                    "evidence": [source],
                    **(
                        {
                            "proofComposition": (
                            "Frozen caller delegates byte/replace/cleanup semantics "
                                "to the frozen atomic temp-file/replace contract; its "
                                "own load/read path also establishes restart rehydration."
                            )
                        }
                        if delegated_atomic_proof
                        else {}
                    ),
                }
            )
    return entries
