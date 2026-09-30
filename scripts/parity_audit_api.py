"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any
from parity_audit_shared import (
    CONTROLLER_API_DIFFERENTIAL_TEST_PREFIX,
    CONTROLLER_API_TEST_FEATURES,
    SECURITY_AUTHORIZATION_TEST,
)
from parity_audit_process import (
    bounded_slskr_test_command,
    feature_family,
    fresh_json_evidence_paths,
    run_logged,
)


def security_authorization_ledger(
    root: Path, reuse_evidence: bool = False
) -> dict[tuple[str, str, str, str], bool]:
    """Run the exhaustive in-process auth-gate differential (crates/slskr/src/lib.rs)
    and return real, freshly executed pass/fail evidence keyed by
    (target, method, route, case). This is the only source that may promote a
    security-authorization manifest case out of ``needs-proof`` -- the test
    itself proves the live dispatcher (`route_http_request`'s `check_route_auth`
    gate) against the declared policy tables for all 10 credential profiles.
    Raises if the differential test fails: a real enforcement regression must
    fail manifest generation, not silently look like unlinked evidence.
    """
    ledger_path = Path(tempfile.gettempdir()) / "slskr-parity-evidence" / "security-authorization.json"
    evidence_started_ns: int | None = None
    if not reuse_evidence:
        evidence_started_ns = time.time_ns()
        run_logged(
            bounded_slskr_test_command(
                "bounded-security-authorization-tests",
                ["--exact", SECURITY_AUTHORIZATION_TEST],
            ),
            cwd=root,
        )
        if not ledger_path.is_file() or ledger_path.stat().st_mtime_ns < evidence_started_ns:
            raise RuntimeError(
                "fresh security evidence is missing or predates the current differential run: "
                f"{ledger_path}"
            )
    if not ledger_path.is_file():
        raise RuntimeError(f"reusable security evidence is missing: {ledger_path}")
    rows = json.loads(ledger_path.read_text(encoding="utf-8"))
    return {
        (row["target"], row["method"], row["route"], row["case"]): bool(row["pass"])
        for row in rows
    }


def controller_api_ledger(
    root: Path,
    slskd_root: Path,
    slskdn_root: Path,
    reuse_evidence: bool = False,
) -> dict[tuple[str, str, str, str], bool]:
    """Run every controller-api bulk differential test (crates/slskr/src/lib.rs,
    named `controller_api_differential_*` by convention) and union their
    evidence ledgers, keyed by (target, method, route, case). Each such test
    proves a real, executed behavioral case (not route presence alone) for a
    specific route family -- e.g. malformed/missing-id contract behavior for
    the UUID-guarded families `versioned_get_failure_contract` already
    enforces in production. New tests just need the same name prefix and to
    write their own file under the shared evidence directory; no changes
    here are needed to pick them up. Raises if any differential test fails:
    a real behavioral regression must fail manifest generation.
    """
    evidence_dir = Path(tempfile.gettempdir()) / "slskr-parity-evidence" / "controller-api"
    ledger: dict[tuple[str, str, str, str], bool] = {}
    if reuse_evidence:
        if not evidence_dir.is_dir():
            raise RuntimeError(f"reusable controller evidence is missing: {evidence_dir}")
        for ledger_path in sorted(evidence_dir.glob("*.json")):
            rows = json.loads(ledger_path.read_text(encoding="utf-8"))
            for row in rows:
                ledger[(row["target"], row["method"], row["route"], row["case"])] = bool(
                    row["pass"]
                )
        return ledger

    evidence_started_ns = time.time_ns()
    for feature in CONTROLLER_API_TEST_FEATURES:
        run_logged(
            bounded_slskr_test_command(feature, CONTROLLER_API_DIFFERENTIAL_TEST_PREFIX),
            cwd=root,
        )
    ledger = {}
    for ledger_path in fresh_json_evidence_paths(evidence_dir, evidence_started_ns):
        rows = json.loads(ledger_path.read_text(encoding="utf-8"))
        for row in rows:
            ledger[(row["target"], row["method"], row["route"], row["case"])] = bool(
                row["pass"]
            )

    # The route-presence case is deliberately kept separate from behavioral
    # evidence above.  The existing frozen-snapshot controller gate
    # materializes every declared route against a local slskR daemon and
    # distinguishes a real handler response from generic router fallthrough,
    # HTML fallback, or the compatibility-operation shell.  It proves only
    # presence; the remaining cases still require the differential tests
    # above and below.
    audit_dir = Path(tempfile.mkdtemp(prefix="slskr-controller-manifest-"))
    try:
        environment = os.environ.copy()
        environment.update(
            {
                "SLSKR_CONTROLLER_AUDIT_DIR": str(audit_dir),
                "SLSKR_CONTROLLER_AUDIT_KEEP": "1",
                "SLSKR_UPSTREAM_GIT_REPO": os.environ.get(
                    "SLSKR_UPSTREAM_GIT_REPO", str(slskdn_root)
                ),
                "SLSKR_SLSKD_REF": "16e5d86ec9a91120f3ef40b85cb22036566b788a",
                "SLSKR_SLSKDN_REF": "65a14a8b821de4df4ab7ef3ab3b156d7206837a3",
            }
        )
        subprocess.run(
            ["bash", "scripts/check-slskdn-controller-parity.sh"],
            cwd=root,
            check=True,
            env=environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        route_presence = []
        for target, report_name in (
            ("slskdn", "controller-audit.json"),
            ("slskd", "slskd-controller-audit.json"),
        ):
            report = json.loads((audit_dir / report_name).read_text(encoding="utf-8"))
            for row in report:
                route_presence.append(
                    {
                        "target": target,
                        "method": row["method"],
                        "route": row["route"],
                        "case": "route-presence",
                        "pass": row.get("result") == "handled",
                    }
                )
        presence_path = evidence_dir / "route_presence_frozen_snapshot.json"
        presence_path.write_text(
            json.dumps(route_presence, indent=2) + "\n", encoding="utf-8"
        )
        for row in route_presence:
            ledger[(row["target"], row["method"], row["route"], row["case"])] = bool(
                row["pass"]
            )
    finally:
        shutil.rmtree(audit_dir, ignore_errors=True)
    return ledger


def api_entries(
    target: str,
    rows: list[dict[str, Any]],
    security_ledger: dict[tuple[str, str, str, str], bool] | None = None,
    controller_ledger: dict[tuple[str, str, str, str], bool] | None = None,
) -> list[dict[str, Any]]:
    entries = []
    for row in rows:
        subject = f"{row['method']} {row['route']}"
        cases = [
            "route-presence",
            "nominal-status-headers-body",
            "malformed-path-query-or-body",
            "missing-empty-or-conflict-state",
            "runtime-failure-and-timeout",
        ]
        if row["method"] == "GET":
            cases.append("populated-dynamic-state")
        else:
            cases.extend(
                [
                    "mutation-side-effects-and-readback",
                    "restart-persistence-or-reset",
                    "concurrency-and-idempotency",
                ]
            )
        for case in cases:
            proven = (
                controller_ledger.get((target, row["method"], row["route"], case))
                if controller_ledger is not None
                else None
            )
            entries.append(
                {
                    "id": f"api:{target}:{row['method']}:{row['route']}:{case}",
                    "workstream": f"{target}-controller-api",
                    "featureFamily": feature_family(row["route"]),
                    "targets": [target],
                    "surface": "controller-route-case",
                    "subject": subject,
                    "case": case,
                    "status": "complete" if proven else "needs-proof",
                    "coverage": {
                        "routeInventory": "complete",
                        "behavioralDifferential": "complete" if proven else "open",
                    },
                    "evidence": row["controller"],
                }
            )

        for profile in (
            "anonymous",
            "basic-readonly",
            "basic-readwrite",
            "basic-administrator",
            "bearer-readonly",
            "bearer-readwrite",
            "bearer-administrator",
            "invalid-or-expired-credential",
            "missing-required-scope",
            "wrong-authentication-scheme",
        ):
            proven = (
                security_ledger.get((target, row["method"], row["route"], profile))
                if security_ledger is not None
                else None
            )
            entries.append(
                {
                    "id": f"security:{target}:{row['method']}:{row['route']}:{profile}",
                    "workstream": "security-authorization",
                    "featureFamily": feature_family(row["route"]),
                    "targets": [target],
                    "surface": "controller-authorization-case",
                    "subject": subject,
                    "case": profile,
                    "status": "complete" if proven else "needs-proof",
                    "coverage": {
                        "authorizationMetadata": "complete",
                        "liveHttpDifferential": "complete" if proven else "open",
                        "expected": row["auth"],
                    },
                    "evidence": row["controller"],
                }
            )
    return entries
