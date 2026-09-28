"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

import collections
import json
import re
import tempfile
import time
from parity_not_applicable import PERSISTENCE_CASES, persistence_not_applicable_cases
from pathlib import Path
from typing import Any
from parity_audit_shared import (
    PERSISTENCE_DIFFERENTIAL_TEST_PREFIX,
)
from parity_audit_process import (
    bounded_slskr_test_command,
    fresh_json_evidence_paths,
    run_logged,
)


def database_domains(root: Path) -> dict[str, list[str]]:
    source_root = root / "src/slskd"
    domains: dict[str, set[str]] = collections.defaultdict(set)
    internal_tables = {
        "__HashDbMigrations",
        "Messages_fts",
        "filenames_config",
        "filenames_content",
        "filenames_data",
        "filenames_docsize",
        "filenames_idx",
        "version",
    }
    for source_path in sorted(source_root.rglob("*.cs")):
        source = source_path.read_text(encoding="utf-8-sig", errors="ignore")
        source = re.sub(r"/\*.*?\*/", "", source, flags=re.DOTALL)
        source = re.sub(r"//[^\n]*", "", source)
        relative = str(source_path.relative_to(source_root))
        for _entity_type, name in re.findall(r"DbSet<([^>]+)>\s+(\w+)", source):
            if name not in internal_tables:
                domains[name].add(relative)
        for name in re.findall(
            r"CREATE\s+(?:VIRTUAL\s+)?TABLE(?:\s+IF\s+NOT\s+EXISTS)?\s+"
            r"[\[\"']?([A-Za-z_][A-Za-z0-9_]*)",
            source,
            flags=re.IGNORECASE,
        ):
            if name not in internal_tables:
                domains[name].add(relative)
        for name in re.findall(r'\.ToTable\(\s*"([^"]+)"', source):
            if name not in internal_tables:
                domains[name].add(relative)
    return {name: sorted(paths) for name, paths in sorted(domains.items())}




def persistence_lifecycle_ledger(
    root: Path, reuse_evidence: bool = False
) -> dict[tuple[str, str, str], bool]:
    """Run every persistence-lifecycle bulk differential test (crates/slskr/
    src/lib.rs, named `persistence_lifecycle_differential_*` by convention)
    and union their evidence ledgers, keyed by (target, domain, case). Each
    such test independently re-verifies a real create/rehydrate/roundtrip
    behavior for a specific database domain against slskR's own real
    persistence layer, gated on that domain actually mapping (by real table
    name, not guesswork) to one of the frozen oracle's EF Core domains.
    Raises if any differential test fails.
    """
    evidence_dir = Path(tempfile.gettempdir()) / "slskr-parity-evidence" / "persistence-lifecycle"
    if reuse_evidence:
        if not evidence_dir.is_dir():
            raise RuntimeError(f"reusable persistence evidence is missing: {evidence_dir}")
        ledger: dict[tuple[str, str, str], bool] = {}
        for ledger_path in sorted(evidence_dir.glob("*.json")):
            for row in json.loads(ledger_path.read_text(encoding="utf-8")):
                ledger[(row["target"], row["domain"], row["case"])] = bool(row["pass"])
        return ledger

    evidence_started_ns = time.time_ns()
    run_logged(
        bounded_slskr_test_command(
            "bounded-persistence-tests", PERSISTENCE_DIFFERENTIAL_TEST_PREFIX
        ),
        cwd=root,
    )
    ledger: dict[tuple[str, str, str], bool] = {}
    for ledger_path in fresh_json_evidence_paths(evidence_dir, evidence_started_ns):
        rows = json.loads(ledger_path.read_text(encoding="utf-8"))
        for row in rows:
            ledger[(row["target"], row["domain"], row["case"])] = bool(row["pass"])
    return ledger


def persistence_entries(
    target: str,
    domains: dict[str, list[str]],
    persistence_ledger: dict[tuple[str, str, str], bool] | None = None,
    frozen_root: Path | None = None,
) -> list[dict[str, Any]]:
    entries = []
    for domain, sources in domains.items():
        family = sources[0].split("/", 1)[0].lower() if sources else domain.lower()
        not_applicable_cases = persistence_not_applicable_cases(
            frozen_root, domain, sources
        )
        for case in PERSISTENCE_CASES:
            proven = (
                persistence_ledger.get((target, domain, case))
                if persistence_ledger is not None
                else None
            )
            not_applicable_reason = not_applicable_cases.get(case)
            entries.append(
                {
                    "id": f"persistence:{target}:{domain}:{case}",
                    "workstream": "persistence-lifecycle",
                    "featureFamily": family,
                    "targets": [target],
                    "surface": "database-lifecycle-case",
                    "subject": domain,
                    "case": case,
                    "status": "complete" if proven or not_applicable_reason else "needs-proof",
                    "coverage": {
                        "frozenDatabaseInventory": "complete",
                        "behavioralDifferentialOrNotApplicableProof": (
                            "complete"
                            if proven
                            else "not-applicable"
                            if not_applicable_reason
                            else "open"
                        ),
                    },
                    **(
                        {
                            "notApplicableReason": (
                                not_applicable_reason
                            )
                        }
                        if not_applicable_reason
                        else {}
                    ),
                    "evidence": sources,
                }
            )
    return entries
