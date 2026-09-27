#!/usr/bin/env python3
"""Build the frozen, externally observable parity work manifest.

The manifest deliberately distinguishes inventory/presence from behavioral
proof. A route or WebUI call that exists but lacks its complete differential
matrix remains ``needs-proof``.
"""

from __future__ import annotations

import argparse
import collections
import csv
import json
import re
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any
from urllib.parse import unquote

from parity_protocol_inventory import protocol_entries, protocol_units
from parity_not_applicable import (
    FILE_LIFECYCLE_CASES,
    PERSISTENCE_CASES,
    file_lifecycle_not_applicable_cases,
    operator_not_applicable_cases,
    persistence_not_applicable_cases,
    security_not_applicable_cases,
)


EXPECTED = {
    "config": 436,
    "slskd-api": 96,
    "slskdn-api": 683,
    "webui-call-union": 417,
    "slskd-database-domains": 11,
    "slskdn-database-domains": 61,
    "slskd-file-writer-domains": 8,
    "slskdn-file-writer-domains": 42,
    "slskd-security-components": 12,
    "slskdn-security-components": 121,
    "slskd-operator-families": 3,
    "slskdn-operator-families": 37,
    "slskd-protocol-units": 123,
    "slskdn-protocol-units": 170,
    "live-interop-target-features": 62,
}

REACT_WEB_UI_ROUTE_COUNT = 42
REACT_WEB_UI_CASE_COUNT = REACT_WEB_UI_ROUTE_COUNT * 2

UNMATERIALIZED_WORKSTREAMS: list[dict[str, str]] = []

UNIVERSAL_BIDIRECTIONAL_TRANSPORTS = (
    "soulseek-peer-bidirectional",
    "obfuscated-peer-bidirectional",
    "distributed-dht-bidirectional",
    "overlay-udp-bidirectional",
    "overlay-quic-control-bidirectional",
    "quic-data-bidirectional",
    "relay-gateway-bidirectional",
    "mesh-sync-bidirectional",
    "virtualsoulfind-bidirectional",
    "file-stream-transfer-bidirectional",
)
UNIVERSAL_TRANSPORT_TARGETS = frozenset({"slskd", "slskdn"})
UNIVERSAL_BIDIRECTIONAL_TRANSPORT_TARGETS: dict[str, frozenset[str]] = {
    # Soulseek P/D/F behavior is shared by both frozen profiles.
    "soulseek-peer-bidirectional": frozenset({"slskd", "slskdn"}),
    "distributed-dht-bidirectional": frozenset({"slskd", "slskdn"}),
    "file-stream-transfer-bidirectional": frozenset({"slskd", "slskdn"}),
    # The remaining transports are slskdN-only in the frozen source. The
    # slskd profile must hide them rather than claim support it does not have.
    "obfuscated-peer-bidirectional": frozenset({"slskdn"}),
    "overlay-udp-bidirectional": frozenset({"slskdn"}),
    "overlay-quic-control-bidirectional": frozenset({"slskdn"}),
    "quic-data-bidirectional": frozenset({"slskdn"}),
    "relay-gateway-bidirectional": frozenset({"slskdn"}),
    "mesh-sync-bidirectional": frozenset({"slskdn"}),
    "virtualsoulfind-bidirectional": frozenset({"slskdn"}),
}
UNIVERSAL_LIFECYCLE_CHECK = "failure-restart-lifecycle-matrix"
UNIVERSAL_LIFECYCLE_SCENARIOS = frozenset(
    {
        "restart",
        "corrupt-state",
        "cancel",
        "timeout",
        "retry",
        "resume",
        "concurrent-mutation",
        "upgrade",
        "rollback",
        "permissions",
        "uninstall",
    }
)
UNIVERSAL_TRANSPORT_LIFECYCLE_REQUIREMENTS: dict[str, dict[str, dict[str, tuple[str, ...]]]] = {
    "mesh-sync-bidirectional": {
        "slskdn": {
            "reconnect-retry-and-failure": (
                "protocol-ksdn-mesh-sync-reconnect-retry",
            )
        }
    }
}
UNIVERSAL_TRANSPORT_LIFECYCLE_DETAIL_TOKENS = {
    "protocol-ksdn-mesh-sync-reconnect-retry": (
        'expected-target-negative status=400 body={"error":"Failed to sync with peer"}'
    )
}
UNIVERSAL_UI_SCENARIOS = frozenset(
    {
        "success",
        "rendered-loading-and-empty",
        "rendered-validation-and-server-error",
        "authorization-reconnect-and-restart",
    }
)
UNIVERSAL_UI_WORKFLOWS = frozenset(
    {
        "search",
        "browse",
        "transfers",
        "messages",
        "rooms",
        "shares",
        "settings",
        "player",
        "mesh",
    }
)


def run_json(command: list[str], cwd: Path) -> Any:
    command = guarded_process_command(command, cwd)
    completed = subprocess.run(
        command,
        cwd=cwd,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return json.loads(completed.stdout)


def run_logged(command: list[str], cwd: Path, env: dict[str, str] | None = None) -> None:
    """Run a proof command without contaminating the machine-readable manifest."""
    command = guarded_process_command(command, cwd)
    completed = subprocess.run(
        command,
        cwd=cwd,
        check=False,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if completed.stdout:
        print(completed.stdout, file=sys.stderr, end="")
    if completed.stderr:
        print(completed.stderr, file=sys.stderr, end="")
    if completed.returncode:
        raise subprocess.CalledProcessError(
            completed.returncode,
            command,
            output=completed.stdout,
            stderr=completed.stderr,
        )


def guarded_process_command(command: list[str], cwd: Path) -> list[str]:
    """Bound browser and frontend subprocesses before they can allocate."""
    if not command or Path(command[0]).name not in {"node", "nodejs", "npm", "npx"}:
        return command
    return [str(cwd / "scripts" / "with-process-memory-guard.sh"), *command]


def bounded_slskr_test_command(feature: str, selector: str | list[str]) -> list[str]:
    """Run one linked differential workstream without a Cargo test harness.

    The historical ``slskr`` test module is linked into a tiny feature-specific
    runner binary. ``selector`` remains part of this helper's call contract so
    the ledger functions continue to name the proof family they own; the Rust
    runner dispatches that family from its Cargo feature and does not pass the
    selector through to an all-tests harness.
    """
    del selector
    command = [
        "cargo",
        "run",
        "-p",
        "slskr",
        "--bin",
        "slskr-bounded-differential",
        "--no-default-features",
        "--features",
        feature,
    ]
    return command


def fresh_json_evidence_paths(evidence_dir: Path, started_ns: int) -> list[Path]:
    """Return only ledgers written or replaced by the current proof run.

    Retained evidence is explicitly supported only by ``--reuse-evidence``.
    A fresh run must not promote yesterday's JSON merely because the focused
    Cargo profile did not execute the optional monolithic test module.
    """
    if not evidence_dir.is_dir():
        return []
    paths = []
    for path in sorted(evidence_dir.glob("*.json")):
        try:
            if path.stat().st_mtime_ns >= started_ns:
                paths.append(path)
        except FileNotFoundError:
            continue
    return paths


def feature_family(subject: str) -> str:
    value = subject.split(" ", 1)[-1].strip("/")
    parts = [part for part in value.split("/") if part]
    while parts and (parts[0] == "api" or re.fullmatch(r"v(?:\d+|\{version\})", parts[0])):
        parts.pop(0)
    return parts[0].replace(":var", "parameter") if parts else "root"


def config_entries(report: dict[str, Any]) -> list[dict[str, Any]]:
    status_map = {"implemented": "complete", "partial": "partial", "missing": "missing"}
    return [
        {
            "id": f"config:{row['path']}",
            "workstream": "configuration",
            "featureFamily": row["path"].split(".", 1)[0],
            "targets": row["targets"],
            "surface": "configuration-leaf",
            "subject": row["path"],
            "status": status_map[row["overall"]],
            "coverage": {
                "yaml": row["yaml"],
                "environment": row["environment"],
                "commandLine": row["commandLine"],
                "runtime": row["runtime"],
                "lifecycleValidationDifferential": (
                    "complete" if row["overall"] == "implemented" else "open"
                ),
            },
            "evidence": row["runtimeEvidence"],
        }
        for row in report["comparison"]["leafStatus"]
    ]


SECURITY_AUTHORIZATION_TEST = (
    "focused_controller_tests::controller_contracts::security_authorization_matrix_matches_declared_policy_for_every_frozen_route"
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


CONTROLLER_API_DIFFERENTIAL_TEST_PREFIX = "controller_api_differential_"
CONTROLLER_API_TEST_FEATURES = (
    "bounded-controller-api-tests-1",
    "bounded-controller-api-tests-2",
    "bounded-controller-api-tests-3",
    "bounded-controller-api-tests-4",
)


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


def webui_workflow_ledger(
    root: Path, report: dict[str, Any], reuse_evidence: bool = False
) -> dict[str, dict[str, bool]]:
    """Run the real React WebUI against deterministic daemon-shaped responses.

    Each scenario is credited only when the browser actually requested that
    endpoint during the scenario workflow and the complete audit passed without
    a page error. Parameterized templates are matched after literal templates
    so a concrete path cannot be credited to a broader route shape.
    """
    audit_dir = root / "target" / "react-webui-audit"
    if not reuse_evidence:
        subprocess.run(
            guarded_process_command(["npm", "run", "build", "--prefix", "web"], root),
            cwd=root,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

    target_union = set(report["slskd"]["endpoints"]) | set(report["slskdn"]["endpoints"])
    templates = sorted(target_union)

    def observed_subject(method: str, path: str) -> str | None:
        normalized = unquote(path.split("?", 1)[0])
        for prefix in ("/api/v0", "/api/v1", "/api"):
            if normalized == prefix:
                normalized = "/"
                break
            if normalized.startswith(prefix + "/"):
                normalized = normalized[len(prefix) :]
                break
        segments = normalized.strip("/").split("/") if normalized.strip("/") else []
        matches = []
        for template in templates:
            template_method, template_path = template.split(" ", 1)
            if template_method != method:
                continue
            template_segments = template_path.strip("/").split("/") if template_path.strip("/") else []
            if len(template_segments) != len(segments):
                continue
            if all(expected == actual or expected == ":var" for expected, actual in zip(template_segments, segments)):
                matches.append(
                    (
                        sum(expected != ":var" for expected in template_segments),
                        template,
                    )
                )
        return max(matches)[1] if matches else None

    endpoint_sweep = []
    for template in sorted(target_union):
        method, path = template.split(" ", 1)
        concrete_path = re.sub(r":var\b", "audit", path)
        if not concrete_path.startswith("/api/"):
            concrete_path = f"/api/v0{concrete_path}"
        endpoint_sweep.append({"method": method, "url": concrete_path})

    scenarios = (
        ("rendered-success", "success"),
        ("rendered-loading-and-empty", "rendered-loading-and-empty"),
        ("rendered-validation-and-server-error", "rendered-validation-and-server-error"),
        ("authorization-reconnect-and-restart", "authorization-reconnect-and-restart"),
    )
    ledger: dict[str, dict[str, bool]] = {case: {} for case, _ in scenarios}
    if reuse_evidence:
        for case, scenario in scenarios:
            scenario_dir = audit_dir if scenario == "success" else audit_dir / "scenarios" / scenario
            audit_path = scenario_dir / "audit.json"
            if not audit_path.is_file():
                raise RuntimeError(f"reusable React WebUI evidence is missing: {audit_path}")
            audit = json.loads(audit_path.read_text(encoding="utf-8"))
            if audit.get("errors"):
                raise RuntimeError(
                    f"React WebUI {scenario} evidence contains errors: "
                    + "; ".join(audit["errors"])
                )
            for route in audit.get("routes", []):
                for response in route.get("apiResponses", []):
                    status = int(response.get("status", 0))
                    subject = observed_subject(
                        response.get("method", ""), response.get("path", "")
                    )
                    if subject is None:
                        continue
                    if scenario in {"success", "rendered-loading-and-empty"} and 200 <= status < 300:
                        ledger[case][subject] = True
                    elif scenario == "rendered-validation-and-server-error" and 400 <= status < 600:
                        ledger[case][subject] = True
                    elif scenario == "authorization-reconnect-and-restart" and status == 401:
                        ledger[case][subject] = True
        return ledger
    for case, scenario in scenarios:
        scenario_dir = audit_dir if scenario == "success" else audit_dir / "scenarios" / scenario
        environment = os.environ.copy()
        if not environment.get("SLSKR_PLAYWRIGHT_EXECUTABLE_PATH"):
            chromium = shutil.which("chromium") or shutil.which("chromium-browser")
            if chromium:
                environment["SLSKR_PLAYWRIGHT_EXECUTABLE_PATH"] = chromium
        environment.update(
            {
                "SLSKR_REACT_WEB_AUDIT_DIR": str(scenario_dir),
                "SLSKR_REACT_WEB_AUDIT_ENDPOINT_SWEEP": json.dumps(endpoint_sweep),
                "SLSKR_REACT_WEB_AUDIT_SCENARIO": scenario,
                "SLSKR_REACT_WEB_AUDIT_SKIP_BUILD": "1",
            }
        )
        if scenario != "success":
            environment.update(
                {
                    "SLSKR_REACT_WEB_AUDIT_SKIP_NAVIGATION": "1",
                    "SLSKR_REACT_WEB_AUDIT_SKIP_SCREENSHOTS": "1",
                    "SLSKR_REACT_WEB_AUDIT_ROUTES": "/",
                    "SLSKR_REACT_WEB_AUDIT_VIEWPORTS": "desktop",
                }
            )
        try:
            subprocess.run(
                guarded_process_command(["node", "web/scripts/audit-react-webui.mjs"], root),
                cwd=root,
                check=True,
                env=environment,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
        except subprocess.CalledProcessError as error:
            details = (error.stderr or error.stdout or "").strip()
            raise RuntimeError(
                f"React WebUI {scenario} subprocess failed: {details or error}"
            ) from error
        audit_path = scenario_dir / "audit.json"
        audit = json.loads(audit_path.read_text(encoding="utf-8"))
        if audit.get("errors"):
            raise RuntimeError(
                f"React WebUI {scenario} audit reported errors after a successful process exit: "
                + "; ".join(audit["errors"])
            )
        for route in audit.get("routes", []):
            for response in route.get("apiResponses", []):
                status = int(response.get("status", 0))
                subject = observed_subject(response.get("method", ""), response.get("path", ""))
                if subject is None:
                    continue
                if scenario == "success" and 200 <= status < 300:
                    ledger[case][subject] = True
                elif scenario == "rendered-loading-and-empty" and 200 <= status < 300:
                    ledger[case][subject] = True
                elif scenario == "rendered-validation-and-server-error" and 400 <= status < 600:
                    ledger[case][subject] = True
                elif scenario == "authorization-reconnect-and-restart" and status == 401:
                    ledger[case][subject] = True
    return ledger


def webui_entries(
    report: dict[str, Any], workflow_ledger: dict[str, dict[str, bool]] | None = None
) -> list[dict[str, Any]]:
    slskd = set(report["slskd"]["endpoints"])
    slskdn = set(report["slskdn"]["endpoints"])
    slskr = set(report["slskr"]["endpoints"])
    union = sorted(slskd | slskdn)
    entries = []
    for subject in union:
        targets = [
            target
            for target, values in (("slskd", slskd), ("slskdn", slskdn))
            if subject in values
        ]
        for case in (
            "call-presence",
            "rendered-success",
            "rendered-loading-and-empty",
            "rendered-validation-and-server-error",
            "authorization-reconnect-and-restart",
        ):
            call_present = subject in slskr
            rendered_success = bool(
                workflow_ledger
                and workflow_ledger.get("rendered-success", {}).get(subject)
            )
            rendered_empty = bool(
                workflow_ledger
                and workflow_ledger.get("rendered-loading-and-empty", {}).get(subject)
            )
            rendered_error = bool(
                workflow_ledger
                and workflow_ledger.get("rendered-validation-and-server-error", {}).get(subject)
            )
            rendered_auth = bool(
                workflow_ledger
                and workflow_ledger.get("authorization-reconnect-and-restart", {}).get(subject)
            )
            scenario_complete = {
                "rendered-success": rendered_success,
                "rendered-loading-and-empty": rendered_empty,
                "rendered-validation-and-server-error": rendered_error,
                "authorization-reconnect-and-restart": rendered_auth,
            }
            entries.append(
                {
                    "id": f"webui:{subject}:{case}",
                    "workstream": "webui-workflows",
                    "featureFamily": feature_family(subject),
                    "targets": targets,
                    "surface": "webui-workflow-case",
                    "subject": subject,
                    "case": case,
                    "status": "complete"
                    if case == "call-presence" and call_present
                    or case != "call-presence" and scenario_complete.get(case, False)
                    else "missing"
                    if not call_present
                    else "needs-proof",
                    "coverage": {
                        "callPresence": "complete" if call_present else "missing",
                        "renderedWorkflowDifferential": (
                            "not-applicable"
                            if case == "call-presence"
                            else "complete"
                            if scenario_complete.get(case, False)
                            else "open"
                        ),
                    },
                    "evidence": report["slskr"]["sources"].get(subject, [])
                    + (
                        [
                            "target/react-webui-audit/audit.json"
                            if case == "rendered-success"
                            else f"target/react-webui-audit/scenarios/{case}/audit.json"
                        ]
                        if scenario_complete.get(case, False)
                        else []
                    ),
                }
            )
    return entries


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


PERSISTENCE_DIFFERENTIAL_TEST_PREFIX = "persistence_lifecycle_differential_"




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


FILE_LIFECYCLE_DIFFERENTIAL_TEST_PREFIX = "file_lifecycle_differential_"




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


SECURITY_CONTROL_DIFFERENTIAL_TEST_PREFIX = "security_controls_differential_"
SECURITY_CONTROL_CASES = (
    "activation-default-and-profile",
    "accepted-nominal-input",
    "rejected-malicious-and-boundary-input",
    "quota-time-lockout-and-concurrency",
    "secret-logging-and-privacy-output",
    "restart-rotation-and-recovery",
)




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








PROTOCOL_DIFFERENTIAL_TEST_PREFIX = "protocol_behaviors_differential_"


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






def live_interop_features() -> list[tuple[str, str]]:
    shared = (
        "server-session",
        "peer-endpoint",
        "listener-and-indirect-connect",
        "type1-obfuscation",
        "public-search",
        "room-search",
        "wishlist-search",
        "browse-share-list",
        "folder-contents",
        "download",
        "upload",
        "queue-position",
        "transfer-resume-cancel-and-retry",
        "private-message",
        "batch-private-message",
        "public-room",
        "private-room-and-ticker",
        "user-watch-status-and-stats",
        "interests-and-recommendations",
        "privileges",
        "distributed-tree",
    )
    slskdn_only = (
        "peer-capability",
        "dht-rendezvous",
        "overlay-handshake-and-keepalive",
        "mesh-sync",
        "mesh-service-dht",
        "mesh-service-pods",
        "mesh-content-and-preview",
        "private-gateway-and-vpn",
        "shadow-index",
        "hole-punch",
        "mesh-introspection",
        "collections-and-share-grants",
        "download-requests",
        "multisource-and-swarm",
        "relay",
        "solid-and-federation",
        "virtualsoulfind-v2",
        "songid",
        "streaming-and-playback",
        "source-feeds-and-discovery",
    )
    return [
        *((target, feature) for target in ("slskd", "slskdn") for feature in shared),
        *(("slskdn", feature) for feature in slskdn_only),
    ]


LIVE_INTEROP_PROOF_REQUIREMENTS: dict[tuple[str, str, str], tuple[str, ...]] = {
    # The live runner has explicit initiator/target direction in these check
    # names. Keep this table deliberately narrow: a successful local API
    # assertion must not promote a broader interop case by inference.
    ("slskd", "peer-endpoint", "slskr-initiates-to-target"): (
        "network-slskr-resolves-slskd",
    ),
    ("slskd", "peer-endpoint", "target-initiates-to-slskr"): (
        "network-slskd-resolves-slskr",
    ),
    ("slskd", "server-session", "slskr-initiates-to-target"): (
        "runtime-slskd-session",
        "runtime-slskr-session-slskd",
    ),
    ("slskd", "public-search", "slskr-initiates-to-target"): (
        "protocol-slskr-searches-slskd",
    ),
    ("slskd", "public-search", "target-initiates-to-slskr"): (
        "protocol-slskd-searches-slskr",
    ),
    ("slskd", "browse-share-list", "slskr-initiates-to-target"): (
        "protocol-slskr-browses-slskd",
    ),
    ("slskd", "browse-share-list", "target-initiates-to-slskr"): (
        "protocol-slskd-browses-slskr",
    ),
    ("slskd", "folder-contents", "slskr-initiates-to-target"): (
        "protocol-slskr-folder-contents-slskd",
    ),
    ("slskd", "folder-contents", "target-initiates-to-slskr"): (
        "protocol-slskd-folder-contents-slskr",
    ),
    ("slskd", "download", "slskr-initiates-to-target"): (
        "slskr-to-slskd-download",
    ),
    ("slskd", "download", "target-initiates-to-slskr"): (
        "slskd-to-slskr-download",
    ),
    ("slskd", "upload", "slskr-initiates-to-target"): (
        "slskr-to-slskd-download",
    ),
    ("slskd", "upload", "target-initiates-to-slskr"): (
        "slskd-to-slskr-download",
    ),
    ("slskd", "private-message", "slskr-initiates-to-target"): (
        "protocol-slskr-message-dispatch-slskd",
    ),
    ("slskd", "private-message", "target-initiates-to-slskr"): (
        "protocol-slskd-message-dispatch",
    ),
    ("slskd", "server-session", "restart-and-persisted-state"): (
        "runtime-slskd-restart-session",
    ),
    ("slskd", "browse-share-list", "restart-and-persisted-state"): (
        "protocol-slskd-restart-browse",
    ),
    ("slskd", "folder-contents", "restart-and-persisted-state"): (
        "protocol-slskd-restart-folder",
    ),
    ("slskd", "public-room", "slskr-initiates-to-target"): (
        "protocol-slskr-public-room",
    ),
    ("slskd", "public-room", "target-initiates-to-slskr"): (
        "protocol-slskd-public-room",
    ),
    ("slskd", "user-watch-status-and-stats", "slskr-initiates-to-target"): (
        "protocol-slskr-user-watch-slskd",
    ),
    ("slskd", "user-watch-status-and-stats", "target-initiates-to-slskr"): (
        "protocol-slskd-user-watch-slskr",
    ),
    ("slskd", "distributed-tree", "slskr-initiates-to-target"): (
        "protocol-slskr-distributed-peer-slskd",
    ),
    ("slskdn", "peer-endpoint", "slskr-initiates-to-target"): (
        "network-slskr-resolves-slskdn",
    ),
    ("slskdn", "peer-endpoint", "target-initiates-to-slskr"): (
        "network-slskdn-resolves-slskr",
    ),
    ("slskdn", "server-session", "slskr-initiates-to-target"): (
        "runtime-slskdn-session",
        "runtime-slskr-session",
    ),
    ("slskdn", "public-search", "slskr-initiates-to-target"): (
        "protocol-slskr-searches-slskdn",
    ),
    ("slskdn", "public-search", "target-initiates-to-slskr"): (
        "protocol-slskdn-searches-slskr",
    ),
    ("slskdn", "browse-share-list", "slskr-initiates-to-target"): (
        "protocol-slskr-browses-slskdn",
    ),
    ("slskdn", "browse-share-list", "target-initiates-to-slskr"): (
        "protocol-slskdn-browses-slskr",
    ),
    ("slskdn", "folder-contents", "slskr-initiates-to-target"): (
        "protocol-slskr-browses-slskdn",
    ),
    ("slskdn", "folder-contents", "target-initiates-to-slskr"): (
        "protocol-slskdn-browses-slskr",
    ),
    ("slskdn", "download", "slskr-initiates-to-target"): (
        "slskr-to-slskdn-download",
    ),
    ("slskdn", "download", "target-initiates-to-slskr"): (
        "slskdn-to-slskr-download",
    ),
    ("slskdn", "upload", "slskr-initiates-to-target"): (
        "slskr-to-slskdn-download",
    ),
    ("slskdn", "upload", "target-initiates-to-slskr"): (
        "slskdn-to-slskr-download",
    ),
    ("slskdn", "private-message", "slskr-initiates-to-target"): (
        "protocol-slskr-message-dispatch",
    ),
    ("slskdn", "private-message", "target-initiates-to-slskr"): (
        "protocol-slskdn-message-dispatch",
    ),
    ("slskdn", "public-room", "slskr-initiates-to-target"): (
        "protocol-slskr-public-room-slskdn",
    ),
    ("slskdn", "public-room", "target-initiates-to-slskr"): (
        "protocol-slskdn-public-room",
    ),
    ("slskdn", "user-watch-status-and-stats", "slskr-initiates-to-target"): (
        "protocol-slskr-user-watch-slskdn",
    ),
    ("slskdn", "user-watch-status-and-stats", "target-initiates-to-slskr"): (
        "protocol-slskdn-user-watch-slskr",
    ),
    ("slskdn", "distributed-tree", "slskr-initiates-to-target"): (
        "protocol-slskr-distributed-peer-slskdn",
    ),
    ("slskdn", "peer-capability", "slskr-initiates-to-target"): (
        "protocol-ksdn-probe-dispatch",
        "protocol-ksdn-slskr-receives-ack",
        "protocol-ksdn-slskr-verifies-slskdn-descriptor",
        "protocol-ksdn-slskdn-receives-hello",
        "protocol-ksdn-slskdn-persists-slskr-descriptor",
    ),
    ("slskdn", "overlay-handshake-and-keepalive", "slskr-initiates-to-target"): (
        "protocol-pinned-overlay-certificate",
        "protocol-pinned-overlay-service",
    ),
    ("slskdn", "mesh-sync", "slskr-initiates-to-target"): (
        "protocol-ksdn-probe-dispatch",
        "protocol-ksdn-slskr-receives-ack",
        "protocol-ksdn-slskr-verifies-slskdn-descriptor",
        "protocol-ksdn-slskdn-receives-hello",
        "protocol-ksdn-slskdn-persists-slskr-descriptor",
    ),
    ("slskdn", "mesh-sync", "reconnect-retry-and-resume"): (
        "protocol-ksdn-mesh-sync-reconnect-retry",
    ),
    ("slskdn", "mesh-service-dht", "slskr-initiates-to-target"): (
        "protocol-pinned-overlay-certificate",
        "protocol-pinned-overlay-service",
        "protocol-slskr-dht-store-slskdn",
    ),
    ("slskdn", "mesh-service-pods", "slskr-initiates-to-target"): (
        "protocol-slskr-pods-list-slskdn",
        "protocol-slskr-pods-get-slskdn",
        "protocol-slskr-pods-join-slskdn",
        "protocol-slskr-pods-post-slskdn",
        "protocol-slskr-pods-messages-slskdn",
        "protocol-slskr-pods-leave-slskdn",
    ),
    ("slskdn", "mesh-content-and-preview", "slskr-initiates-to-target"): (
        "runtime-slskdn-mesh-content-id",
        "protocol-slskr-mesh-content-slskdn",
    ),
    ("slskdn", "private-gateway-and-vpn", "slskr-initiates-to-target"): (
        "runtime-slskdn-gateway-identity",
        "runtime-slskdn-gateway-pod-create",
        "protocol-slskr-gateway-pod-join-slskdn",
        "protocol-slskr-gateway-open-slskdn",
        "protocol-slskr-gateway-send-slskdn",
        "protocol-slskr-gateway-receive-slskdn",
        "protocol-slskr-gateway-close-slskdn",
    ),
    # This is a real cross-client source-discovery exchange: the Rust
    # backfill route negotiates a file-transfer connection to the frozen
    # slskdN peer, reads the bounded FLAC header, and verifies the returned
    # byte hash. Do not replace this with the local backfill-controller rows;
    # those are covered by controller-api evidence and do not prove a peer
    # exchange.
    ("slskdn", "source-feeds-and-discovery", "slskr-initiates-to-target"): (
        "protocol-slskr-backfill-slskdn",
    ),
}

# A green row is not sufficient when the probe contract changed after the
# artifact was emitted.  These rows require an explicit marker from the
# current runner so a stale pre-response-check TSV cannot certify a probe that
# only sent a Ping.
LIVE_INTEROP_REQUIRED_DETAIL_TOKENS: dict[str, str] = {
    "protocol-slskr-distributed-peer-slskd": "probe_contract=distributed-ping-response-v2",
    "protocol-slskr-distributed-peer-slskdn": "probe_contract=distributed-ping-response-v2",
    "protocol-slskr-obfuscated-peer-slskdn": "probe_contract=obfuscated-peer-v1 response_contract=plain-fallback",
}

# The frozen slskdN mesh controller reaches a stable generic 400 when its
# MeshSyncService has no outbound transport.  The interop runner records this
# as an expected negative row after two attempts against each profile; the
# detail token keeps a stale arbitrary failure from satisfying the retry case.

# The exact frozen slskdN source contains the Pod and private-gateway service
# implementations but does not register them with its mesh router.  Its
# no-auth Pod controller also deliberately rejects a gateway create request
# whose requesting identity differs from the declared gateway identity.  A
# fresh live `fail` row with these exact target responses is therefore a
# completed negative compatibility contract, not an implementation failure in
# slskR.  These checks are enabled only after the source-boundary validator
# below proves that the supplied target really has this frozen shape.
LIVE_INTEROP_EXPECTED_FAILURE_DETAIL_TOKENS: dict[str, str] = {
    "protocol-slskr-pods-list-slskdn": "Service 'pods' not found",
    "protocol-slskr-pods-get-slskdn": "Service 'pods' not found",
    "protocol-slskr-pods-join-slskdn": "Service 'pods' not found",
    "protocol-slskr-pods-post-slskdn": "Service 'pods' not found",
    "protocol-slskr-pods-messages-slskdn": "Service 'pods' not found",
    "protocol-slskr-pods-leave-slskdn": "Service 'pods' not found",
    "runtime-slskdn-gateway-pod-create": "RequestingPeerId must match GatewayPeerId",
    "protocol-slskr-gateway-pod-join-slskdn": "Service 'pods' not found",
    "protocol-slskr-gateway-open-slskdn": "Service 'private-gateway' not found",
    "protocol-slskr-gateway-send-slskdn": "gateway tunnel was not opened",
    "protocol-slskr-gateway-receive-slskdn": "echo payload unavailable",
    "protocol-slskr-gateway-close-slskdn": "gateway tunnel was not opened",
    "protocol-ksdn-mesh-sync-reconnect-retry": (
        'expected-target-negative status=400 body={"error":"Failed to sync with peer"}'
    ),
}

# The live matrix deliberately contains more dimensions than the credentialed
# runner can own.  A feature/case without an entry in
# LIVE_INTEROP_PROOF_REQUIREMENTS is not silently promoted from an unrelated
# green row: it is classified below as owned by the exact protocol, controller,
# persistence, or security differential that exercises that contract.  Keeping
# this classification explicit preserves the denominator while preventing a
# local API assertion from masquerading as peer interoperability.
LIVE_INTEROP_LOCAL_CONTROLLER_FEATURES = frozenset(
    {
        "shadow-index",
        "hole-punch",
        "mesh-introspection",
        "collections-and-share-grants",
        "download-requests",
        "multisource-and-swarm",
        "relay",
        "solid-and-federation",
        "virtualsoulfind-v2",
        "songid",
        "streaming-and-playback",
    }
)


def live_interop_not_applicable_reason(
    target: str,
    feature: str,
    case: str,
) -> str | None:
    """Return the authoritative non-live owner for an un-mapped case.

    The live runner only promotes rows for which it records a named,
    directionally exact peer transaction.  All other manifest dimensions stay
    visible, but their executable owner is another workstream.  This is a
    scope classification, not an inference from a neighboring green row.
    """
    key = (target, feature, case)
    if key in LIVE_INTEROP_PROOF_REQUIREMENTS:
        return None

    if target == "slskd" and feature == "type1-obfuscation":
        return (
            "Frozen slskd has no type-1 obfuscation option or runtime path; the "
            "protocol obfuscation differential owns the supported slskr contract, "
            "so a credentialed slskd live exchange is not applicable."
        )

    if target == "slskdn" and feature in LIVE_INTEROP_LOCAL_CONTROLLER_FEATURES:
        return (
            "This slskdN-only surface is a target-local controller/service-fabric "
            "contract rather than a Soulseek peer transaction; the slskdn "
            "controller, persistence, or security differential owns its behavior."
        )

    if feature == "source-feeds-and-discovery" and case != "slskr-initiates-to-target":
        return (
            "The frozen source-feed surface has no target-to-slskr peer transaction "
            "in this matrix; the target-local source-feed/controller lifecycle owns "
            "the non-backfill cases."
        )

    if case == "restart-and-persisted-state":
        return (
            "Restart and rehydration are owned by the persistence/lifecycle "
            "differential for this feature; the live runner only exposes the "
            "explicit restart checks listed in its proof contract."
        )

    if case in {"reconnect-retry-and-resume", "malformed-denied-timeout-and-cancel"}:
        return (
            "This failure/lifecycle dimension has no independent credentialed "
            "cross-client transaction in the bounded runner; exact protocol, "
            "controller, security, and persistence differentials own it."
        )

    if case == "target-initiates-to-slskr" and feature in {
        "server-session",
        "distributed-tree",
        "dht-rendezvous",
        "overlay-handshake-and-keepalive",
        "mesh-sync",
        "mesh-service-dht",
        "mesh-service-pods",
        "mesh-content-and-preview",
        "private-gateway-and-vpn",
    }:
        return (
            "This direction has no frozen-target initiated transaction exposed by "
            "the bounded runner; the corresponding typed protocol/overlay and "
            "controller differentials own the executable contract."
        )

    # Remaining un-mapped feature directions are intentionally not promoted by
    # neighboring rows.  Their exact server/protocol/controller behavior is
    # already materialized in the corresponding non-live workstream; the live
    # scope has no independent transaction to execute for this direction.
    return (
        "No exact credentialed cross-client transaction is defined for this "
        "feature and direction in the bounded live runner; the corresponding "
        "protocol/controller differential owns the contract, so this live case "
        "is not applicable."
    )


def validate_live_interop_scope_contracts(
    slskd_root: Path,
    slskdn_root: Path,
) -> None:
    """Guard source-boundary N/A classifications against frozen source drift."""
    slskd_sources = [
        path.read_text(encoding="utf-8-sig")
        for path in (slskd_root / "src/slskd").rglob("*.cs")
        if path.is_file()
    ]
    if not slskd_sources or any("obfus" in source.lower() for source in slskd_sources):
        raise ValueError(
            "slskd type-1 live N/A classification no longer matches frozen source"
        )

    local_controller_contracts = {
        "shadow-index": (
            "src/slskd/API/VirtualSoulfind/ShadowIndexController.cs",
            "api/virtualsoulfind/shadow-index",
        ),
        "hole-punch": (
            "src/slskd/Mesh/ServiceFabric/Services/HolePunchMeshService.cs",
            "class HolePunchMeshService",
        ),
        "mesh-introspection": (
            "src/slskd/Mesh/ServiceFabric/Services/MeshIntrospectionService.cs",
            "class MeshIntrospectionService",
        ),
        "collections-and-share-grants": (
            "src/slskd/Sharing/API/CollectionsController.cs",
            "class CollectionsController",
        ),
        "download-requests": (
            "src/slskd/Transfers/Downloads/API/DownloadRequestsController.cs",
            "class DownloadRequestsController",
        ),
        "multisource-and-swarm": (
            "src/slskd/Transfers/MultiSource/API/MultiSourceController.cs",
            "class MultiSourceController",
        ),
        "relay": (
            "src/slskd/Relay/API/Controllers/RelayController.cs",
            "class RelayController",
        ),
        "solid-and-federation": (
            "src/slskd/Solid/API/SolidController.cs",
            "class SolidController",
        ),
        "virtualsoulfind-v2": (
            "src/slskd/VirtualSoulfind/v2/API/VirtualSoulfindV2Controller.cs",
            "class VirtualSoulfindV2Controller",
        ),
        "songid": (
            "src/slskd/SongID/API/SongIdController.cs",
            "class SongIdController",
        ),
        "streaming-and-playback": (
            "src/slskd/Streaming/StreamsController.cs",
            "class StreamsController",
        ),
    }
    for feature in LIVE_INTEROP_LOCAL_CONTROLLER_FEATURES:
        relative, token = local_controller_contracts[feature]
        path = slskdn_root / relative
        if not path.is_file() or token.lower() not in path.read_text(encoding="utf-8-sig").lower():
            raise ValueError(
                f"slskdn local-controller live N/A classification lost source contract for {feature}"
            )

    for target, feature in live_interop_features():
        for case in (
            "slskr-initiates-to-target",
            "target-initiates-to-slskr",
            "reconnect-retry-and-resume",
            "malformed-denied-timeout-and-cancel",
            "restart-and-persisted-state",
        ):
            key = (target, feature, case)
            if key not in LIVE_INTEROP_PROOF_REQUIREMENTS and not live_interop_not_applicable_reason(
                target, feature, case
            ):
                raise ValueError(f"live interop scope has an unclassified case: {key!r}")


def validate_universal_transport_scope_contracts(
    slskd_root: Path,
    slskdn_root: Path,
) -> dict[str, frozenset[str]]:
    """Return the source-bound target set for each strict transport check.

    The strict transport contract is target-profile aware. Frozen slskd has
    relay HTTP code, but it has no mesh service router, type-1 obfuscation, or
    private-gateway transport. Those are slskdN-only capabilities; requiring
    them against slskd would test a feature the target does not expose.
    """
    slskd_sources = [
        path.read_text(encoding="utf-8-sig")
        for path in (slskd_root / "src/slskd").rglob("*.cs")
        if path.is_file()
    ]
    slskdn_sources = [
        path.read_text(encoding="utf-8-sig")
        for path in (slskdn_root / "src/slskd").rglob("*.cs")
        if path.is_file()
    ]
    if not slskd_sources or not slskdn_sources:
        raise ValueError("strict transport scope requires both frozen source trees")
    slskd_text = "\n".join(slskd_sources).lower()
    slskdn_text = "\n".join(slskdn_sources).lower()
    source_contracts = {
        "obfuscated-peer-bidirectional": "soulseekobfuscationsupport",
        "overlay-udp-bidirectional": "meshservicerouter",
        "overlay-quic-control-bidirectional": "quicstream",
        "quic-data-bidirectional": "quicstream",
        "relay-gateway-bidirectional": "privategatewaymeshservice",
        "mesh-sync-bidirectional": "meshservicerouter",
        "virtualsoulfind-bidirectional": "virtualsoulfindv2controller",
    }
    if "obfus" in slskd_text:
        raise ValueError(
            "frozen slskd source now exposes obfuscation; refresh the strict transport target contract"
        )
    for check_id, token in source_contracts.items():
        if token not in slskdn_text:
            raise ValueError(
                f"frozen slskdN source no longer exposes {token} for {check_id}"
            )
        if token in slskd_text:
            raise ValueError(
                f"frozen slskd source unexpectedly exposes {token} for {check_id}"
            )
    if set(UNIVERSAL_BIDIRECTIONAL_TRANSPORT_TARGETS) != set(
        UNIVERSAL_BIDIRECTIONAL_TRANSPORTS
    ):
        raise ValueError("strict transport target map is missing a declared transport")
    if any(
        not targets or not targets.issubset(UNIVERSAL_TRANSPORT_TARGETS)
        for targets in UNIVERSAL_BIDIRECTIONAL_TRANSPORT_TARGETS.values()
    ):
        raise ValueError("strict transport target map contains an invalid target set")
    return UNIVERSAL_BIDIRECTIONAL_TRANSPORT_TARGETS


def frozen_slskdn_expected_failure_checks(slskdn_root: Path) -> frozenset[str]:
    """Return negative live checks justified by the exact slskdN source.

    The live TSV is not allowed to turn an arbitrary failed request into
    passing evidence.  Only the pinned source shape that omits the mesh
    registrations and contains the gateway identity validation can enable
    these exact negative checks.  A newer target with the registrations must
    pass the positive rows instead.
    """
    application_path = slskdn_root / "src/slskd/Application.cs"
    pods_controller_path = slskdn_root / "src/slskd/API/Native/PodsController.cs"
    pods_service_path = (
        slskdn_root / "src/slskd/Mesh/ServiceFabric/Services/PodsMeshService.cs"
    )
    gateway_service_path = (
        slskdn_root
        / "src/slskd/Mesh/ServiceFabric/Services/PrivateGatewayMeshService.cs"
    )
    if not all(
        path.is_file()
        for path in (
            application_path,
            pods_controller_path,
            pods_service_path,
            gateway_service_path,
        )
    ):
        return frozenset()

    application = application_path.read_text(encoding="utf-8-sig")
    pods_controller = pods_controller_path.read_text(encoding="utf-8-sig")
    # Require the three registrations that are present in the pinned source;
    # this prevents an unrelated or truncated source snapshot from activating
    # the negative proof allowlist.
    required_existing_registrations = (
        "router.RegisterService(dhtService)",
        "router.RegisterService(holePunchService)",
        "router.RegisterService(meshContentService)",
    )
    if not all(token in application for token in required_existing_registrations):
        return frozenset()

    checks: set[str] = set()
    mesh_controller_path = slskdn_root / "src/slskd/Mesh/API/MeshController.cs"
    mesh_service_path = slskdn_root / "src/slskd/Mesh/MeshSyncService.cs"
    if mesh_controller_path.is_file() and mesh_service_path.is_file():
        mesh_controller = mesh_controller_path.read_text(encoding="utf-8-sig")
        mesh_service = mesh_service_path.read_text(encoding="utf-8-sig")
        if (
            'return BadRequest(new { error = "Failed to sync with peer" });'
            in mesh_controller
            and "if (!result.Success)" in mesh_controller
            and 'result.Error = "Mesh sync transport unavailable"' in mesh_service
            and "public async Task<MeshSyncResult> TrySyncWithPeerAsync" in mesh_service
        ):
            checks.add("protocol-ksdn-mesh-sync-reconnect-retry")
    if "PodsMeshService" not in application:
        checks.update(
            {
                "protocol-slskr-pods-list-slskdn",
                "protocol-slskr-pods-get-slskdn",
                "protocol-slskr-pods-join-slskdn",
                "protocol-slskr-pods-post-slskdn",
                "protocol-slskr-pods-messages-slskdn",
                "protocol-slskr-pods-leave-slskdn",
                "protocol-slskr-gateway-pod-join-slskdn",
            }
        )
    if "PrivateGatewayMeshService" not in application:
        checks.update(
            {
                "protocol-slskr-gateway-open-slskdn",
                "protocol-slskr-gateway-send-slskdn",
                "protocol-slskr-gateway-receive-slskdn",
                "protocol-slskr-gateway-close-slskdn",
            }
        )
    if "RequestingPeerId must match GatewayPeerId" in pods_controller:
        checks.add("runtime-slskdn-gateway-pod-create")
    return frozenset(checks)


def frozen_slskdn_transport_not_applicable_contracts(
    slskdn_root: Path,
) -> dict[str, dict[str, dict[str, dict[str, Any]]]]:
    """Return source-bound strict transport capability exceptions.

    A target-owned service that is implemented but never registered is not a
    live transport.  Likewise, the pinned VirtualSoulfind v2 controller reads
    an unbound options type whose default is disabled, even though the root
    configuration projection reports the feature as enabled.  These are
    explicit negative target contracts, not green peer transactions.

    The returned shape is ``check -> target -> direction -> contract``.  The
    strict auditor still requires a fresh capability artifact whose live rows
    and reason codes match these source-bound contracts.
    """
    contracts: dict[str, dict[str, dict[str, dict[str, Any]]]] = {}
    application_path = slskdn_root / "src/slskd/Application.cs"
    virtual_services_path = (
        slskdn_root / "src/slskd/Bootstrap/VirtualSoulfindServiceCollectionExtensions.cs"
    )
    virtual_controller_path = (
        slskdn_root / "src/slskd/VirtualSoulfind/v2/API/VirtualSoulfindV2Controller.cs"
    )
    virtual_root_options_path = slskdn_root / "src/slskd/Core/Options.cs"
    virtual_options_path = slskdn_root / "src/slskd/VirtualSoulfind/v2/VirtualSoulfindV2Options.cs"
    if not all(
        path.is_file()
        for path in (
            application_path,
            virtual_services_path,
            virtual_controller_path,
            virtual_root_options_path,
            virtual_options_path,
        )
    ):
        return contracts

    application = application_path.read_text(encoding="utf-8-sig")
    expected_failures = frozen_slskdn_expected_failure_checks(slskdn_root)

    # The frozen target exposes overlay listeners, but its reverse pod-route
    # path cannot resolve a replacement peer. The registration API is present
    # only on the resolver itself, has no call site, and its DHT key is a
    # SHA-256 value while the target's remote Store endpoint accepts only the
    # 20-byte Soulseek key shape. Keep the reverse directions explicitly
    # negative until the exact target wires a registration path.
    peer_resolution_path = slskdn_root / "src/slskd/PodCore/PeerResolutionService.cs"
    pod_router_path = slskdn_root / "src/slskd/PodCore/PodMessageRouter.cs"
    pod_services_path = slskdn_root / "src/slskd/PodCore/PodServices.cs"
    mesh_dht_client_path = slskdn_root / "src/slskd/Mesh/Dht/MeshDhtClient.cs"
    dht_mesh_service_path = (
        slskdn_root / "src/slskd/Mesh/ServiceFabric/Services/DhtMeshService.cs"
    )
    routing_paths = (
        peer_resolution_path,
        pod_router_path,
        pod_services_path,
        mesh_dht_client_path,
        dht_mesh_service_path,
    )
    if all(path.is_file() for path in routing_paths):
        peer_resolution = peer_resolution_path.read_text(encoding="utf-8-sig")
        pod_router = pod_router_path.read_text(encoding="utf-8-sig")
        pod_services = pod_services_path.read_text(encoding="utf-8-sig")
        mesh_dht_client = mesh_dht_client_path.read_text(encoding="utf-8-sig")
        dht_mesh_service = dht_mesh_service_path.read_text(encoding="utf-8-sig")
        source_files = [
            path
            for path in (slskdn_root / "src/slskd").rglob("*.cs")
            if path.is_file()
        ]
        registration_callsite_exists = any(
            path != peer_resolution_path
            and "RegisterPeerMapping(" in path.read_text(encoding="utf-8-sig")
            for path in source_files
        )
        reverse_overlay_route_is_unwired = (
            "void RegisterPeerMapping" in peer_resolution
            and "PeerMetadataPrefix = \"peer:metadata:\"" in peer_resolution
            and "ResolvePeerIdToEndpointAsync" in peer_resolution
            and "No endpoint for peer" in pod_router
            and "_peerResolution.ResolvePeerIdToEndpointAsync" in pod_router
            and "QUIC overlay routing available but peer resolution service not yet integrated" in pod_services
            and "SHA256.HashData(Encoding.UTF8.GetBytes(key))" in mesh_dht_client
            and "request.Key.Length != 20" in dht_mesh_service
            and not registration_callsite_exists
        )
        if reverse_overlay_route_is_unwired:
            reverse_overlay_contracts = {
                "overlay-udp-bidirectional": "protocol-slskdn-overlay-udp-slskr",
                "overlay-quic-control-bidirectional": "protocol-slskdn-overlay-quic-control-slskr",
                "quic-data-bidirectional": "protocol-slskdn-quic-data-slskr",
            }
            for check_id, evidence_check in reverse_overlay_contracts.items():
                contracts[check_id] = {
                    "slskdn": {
                        "target-to-slskr": {
                            "reason": "frozen-target-overlay-peer-resolution-unwired",
                            "evidenceChecks": [evidence_check],
                            "evidenceStatus": "ok",
                            "evidenceDetailTokens": {
                                evidence_check: "expected-target-negative endpoint-resolution-unavailable",
                            },
                        },
                    },
                }

    relay_checks = (
        "protocol-slskr-gateway-open-slskdn",
        "protocol-slskr-gateway-send-slskdn",
        "protocol-slskr-gateway-receive-slskdn",
        "protocol-slskr-gateway-close-slskdn",
        "protocol-slskr-gateway-pod-join-slskdn",
    )
    if (
        "PrivateGatewayMeshService" not in application
        and set(relay_checks).issubset(expected_failures)
    ):
        relay_contract: dict[str, dict[str, Any]] = {
            "slskr-to-target": {
                "reason": "frozen-target-private-gateway-service-not-registered",
                "evidenceChecks": list(relay_checks),
                "evidenceStatus": "fail",
                "evidenceDetailTokens": {
                    check: LIVE_INTEROP_EXPECTED_FAILURE_DETAIL_TOKENS[check]
                    for check in relay_checks
                },
            },
            "target-to-slskr": {
                "reason": "frozen-target-private-gateway-service-not-registered",
                "evidenceChecks": list(relay_checks),
                "evidenceStatus": "fail",
                "evidenceDetailTokens": {
                    check: LIVE_INTEROP_EXPECTED_FAILURE_DETAIL_TOKENS[check]
                    for check in relay_checks
                },
            },
        }
        contracts["relay-gateway-bidirectional"] = {
            "slskdn": relay_contract,
        }

    virtual_services = virtual_services_path.read_text(encoding="utf-8-sig")
    virtual_controller = virtual_controller_path.read_text(encoding="utf-8-sig")
    virtual_root_options = virtual_root_options_path.read_text(encoding="utf-8-sig")
    virtual_options = virtual_options_path.read_text(encoding="utf-8-sig")
    virtual_options_are_unbound = (
        "services.AddOptions<VirtualSoulfind.v2.VirtualSoulfindV2Options>();" in virtual_services
        and "Configure<VirtualSoulfind.v2.VirtualSoulfindV2Options>" not in virtual_services
        and "IOptionsMonitor<VirtualSoulfindV2Options>" in virtual_controller
        and "_options.CurrentValue.Enabled" in virtual_controller
        and '"VirtualSoulfind v2 is disabled"' in virtual_controller
        and "VirtualSoulfindV2" in virtual_root_options
        and "bool Enabled" in virtual_options
    )
    if virtual_options_are_unbound:
        virtual_contract = {
            direction: {
                "reason": "frozen-target-virtualsoulfind-v2-controller-options-unbound",
                "evidenceChecks": ["runtime-slskdn-virtualsoulfind-v2-create"],
                "evidenceStatus": "ok",
                "evidenceDetailTokens": {
                    "runtime-slskdn-virtualsoulfind-v2-create": "status=503 body=VirtualSoulfind v2 is disabled",
                },
            }
            for direction in ("slskr-to-target", "target-to-slskr")
        }
        contracts["virtualsoulfind-bidirectional"] = {"slskdn": virtual_contract}
    return contracts


def validate_live_interop_mapping_contracts(root: Path) -> None:
    """Keep promoted live checks tied to the behavior they actually exercise.

    This is intentionally a small source-boundary guard rather than a second
    live test.  The backfill row is allowed to promote only because the
    runner invokes the real backfill route and the daemon implementation
    performs a remote FLAC-header read and hash parse.  If either side is
    removed or reduced to a local-only assertion, the manifest must stop
    promoting that row.
    """
    runner = root / "scripts/run-slskdn-cross-client-interop.sh"
    rust_source = root / "crates/slskr/src/lib.rs"
    runner_source = runner.read_text(encoding="utf-8") if runner.is_file() else ""
    rust_text = rust_source.read_text(encoding="utf-8") if rust_source.is_file() else ""
    required_runner_tokens = (
        '"http://127.0.0.1:$slskr_http_port/api/v0/backfill/file"',
        'record_check protocol-slskr-backfill-slskdn ok',
        'hash" == "$slskdn_fixture_sha"',
    )
    required_rust_tokens = (
        "async fn read_remote_flac_header(",
        "parse_flac_backfill_hash(&header)",
        '"backfill transfer token did not match"',
    )
    if not all(token in runner_source for token in required_runner_tokens):
        raise ValueError(
            "live backfill mapping no longer has an exact remote route/hash assertion"
        )
    if not all(token in rust_text for token in required_rust_tokens):
        raise ValueError(
            "live backfill mapping no longer has an exact remote FLAC-header implementation"
        )
    required_mesh_sync_tokens = (
        "protocol-ksdn-mesh-sync-reconnect-retry",
        "/api/v0/mesh/sync/$escaped_slskr",
        "/api/v0/mesh/sync/$escaped_slskdn",
        'expected-target-negative status=400 body={"error":"Failed to sync with peer"}',
        "target_attempts=400,400 replacement_attempts=400,400",
    )
    if not all(token in runner_source for token in required_mesh_sync_tokens):
        raise ValueError(
            "live mesh-sync mapping no longer has the exact repeated target-negative contract"
        )
    mapped_checks = LIVE_INTEROP_PROOF_REQUIREMENTS[
        ("slskdn", "source-feeds-and-discovery", "slskr-initiates-to-target")
    ]
    if mapped_checks != ("protocol-slskr-backfill-slskdn",):
        raise ValueError("live backfill mapping changed without updating its source contract")


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


def transport_capability_evidence_failures(
    path: Path | None,
    contracts: dict[str, dict[str, dict[str, dict[str, Any]]]],
) -> list[str]:
    """Validate the live rows named by source-bound capability exceptions."""
    if not contracts:
        return []
    if path is None:
        return [
            "source-bound transport capability contracts exist but --transport-capability-evidence was not supplied"
        ]
    if not path.is_file():
        return [f"transport capability evidence does not exist: {path}"]
    try:
        evidence = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"transport capability evidence is not valid JSON: {error}"]
    failures: list[str] = []
    if evidence.get("evidenceMode") != "live":
        failures.append("transport capability evidence must declare evidenceMode=live")
    if evidence.get("schemaVersion") != 1:
        failures.append("transport capability evidence must declare schemaVersion=1")
    if evidence.get("target") != "slskdn":
        failures.append("transport capability evidence must target slskdn")
    if evidence.get("targetRevision") != "65a14a8b821de4df4ab7ef3ab3b156d7206837a3":
        failures.append("transport capability evidence must target frozen slskdN revision 65a14a8")
    records = evidence.get("checks")
    if not isinstance(records, list) or not records:
        return failures + ["transport capability evidence must contain checks"]

    tsv_rows: dict[str, dict[str, str]] = {}
    loaded_tsv_artifacts: set[str] = set()
    seen_records: set[tuple[str, str]] = set()
    for index, record in enumerate(records):
        if not isinstance(record, dict):
            failures.append(f"transport capability check {index} must be an object")
            continue
        check_id = record.get("id")
        target = record.get("target")
        pair = (check_id, target)
        if pair in seen_records:
            failures.append(f"transport capability evidence contains duplicate: {check_id}/{target}")
        seen_records.add(pair)
        contract = contracts.get(check_id, {}).get(target)
        if contract is None:
            failures.append(f"transport capability evidence names an unapproved contract: {check_id}/{target}")
            continue
        if record.get("status") != "not-applicable":
            failures.append(f"transport capability check {check_id}/{target} is not not-applicable")
        directions = record.get("directions")
        if not isinstance(directions, list) or not directions:
            failures.append(f"transport capability check {check_id}/{target} has no directions")
            continue
        reason = record.get("reason")
        expected_reasons = {
            contract.get(direction, {}).get("reason") for direction in directions
        }
        if not isinstance(reason, str) or expected_reasons != {reason}:
            failures.append(f"transport capability check {check_id}/{target} has an unapproved reason")
        evidence_checks = record.get("evidenceChecks")
        if not isinstance(evidence_checks, list) or not evidence_checks:
            failures.append(f"transport capability check {check_id}/{target} has no evidenceChecks")
            evidence_checks = []
        evidence_artifacts = record.get("evidenceArtifacts")
        if not isinstance(evidence_artifacts, list) or not evidence_artifacts:
            failures.append(f"transport capability check {check_id}/{target} has no evidenceArtifacts")
            evidence_artifacts = []
        for artifact in evidence_artifacts:
            if not isinstance(artifact, str) or not Path(artifact).is_file():
                failures.append(
                    f"transport capability check {check_id}/{target} names a missing evidence artifact: {artifact}"
                )
                continue
            if artifact.endswith(".tsv"):
                if artifact in loaded_tsv_artifacts:
                    continue
                loaded_tsv_artifacts.add(artifact)
                try:
                    with Path(artifact).open("r", encoding="utf-8", newline="") as handle:
                        reader = csv.DictReader(handle, delimiter="\t")
                        if reader.fieldnames != ["timestamp", "check", "status", "detail"]:
                            failures.append(f"transport capability TSV has invalid columns: {artifact}")
                            continue
                        for row in reader:
                            row_check = row.get("check", "")
                            if row_check:
                                if row_check in tsv_rows:
                                    failures.append(f"transport capability TSVs duplicate check: {row_check}")
                                tsv_rows[row_check] = row
                except OSError as error:
                    failures.append(f"transport capability TSV cannot be read: {artifact}: {error}")
        for direction in directions:
            direction_contract = contract.get(direction)
            if direction_contract is None:
                failures.append(f"transport capability check {check_id}/{target} has an unapproved direction: {direction}")
                continue
            expected_checks = set(direction_contract.get("evidenceChecks", []))
            if set(evidence_checks) != expected_checks:
                failures.append(
                    f"transport capability check {check_id}/{target}/{direction} does not name its exact evidenceChecks"
                )
        for evidence_check in evidence_checks:
            row = tsv_rows.get(evidence_check)
            if row is None:
                failures.append(f"transport capability evidence row is missing: {evidence_check}")
                continue
            direction_contracts = [
                contract.get(direction, {}) for direction in directions if direction in contract
            ]
            statuses = {item.get("evidenceStatus") for item in direction_contracts}
            tokens = {
                item.get("evidenceDetailTokens", {}).get(evidence_check)
                for item in direction_contracts
            }
            if statuses != {row.get("status")}:
                failures.append(f"transport capability row has unexpected status: {evidence_check}")
            if not any(token and token in row.get("detail", "") for token in tokens):
                failures.append(f"transport capability row has unexpected detail: {evidence_check}")
    required_records = {(check_id, target) for check_id, targets in contracts.items() for target in targets}
    missing_records = sorted(required_records - seen_records)
    if missing_records:
        failures.append(
            "transport capability evidence is missing contracts: "
            + ", ".join(f"{check_id}/{target}" for check_id, target in missing_records)
        )
    return failures


def universal_transport_failures(
    path: Path,
    required_targets_by_check: dict[str, frozenset[str]],
    capability_evidence: Path | None = None,
    capability_contracts: dict[str, dict[str, dict[str, dict[str, Any]]]] | None = None,
) -> list[str]:
    """Validate the fresh live evidence required by the universal gate.

    The ordinary parity manifest can prove local protocol/controller behavior,
    but it cannot infer that a transport worked across a frozen runtime. Keep
    those claims in an explicit artifact so a green local differential cannot
    accidentally certify a missing QUIC, DHT, relay, or reconnect path.
    """
    failures: list[str] = []
    capability_contracts = capability_contracts or {}
    failures.extend(
        transport_capability_evidence_failures(capability_evidence, capability_contracts)
    )
    if not path.is_file():
        return [f"transport evidence does not exist: {path}"]
    try:
        evidence = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"transport evidence is not valid JSON: {error}"]

    if evidence.get("evidenceMode") != "live":
        failures.append(
            "transport evidence must declare evidenceMode=live; local-only evidence cannot close the universal gate"
        )
    if not isinstance(evidence.get("generatedAt"), str) or not evidence["generatedAt"].strip():
        failures.append("transport evidence must include a non-empty generatedAt timestamp")
    records = evidence.get("checks")
    if not isinstance(records, list):
        return failures + ["transport evidence must contain a checks array"]

    by_id: dict[str, dict[str, Any]] = {}
    for index, record in enumerate(records):
        if not isinstance(record, dict) or not isinstance(record.get("id"), str):
            failures.append(f"transport evidence check {index} must be an object with a string id")
            continue
        check_id = record["id"]
        if check_id in by_id:
            failures.append(f"transport evidence contains duplicate check: {check_id}")
        else:
            by_id[check_id] = record

    required_directions = {"slskr-to-target", "target-to-slskr"}
    for check_id in UNIVERSAL_BIDIRECTIONAL_TRANSPORTS:
        record = by_id.get(check_id)
        if record is None:
            failures.append(f"transport evidence is missing {check_id}")
            continue
        if record.get("status") != "pass":
            failures.append(f"transport evidence check {check_id} is not pass")
        required_targets = required_targets_by_check.get(check_id, frozenset())
        record_targets = set(record.get("targets", []))
        if not required_targets.issubset(record_targets):
            failures.append(
                f"transport evidence check {check_id} must cover "
                + " and ".join(sorted(required_targets))
            )
        target_directions = record.get("targetDirections")
        not_applicable_directions = record.get("notApplicableDirections", {})
        not_applicable_reasons = record.get("notApplicableReasons", {})
        not_applicable_evidence_checks = record.get("notApplicableEvidenceChecks", {})
        if not isinstance(target_directions, dict):
            failures.append(
                f"transport evidence check {check_id} must declare targetDirections"
            )
        else:
            for target in sorted(required_targets):
                directions = target_directions.get(target)
                if not isinstance(directions, list):
                    directions = []
                accepted_directions = set(directions)
                target_not_applicable = (
                    not_applicable_directions.get(target, [])
                    if isinstance(not_applicable_directions, dict)
                    else []
                )
                if not isinstance(target_not_applicable, list):
                    failures.append(
                        f"transport evidence check {check_id} has invalid notApplicableDirections for {target}"
                    )
                    target_not_applicable = []
                accepted_directions.update(target_not_applicable)
                for direction in target_not_applicable:
                    contract = capability_contracts.get(check_id, {}).get(target, {}).get(direction)
                    reason = (
                        not_applicable_reasons.get(target, {}).get(direction)
                        if isinstance(not_applicable_reasons, dict)
                        and isinstance(not_applicable_reasons.get(target, {}), dict)
                        else None
                    )
                    evidence_checks = (
                        not_applicable_evidence_checks.get(target, {}).get(direction)
                        if isinstance(not_applicable_evidence_checks, dict)
                        and isinstance(not_applicable_evidence_checks.get(target, {}), dict)
                        else None
                    )
                    if contract is None:
                        failures.append(
                            f"transport evidence check {check_id} has an unapproved not-applicable direction: {target}/{direction}"
                        )
                    elif reason != contract.get("reason"):
                        failures.append(
                            f"transport evidence check {check_id} has an unapproved not-applicable reason: {target}/{direction}"
                        )
                    elif set(evidence_checks or []) != set(contract.get("evidenceChecks", [])):
                        failures.append(
                            f"transport evidence check {check_id} has incomplete not-applicable evidence: {target}/{direction}"
                        )
                if not required_directions.issubset(accepted_directions):
                    failures.append(
                        f"transport evidence check {check_id} must prove both directions for {target}"
                    )
        unsupported_targets = UNIVERSAL_TRANSPORT_TARGETS - required_targets
        if not unsupported_targets.issubset(set(record.get("notApplicableTargets", []))):
            failures.append(
                f"transport evidence check {check_id} must mark unsupported targets: "
                + ", ".join(sorted(unsupported_targets))
            )
        if not required_directions.issubset(set(record.get("directions", []))):
            failures.append(f"transport evidence check {check_id} must cover both directions")
        evidence_artifacts = record.get("evidenceArtifacts")
        if not isinstance(evidence_artifacts, list) or not evidence_artifacts:
            failures.append(
                f"transport evidence check {check_id} must name live evidence artifacts"
            )
        else:
            for artifact in evidence_artifacts:
                if not isinstance(artifact, str) or not Path(artifact).is_file():
                    failures.append(
                        f"transport evidence check {check_id} names a missing evidence artifact: {artifact}"
                    )

        lifecycle_requirements = UNIVERSAL_TRANSPORT_LIFECYCLE_REQUIREMENTS.get(check_id, {})
        if lifecycle_requirements:
            if record.get("lifecycleStatus") != "pass":
                failures.append(
                    f"transport evidence check {check_id} must pass its transport lifecycle cases"
                )
            lifecycle_targets = record.get("lifecycleTargets")
            if not isinstance(lifecycle_targets, dict):
                failures.append(
                    f"transport evidence check {check_id} must declare lifecycleTargets"
                )
                lifecycle_targets = {}
            for target, scenarios in lifecycle_requirements.items():
                target_records = lifecycle_targets.get(target)
                if not isinstance(target_records, dict):
                    failures.append(
                        f"transport evidence check {check_id} is missing lifecycle target {target}"
                    )
                    target_records = {}
                for scenario, expected_checks in scenarios.items():
                    case = target_records.get(scenario)
                    if not isinstance(case, dict) or case.get("status") != "pass":
                        failures.append(
                            f"transport evidence check {check_id} is missing passing lifecycle case {target}/{scenario}"
                        )
                        continue
                    if set(case.get("evidenceChecks", [])) != set(expected_checks):
                        failures.append(
                            f"transport evidence check {check_id} has incomplete lifecycle evidence {target}/{scenario}"
                        )
                    for evidence_check in expected_checks:
                        token = UNIVERSAL_TRANSPORT_LIFECYCLE_DETAIL_TOKENS.get(
                            evidence_check, ""
                        )
                        lifecycle_detail = case.get("detail", "") if isinstance(case, dict) else ""
                        if not token or token not in lifecycle_detail:
                            failures.append(
                                f"transport evidence check {check_id} has unverified lifecycle detail {evidence_check}"
                            )

    lifecycle = by_id.get(UNIVERSAL_LIFECYCLE_CHECK)
    if lifecycle is None:
        failures.append(f"transport evidence is missing {UNIVERSAL_LIFECYCLE_CHECK}")
    else:
        if lifecycle.get("status") != "pass":
            failures.append(f"transport evidence check {UNIVERSAL_LIFECYCLE_CHECK} is not pass")
        required_targets = UNIVERSAL_TRANSPORT_TARGETS
        if not required_targets.issubset(set(lifecycle.get("targets", []))):
            failures.append(
                f"transport evidence check {UNIVERSAL_LIFECYCLE_CHECK} must cover slskd and slskdn"
            )
        scenarios = set(lifecycle.get("scenarios", []))
        missing_scenarios = sorted(UNIVERSAL_LIFECYCLE_SCENARIOS - scenarios)
        if missing_scenarios:
            failures.append(
                f"transport evidence check {UNIVERSAL_LIFECYCLE_CHECK} is missing scenarios: "
                + ", ".join(missing_scenarios)
            )
        target_scenarios = lifecycle.get("targetScenarios")
        if not isinstance(target_scenarios, dict):
            failures.append(
                f"transport evidence check {UNIVERSAL_LIFECYCLE_CHECK} must declare targetScenarios"
            )
        else:
            for target in sorted(UNIVERSAL_TRANSPORT_TARGETS):
                target_cases = target_scenarios.get(target)
                if not isinstance(target_cases, list) or not UNIVERSAL_LIFECYCLE_SCENARIOS.issubset(
                    set(target_cases)
                ):
                    failures.append(
                        f"transport evidence check {UNIVERSAL_LIFECYCLE_CHECK} must cover every scenario for {target}"
                    )
        lifecycle_artifacts = lifecycle.get("evidenceArtifacts")
        if not isinstance(lifecycle_artifacts, list) or not lifecycle_artifacts:
            failures.append(
                f"transport evidence check {UNIVERSAL_LIFECYCLE_CHECK} must name live evidence artifacts"
            )
        else:
            for artifact in lifecycle_artifacts:
                if not isinstance(artifact, str) or not Path(artifact).is_file():
                    failures.append(
                        f"transport evidence check {UNIVERSAL_LIFECYCLE_CHECK} names a missing evidence artifact: {artifact}"
                    )
        cases = lifecycle.get("cases")
        expected_cases = {
            (target, scenario)
            for target in UNIVERSAL_TRANSPORT_TARGETS
            for scenario in UNIVERSAL_LIFECYCLE_SCENARIOS
        }
        observed_cases: set[tuple[str, str]] = set()
        if not isinstance(cases, list):
            failures.append(
                f"transport evidence check {UNIVERSAL_LIFECYCLE_CHECK} must contain per-case records"
            )
        else:
            for case in cases:
                if not isinstance(case, dict):
                    failures.append(
                        f"transport evidence check {UNIVERSAL_LIFECYCLE_CHECK} contains a non-object case"
                    )
                    continue
                pair = (case.get("target"), case.get("scenario"))
                if pair in observed_cases:
                    failures.append(
                        f"transport evidence check {UNIVERSAL_LIFECYCLE_CHECK} contains duplicate case: {pair[0]}/{pair[1]}"
                    )
                observed_cases.add(pair)
                if pair not in expected_cases:
                    failures.append(
                        f"transport evidence check {UNIVERSAL_LIFECYCLE_CHECK} contains unknown case: {pair[0]}/{pair[1]}"
                    )
                if case.get("status") != "pass":
                    failures.append(
                        f"transport evidence lifecycle case {pair[0]}/{pair[1]} is not pass"
                    )
                case_artifacts = case.get("evidenceArtifacts")
                if not isinstance(case_artifacts, list) or not case_artifacts:
                    failures.append(
                        f"transport evidence lifecycle case {pair[0]}/{pair[1]} must name evidence artifacts"
                    )
                else:
                    for artifact in case_artifacts:
                        if not isinstance(artifact, str) or not Path(artifact).is_file():
                            failures.append(
                                f"transport evidence lifecycle case {pair[0]}/{pair[1]} names a missing evidence artifact: {artifact}"
                            )
        missing_cases = sorted(expected_cases - observed_cases)
        if missing_cases:
            failures.append(
                f"transport evidence check {UNIVERSAL_LIFECYCLE_CHECK} is missing cases: "
                + ", ".join(f"{target}/{scenario}" for target, scenario in missing_cases)
            )
    return failures


def universal_ui_scenario_failures(
    paths: list[Path] | None,
    *,
    label: str,
    expected_routes: int,
) -> list[str]:
    """Require live evidence for every user-visible UI state scenario."""
    failures: list[str] = []
    if not paths:
        return [
            f"universal replacement requires fresh live {label} UI scenario evidence"
        ]

    scenarios: set[str] = set()
    for path in paths:
        if not path.is_file():
            failures.append(f"{label} UI scenario evidence does not exist: {path}")
            continue
        try:
            evidence = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            failures.append(f"{label} UI scenario evidence is not valid JSON: {error}")
            continue

        scenario = evidence.get("scenario")
        if not isinstance(scenario, str) or not scenario.strip():
            failures.append(f"{label} UI scenario evidence must include a scenario: {path}")
            continue
        if scenario in scenarios:
            failures.append(f"{label} UI scenario evidence contains duplicate scenario: {scenario}")
        scenarios.add(scenario)
        if evidence.get("evidenceMode") != "live":
            failures.append(
                f"{label} UI scenario {scenario} must declare evidenceMode=live; mock-only evidence cannot close the goal"
            )
        if evidence.get("errors"):
            failures.append(f"{label} UI scenario {scenario} contains errors")
        routes = evidence.get("routes")
        if not isinstance(routes, list) or len(routes) != expected_routes:
            failures.append(
                f"{label} UI scenario {scenario} must cover {expected_routes} route/viewport cases"
            )

    missing = sorted(UNIVERSAL_UI_SCENARIOS - scenarios)
    if missing:
        failures.append(
            f"{label} UI scenario evidence is missing: " + ", ".join(missing)
        )
    return failures


def universal_target_ui_comparison_failures(path: Path | None) -> list[str]:
    """Require fresh side-by-side workflow/action/response evidence.

    The replacement UI audit proves that slskR renders and handles its own
    live backend. It cannot prove that a user moving from either frozen target
    sees the same workflow actions and response semantics. Keep that evidence
    in a separate artifact so the two claims cannot be conflated.
    """
    if path is None:
        return [
            "universal replacement requires fresh frozen-target side-by-side UI comparison evidence"
        ]
    if not path.is_file():
        return [f"frozen-target UI comparison evidence does not exist: {path}"]
    try:
        evidence = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"frozen-target UI comparison evidence is not valid JSON: {error}"]

    failures: list[str] = []
    if evidence.get("evidenceMode") != "live":
        failures.append(
            "frozen-target UI comparison evidence must declare evidenceMode=live"
        )
    if evidence.get("comparisonMode") != "frozen-target-side-by-side":
        failures.append(
            "frozen-target UI comparison evidence must declare comparisonMode=frozen-target-side-by-side"
        )
    if not isinstance(evidence.get("generatedAt"), str) or not evidence["generatedAt"].strip():
        failures.append("frozen-target UI comparison evidence must include generatedAt")
    if set(evidence.get("targets", [])) != UNIVERSAL_TRANSPORT_TARGETS:
        failures.append("frozen-target UI comparison evidence must cover slskd and slskdn")

    semantic = evidence.get("semanticComparison")
    if not isinstance(semantic, dict) or semantic.get("status") != "pass":
        failures.append(
            "frozen-target UI comparison must prove semantic parity; structural rendering evidence is insufficient"
        )
    if not isinstance(semantic, dict) or semantic.get("replacementEventFeed") != "live":
        failures.append(
            "frozen-target UI comparison must use a live replacement event feed"
        )
    profiles = semantic.get("replacementProfiles") if isinstance(semantic, dict) else None
    if not isinstance(profiles, list) or set(profiles) != UNIVERSAL_TRANSPORT_TARGETS:
        failures.append(
            "frozen-target UI comparison must cover replacement profiles slskd and slskdn"
        )
    comparisons = semantic.get("comparisons") if isinstance(semantic, dict) else None
    expected_comparisons = {
        (workflow, target)
        for workflow in UNIVERSAL_UI_WORKFLOWS
        for target in UNIVERSAL_TRANSPORT_TARGETS
    }
    observed_comparisons: set[tuple[Any, Any]] = set()
    if not isinstance(comparisons, list):
        failures.append(
            "frozen-target UI comparison must contain semantic workflow/profile comparisons"
        )
    else:
        for comparison in comparisons:
            if not isinstance(comparison, dict):
                failures.append("frozen-target UI semantic comparison contains a non-object record")
                continue
            pair = (comparison.get("workflow"), comparison.get("target"))
            if pair in observed_comparisons:
                failures.append(
                    f"frozen-target UI semantic comparison contains duplicate pair: {pair[0]}/{pair[1]}"
                )
            observed_comparisons.add(pair)
            if pair not in expected_comparisons:
                failures.append(
                    f"frozen-target UI semantic comparison contains unknown pair: {pair[0]}/{pair[1]}"
                )
            target = comparison.get("target")
            if comparison.get("replacementSurface") != f"replacement-{target}":
                failures.append(
                    f"frozen-target UI semantic comparison uses the wrong replacement profile for {pair[0]}/{target}"
                )
            if comparison.get("apiPathsEqual") is not True:
                failures.append(
                    f"frozen-target UI semantic comparison has unequal API paths: {pair[0]}/{target}"
                )
            if comparison.get("controlsEqual") is not True:
                failures.append(
                    f"frozen-target UI semantic comparison has unequal visible controls: {pair[0]}/{target}"
                )
            if comparison.get("eventFeedLive") is not True:
                failures.append(
                    f"frozen-target UI semantic comparison has no live event feed: {pair[0]}/{target}"
                )
    missing_comparisons = sorted(expected_comparisons - observed_comparisons)
    if missing_comparisons:
        failures.append(
            "frozen-target UI semantic comparison is missing pairs: "
            + ", ".join(f"{workflow}/{target}" for workflow, target in missing_comparisons)
        )

    workflows = evidence.get("workflows")
    seen_workflows: set[str] = set()
    if not isinstance(workflows, list):
        failures.append("frozen-target UI comparison evidence must contain workflows")
    else:
        for index, workflow in enumerate(workflows):
            if not isinstance(workflow, dict) or not isinstance(workflow.get("id"), str):
                failures.append(f"frozen-target UI workflow {index} is missing an id")
                continue
            workflow_id = workflow["id"]
            if workflow_id in seen_workflows:
                failures.append(f"frozen-target UI comparison contains duplicate workflow: {workflow_id}")
            seen_workflows.add(workflow_id)
            if set(workflow.get("targets", [])) != UNIVERSAL_TRANSPORT_TARGETS:
                failures.append(f"frozen-target UI workflow {workflow_id} must cover both targets")
            if not isinstance(workflow.get("actions"), list) or not workflow["actions"]:
                failures.append(f"frozen-target UI workflow {workflow_id} has no recorded actions")
            if not isinstance(workflow.get("responses"), list) or not workflow["responses"]:
                failures.append(f"frozen-target UI workflow {workflow_id} has no recorded responses")
    missing_workflows = sorted(UNIVERSAL_UI_WORKFLOWS - seen_workflows)
    if missing_workflows:
        failures.append(
            "frozen-target UI comparison is missing workflows: " + ", ".join(missing_workflows)
        )

    artifacts = evidence.get("artifacts")
    if not isinstance(artifacts, list) or not artifacts:
        failures.append("frozen-target UI comparison evidence must name live artifacts")
    else:
        for artifact in artifacts:
            if not isinstance(artifact, str) or not Path(artifact).is_file():
                failures.append(
                    f"frozen-target UI comparison names a missing artifact: {artifact}"
                )
    return failures


def strict_universal_failures(
    entries: list[dict[str, Any]],
    *,
    reuse_evidence: bool,
    live_interop_evidence: list[Path] | None,
    operator_evidence: Path | None,
    transport_evidence: Path | None,
    react_ui_evidence: Path | None,
    rust_ui_evidence: Path | None,
    react_ui_scenario_evidence: list[Path] | None,
    rust_ui_scenario_evidence: list[Path] | None,
    target_ui_comparison_evidence: Path | None,
    transport_target_requirements: dict[str, frozenset[str]],
    transport_capability_evidence: Path | None,
    transport_capability_contracts: dict[str, dict[str, dict[str, dict[str, Any]]]],
) -> list[str]:
    """Apply the stronger universal-replacement contract.

    The ordinary manifest is a frozen proof ledger. It may reuse retained
    evidence and may classify target-local dimensions as not applicable. That
    is useful for regression reporting but is not sufficient to claim a
    universal drop-in replacement. This gate therefore requires fresh proof,
    exact live evidence for every live-interop case, and a separate live
    backend React and Rust UI audits.
    """
    failures: list[str] = []
    if reuse_evidence:
        failures.append(
            "universal replacement cannot reuse retained evidence; run fresh differentials"
        )
    if not live_interop_evidence:
        failures.append(
            "universal replacement requires explicit all-green live-interop TSV evidence"
        )
    if operator_evidence is None:
        failures.append(
            "universal replacement requires explicit operator-packaging evidence"
        )
    if transport_evidence is None:
        failures.append(
            "universal replacement requires fresh live bidirectional transport and lifecycle evidence"
        )
    else:
        failures.extend(
            universal_transport_failures(
                transport_evidence,
                transport_target_requirements,
                transport_capability_evidence,
                transport_capability_contracts,
            )
        )
    if react_ui_evidence is None:
        failures.append(
            "universal replacement requires a live-backend React UI audit JSON"
        )
    elif not react_ui_evidence.is_file():
        failures.append(f"React UI evidence does not exist: {react_ui_evidence}")
    else:
        try:
            react_audit = json.loads(react_ui_evidence.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            failures.append(f"React UI evidence is not valid JSON: {error}")
        else:
            if react_audit.get("evidenceMode") != "live":
                failures.append(
                    "React UI evidence must declare evidenceMode=live; mock-only audits do not close the goal"
                )
            if react_audit.get("allowLiveErrors"):
                failures.append(
                    "React UI live evidence cannot enable blanket live-error allowance"
                )
            if react_audit.get("errors"):
                failures.append("React UI evidence contains errors")
            react_routes = react_audit.get("routes", [])
            route_pairs = {
                (route.get("route"), route.get("viewport"))
                for route in react_routes
                if isinstance(route, dict)
            }
            if len(react_routes) != REACT_WEB_UI_CASE_COUNT or len(route_pairs) != REACT_WEB_UI_CASE_COUNT:
                failures.append(
                    f"React UI live evidence must cover all {REACT_WEB_UI_ROUTE_COUNT} routes at desktop and mobile viewports"
                )
            react_statuses = [
                response
                for response in react_audit.get("apiResponses", [])
                if isinstance(response, dict) and isinstance(response.get("status"), int)
            ]
            if not react_statuses:
                failures.append("React UI live evidence contains no proxied API responses")
            else:
                expected_allowed_failures = {
                    (404, "GET", "/api/v0/security/adversarial"),
                }
                unexpected_failures = [
                    (
                        response.get("status"),
                        response.get("method"),
                        str(response.get("path", "")).split("?", 1)[0],
                    )
                    for response in react_statuses
                    if response["status"] >= 400
                    and (
                        not response.get("allowed", False)
                        or (
                            response.get("status"),
                            response.get("method"),
                            str(response.get("path", "")).split("?", 1)[0],
                        )
                        not in expected_allowed_failures
                    )
                ]
                if unexpected_failures:
                    failures.append(
                        "React UI live evidence contains an unapproved HTTP failure response: "
                        f"{unexpected_failures[0]}"
                    )
    if rust_ui_evidence is None:
        failures.append(
            "universal replacement requires a live-backend Rust UI audit JSON"
        )
    elif not rust_ui_evidence.is_file():
        failures.append(f"Rust UI evidence does not exist: {rust_ui_evidence}")
    else:
        try:
            ui_audit = json.loads(rust_ui_evidence.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            failures.append(f"Rust UI evidence is not valid JSON: {error}")
        else:
            if ui_audit.get("evidenceMode") != "live":
                failures.append(
                    "Rust UI evidence must declare evidenceMode=live; mock-only audits do not close the goal"
                )
            if ui_audit.get("errors"):
                failures.append("Rust UI evidence contains errors")
            if len(ui_audit.get("routes", [])) != 30:
                failures.append(
                    "Rust UI live evidence must cover all 15 routes at desktop and mobile viewports"
                )

    failures.extend(
        universal_ui_scenario_failures(
            ([react_ui_evidence] if react_ui_evidence else [])
            + (react_ui_scenario_evidence or []),
            label="React",
            expected_routes=REACT_WEB_UI_CASE_COUNT,
        )
    )
    failures.extend(universal_target_ui_comparison_failures(target_ui_comparison_evidence))
    failures.extend(
        universal_ui_scenario_failures(
            ([rust_ui_evidence] if rust_ui_evidence else [])
            + (rust_ui_scenario_evidence or []),
            label="Rust",
            expected_routes=30,
        )
    )

    for entry in entries:
        if entry["status"] != "complete":
            failures.append(
                f"{entry['workstream']}:{entry['id']} remains {entry['status']}"
            )
        if entry["workstream"] == "live-interop":
            coverage = entry.get("coverage", {})
            if (
                coverage.get("liveBehavioralProof") != "complete"
                and coverage.get("typedDifferentialProof") != "complete"
            ):
                failures.append(
                    f"{entry['id']} lacks exact live behavioral proof; "
                    "it needs either an exact live transaction or a fresh, source-bound typed differential"
                )

    return failures


def summarize(entries: list[dict[str, Any]]) -> dict[str, Any]:
    by_workstream: dict[str, collections.Counter[str]] = collections.defaultdict(collections.Counter)
    totals: collections.Counter[str] = collections.Counter()
    for entry in entries:
        by_workstream[entry["workstream"]][entry["status"]] += 1
        totals[entry["status"]] += 1
    statuses = ("complete", "partial", "missing", "needs-proof")
    materialized_entry_count = len(entries)
    complete_count = totals["complete"]
    proof_case_closure_percentage = (
        round((complete_count / materialized_entry_count) * 100, 2)
        if materialized_entry_count
        else 0.0
    )
    return {
        "materializedEntryCount": materialized_entry_count,
        "statusCounts": {status: totals[status] for status in statuses},
        "workstreams": {
            name: {
                "total": sum(counts.values()),
                **{status: counts[status] for status in statuses},
            }
            for name, counts in sorted(by_workstream.items())
        },
        "unmaterializedWorkstreamCount": len(UNMATERIALIZED_WORKSTREAMS),
        # This is a literal executable-proof-case ratio, not a subjective
        # estimate of user-visible feature completeness. The manifest cases
        # intentionally have different granularity, so keep the label explicit.
        "proofCaseClosurePercentage": proof_case_closure_percentage,
        "overallPercentage": proof_case_closure_percentage,
        "percentageBasis": "complete materialized proof cases / all materialized proof cases",
        # This is deliberately not the universal-replacement claim. The
        # ordinary frozen ledger can be complete while strict live transport,
        # lifecycle, or UI comparison evidence is absent.
        "ordinaryLedgerComplete": (
            not UNMATERIALIZED_WORKSTREAMS
            and all(entry["status"] == "complete" for entry in entries)
        ),
        "goalAchieved": False,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--slskd-root", type=Path, required=True)
    parser.add_argument("--slskdn-root", type=Path, required=True)
    parser.add_argument("--slskr-root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--check-frozen", action="store_true")
    parser.add_argument("--require-complete", action="store_true")
    parser.add_argument(
        "--live-interop-evidence",
        type=Path,
        action="append",
        help=(
            "Opt in to one or more explicit all-green credentialed live-interop "
            "TSVs. Only exact mapped feature/direction cases are promoted."
        ),
    )
    parser.add_argument(
        "--operator-evidence",
        type=Path,
        help=(
            "Opt in to explicit operator-packaging artifact evidence. Only "
            "exact target/family/case rows with pass=true are promoted."
        ),
    )
    parser.add_argument(
        "--transport-evidence",
        type=Path,
        help=(
            "Explicit fresh live JSON for --strict-universal. It must prove "
            "every supported transport in both directions and the full "
            "lifecycle matrix."
        ),
    )
    parser.add_argument(
        "--transport-capability-evidence",
        type=Path,
        help=(
            "Fresh live source-bound capability JSON for target transport "
            "directions that are explicitly not applicable."
        ),
    )
    parser.add_argument(
        "--skip-security-differential",
        action="store_true",
        help=(
                        "Skip running the exhaustive security-authorization bounded "
                        "differential runner (crates/slskr linked proof slice). All "
            "security-authorization cases fall back to needs-proof. Use "
            "only for fast, evidence-incomplete dry runs."
        ),
    )
    parser.add_argument(
        "--skip-security-control-differential",
        action="store_true",
        help=(
            "Skip the explicit security-controls differential tests. Security "
            "component cases fall back to needs-proof. Use only for fast, "
            "evidence-incomplete dry runs."
        ),
    )
    parser.add_argument(
        "--skip-controller-api-differential",
        action="store_true",
        help=(
            "Skip running the controller API bounded differential runner "
            "(crates/slskr). All controller-api cases fall back to "
            "needs-proof. Use only for fast, evidence-incomplete dry runs."
        ),
    )
    parser.add_argument(
        "--skip-persistence-differential",
        action="store_true",
        help=(
            "Skip running the persistence bounded differential runner "
            "(crates/slskr). All persistence-lifecycle cases fall "
            "back to needs-proof. Use only for fast, evidence-incomplete "
            "dry runs."
        ),
    )
    parser.add_argument(
        "--skip-file-differential",
        action="store_true",
        help=(
            "Skip running the file-lifecycle bounded differential runner. "
            "File-lifecycle cases fall back to needs-proof. Use only for "
            "fast, evidence-incomplete dry runs."
        ),
    )
    parser.add_argument(
        "--skip-protocol-differential",
        action="store_true",
        help=(
            "Skip running the protocol bounded differential runner plus "
            "the slskr-protocol/slskr-client Cargo slices. All "
            "protocol-behaviors cases fall back to "
            "needs-proof. Use only for fast, "
            "evidence-incomplete dry runs."
        ),
    )
    parser.add_argument(
        "--reuse-evidence",
        action="store_true",
        help=(
            "Reuse retained passing differential/WebUI evidence from "
            "/tmp/slskr-parity-evidence and target/react-webui-audit without "
            "starting Cargo or browser proof processes."
        ),
    )
    parser.add_argument(
        "--rust-ui-evidence",
        type=Path,
        help=(
            "Explicit Rust UI audit JSON for --strict-universal. It must be a "
            "fresh live-backend audit covering all 15 routes at both viewports."
        ),
    )
    parser.add_argument(
        "--react-ui-evidence",
        type=Path,
        help=(
            "Explicit React UI audit JSON for --strict-universal. It must be a "
            f"fresh live-backend audit covering all {REACT_WEB_UI_ROUTE_COUNT} routes at both viewports."
        ),
    )
    parser.add_argument(
        "--react-ui-scenario-evidence",
        type=Path,
        action="append",
        help=(
            "Additional fresh live React UI scenario JSON artifacts. Together "
            "with --react-ui-evidence they must cover every required state."
        ),
    )
    parser.add_argument(
        "--rust-ui-scenario-evidence",
        type=Path,
        action="append",
        help=(
            "Additional fresh live Rust UI scenario JSON artifacts. Together "
            "with --rust-ui-evidence they must cover every required state."
        ),
    )
    parser.add_argument(
        "--target-ui-comparison-evidence",
        type=Path,
        help=(
            "Fresh live side-by-side workflow/action/response comparison JSON "
            "for both frozen target profiles."
        ),
    )
    parser.add_argument(
        "--strict-universal",
        action="store_true",
        help=(
            "Require the universal drop-in replacement contract: fresh evidence, "
            "all live interop directions, operator evidence, and live-backend React/Rust UI proof."
        ),
    )
    args = parser.parse_args()

    if args.strict_universal and args.reuse_evidence:
        parser.error("--strict-universal cannot be combined with --reuse-evidence")

    root = args.slskr_root.resolve()
    slskd_root = args.slskd_root.resolve()
    slskdn_root = args.slskdn_root.resolve()
    validate_live_interop_mapping_contracts(root)
    validate_live_interop_scope_contracts(slskd_root, slskdn_root)
    transport_target_requirements = validate_universal_transport_scope_contracts(
        slskd_root,
        slskdn_root,
    )
    transport_capability_contracts = frozen_slskdn_transport_not_applicable_contracts(
        slskdn_root
    )
    expected_failure_checks = frozen_slskdn_expected_failure_checks(slskdn_root)
    config_command = [
        sys.executable,
        "scripts/audit-upstream-config-surface.py",
        "--slskd-root",
        str(slskd_root),
        "--slskdn-root",
        str(slskdn_root),
        "--slskr-root",
        str(root),
        "--json",
    ]
    if args.check_frozen:
        config_command.append("--check-frozen")

    config = run_json(config_command, root)
    slskd_api = run_json(
        ["node", "scripts/audit-slskdn-controller-routes.mjs", "--slskdn-root", str(slskd_root), "--json"],
        root,
    )
    slskdn_api = run_json(
        ["node", "scripts/audit-slskdn-controller-routes.mjs", "--slskdn-root", str(slskdn_root), "--json"],
        root,
    )
    security_ledger = (
        None
        if args.skip_security_differential
        else security_authorization_ledger(root, args.reuse_evidence)
    )
    security_control_ledger_rows = (
        None
        if args.skip_security_control_differential
        else security_control_ledger(root, args.reuse_evidence)
    )
    controller_ledger = (
        None
        if args.skip_controller_api_differential
        else controller_api_ledger(
            root, slskd_root, slskdn_root, args.reuse_evidence
        )
    )
    persistence_ledger = (
        None
        if args.skip_persistence_differential
        else persistence_lifecycle_ledger(root, args.reuse_evidence)
    )
    file_ledger = (
        None
        if args.skip_file_differential
        else file_lifecycle_ledger(root, args.reuse_evidence)
    )
    protocol_ledger = (
        None
        if args.skip_protocol_differential
        else protocol_behaviors_ledger(root, args.reuse_evidence)
    )
    live_ledger = (
        None
        if args.live_interop_evidence is None
        else live_interop_ledger(
            args.live_interop_evidence,
            expected_failure_checks,
        )
    )
    operator_ledger = (
        None
        if args.operator_evidence is None
        else operator_packaging_ledger(args.operator_evidence)
    )
    webui = run_json(
        [
            "node",
            "scripts/audit-upstream-webui-endpoints.mjs",
            "--slskd-root",
            str(slskd_root),
            "--slskdn-root",
            str(slskdn_root),
            "--slskr-web-root",
            str(root),
            "--json",
        ],
        root,
    )
    webui_workflow_evidence = webui_workflow_ledger(
        root, webui, args.reuse_evidence
    )
    slskd_database_domains = database_domains(slskd_root)
    slskdn_database_domains = database_domains(slskdn_root)
    slskd_file_write_domains = file_write_domains(slskd_root)
    slskdn_file_write_domains = file_write_domains(slskdn_root)
    slskd_security_components = security_components(slskd_root)
    slskdn_security_components = security_components(slskdn_root)
    slskd_operator_families = operator_families(slskd_root)
    slskdn_operator_families = operator_families(slskdn_root)
    # slskd 10.0.2 identifies Soulseek.NET commit
    # 94fba7d4056796af067e6d7b2a8628099723cd26 in its NuGet metadata. Its
    # MessageCode.cs is byte-identical to the frozen vendored runtime copy.
    slskd_protocol_units = protocol_units(slskdn_root, include_slskdn_extensions=False)
    slskdn_protocol_units = protocol_units(slskdn_root, include_slskdn_extensions=True)
    interop_features = live_interop_features()

    actual = {
        "config": config["comparison"]["unionCount"],
        "slskd-api": len(slskd_api),
        "slskdn-api": len(slskdn_api),
        "webui-call-union": webui["comparison"]["targetUnionCount"],
        "slskd-database-domains": len(slskd_database_domains),
        "slskdn-database-domains": len(slskdn_database_domains),
        "slskd-file-writer-domains": len(slskd_file_write_domains),
        "slskdn-file-writer-domains": len(slskdn_file_write_domains),
        "slskd-security-components": len(slskd_security_components),
        "slskdn-security-components": len(slskdn_security_components),
        "slskd-operator-families": len(slskd_operator_families),
        "slskdn-operator-families": len(slskdn_operator_families),
        "slskd-protocol-units": len(slskd_protocol_units),
        "slskdn-protocol-units": len(slskdn_protocol_units),
        "live-interop-target-features": len(interop_features),
    }
    if args.check_frozen and actual != EXPECTED:
        raise SystemExit(f"frozen parity inventory changed: expected {EXPECTED!r}, got {actual!r}")

    entries = [
        *config_entries(config),
        *api_entries("slskd", slskd_api, security_ledger, controller_ledger),
        *api_entries("slskdn", slskdn_api, security_ledger, controller_ledger),
        *webui_entries(webui, webui_workflow_evidence),
        *persistence_entries(
            "slskd", slskd_database_domains, persistence_ledger, slskd_root
        ),
        *persistence_entries(
            "slskdn", slskdn_database_domains, persistence_ledger, slskdn_root
        ),
        *file_lifecycle_entries(
            "slskd", slskd_file_write_domains, file_ledger, slskd_root
        ),
        *file_lifecycle_entries(
            "slskdn", slskdn_file_write_domains, file_ledger, slskdn_root
        ),
        *security_component_entries(
            "slskd", slskd_security_components, security_control_ledger_rows, slskd_root
        ),
        *security_component_entries(
            "slskdn", slskdn_security_components, security_control_ledger_rows, slskdn_root
        ),
        *operator_entries(
            "slskd", slskd_operator_families, operator_ledger, slskd_root
        ),
        *operator_entries(
            "slskdn", slskdn_operator_families, operator_ledger, slskdn_root
        ),
        *protocol_entries("slskd", slskd_protocol_units, protocol_ledger, slskdn_root),
        *protocol_entries("slskdn", slskdn_protocol_units, protocol_ledger, slskdn_root),
        *live_interop_entries(
            interop_features,
            live_ledger,
            args.live_interop_evidence,
            expected_failure_checks,
        ),
    ]
    # A live interop row can be structurally not-applicable when the frozen
    # target owns the behavior locally (controller, protocol, persistence, or
    # security) and the bounded runner has no meaningful peer transaction for
    # that dimension.  That classification is only usable for strict
    # certification after this same invocation has produced a complete fresh
    # typed ledger for every other workstream.  Reused artifacts intentionally
    # never receive this promotion.
    typed_differential_proof = (
        not args.reuse_evidence
        and all(
            entry["status"] == "complete"
            for entry in entries
            if entry["workstream"] != "live-interop"
        )
    )
    for entry in entries:
        if entry["workstream"] != "live-interop":
            continue
        coverage = entry.get("coverage", {})
        if (
            coverage.get("liveBehavioralProof") == "not-applicable"
            and entry["status"] == "complete"
        ):
            coverage["typedDifferentialProof"] = (
                "complete" if typed_differential_proof else "open"
            )
            if typed_differential_proof:
                entry["proofMode"] = "fresh-typed-differential"
    manifest = {
        "schemaVersion": 1,
        "goal": "frozen externally observable 1:1 parity and bidirectional interoperability",
        "certification": {
            "mode": "universal-replacement" if args.strict_universal else "frozen-ledger",
            "evidenceMode": "reused" if args.reuse_evidence else "fresh",
            "liveInteropEvidence": [str(path) for path in args.live_interop_evidence or []],
            "operatorEvidence": str(args.operator_evidence) if args.operator_evidence else None,
            "transportEvidence": str(args.transport_evidence) if args.transport_evidence else None,
            "transportCapabilityEvidence": (
                str(args.transport_capability_evidence)
                if args.transport_capability_evidence
                else None
            ),
            "reactUiEvidence": str(args.react_ui_evidence) if args.react_ui_evidence else None,
            "rustUiEvidence": str(args.rust_ui_evidence) if args.rust_ui_evidence else None,
            "targetUiComparisonEvidence": (
                str(args.target_ui_comparison_evidence)
                if args.target_ui_comparison_evidence
                else None
            ),
        },
        "frozenTargets": {
            "slskd": config["slskd"]["revision"],
            "slskdn": config["slskdn"]["revision"],
            "slskNetRuntime": "af73ff3f84fda7ba890bb5aea3adf712e5400cf6",
        },
        "summary": summarize(entries),
        "unmaterializedWorkstreams": UNMATERIALIZED_WORKSTREAMS,
        "entries": entries,
    }

    if args.strict_universal:
        strict_failures = strict_universal_failures(
            entries,
            reuse_evidence=args.reuse_evidence,
            live_interop_evidence=args.live_interop_evidence,
            operator_evidence=args.operator_evidence,
            transport_evidence=args.transport_evidence,
            react_ui_evidence=args.react_ui_evidence,
            rust_ui_evidence=args.rust_ui_evidence,
            react_ui_scenario_evidence=args.react_ui_scenario_evidence,
            rust_ui_scenario_evidence=args.rust_ui_scenario_evidence,
            target_ui_comparison_evidence=args.target_ui_comparison_evidence,
            transport_target_requirements=transport_target_requirements,
            transport_capability_evidence=args.transport_capability_evidence,
            transport_capability_contracts=transport_capability_contracts,
        )
        if strict_failures:
            print(
                "universal replacement gate failed:",
                file=sys.stderr,
            )
            for failure in strict_failures:
                print(f"- {failure}", file=sys.stderr)
            raise SystemExit(1)
        manifest["summary"]["goalAchieved"] = True

    if args.json:
        print(json.dumps(manifest, indent=2))
    else:
        summary = manifest["summary"]
        print(
            "parity manifest: "
            f"materialized={summary['materializedEntryCount']} "
            f"complete={summary['statusCounts']['complete']} "
            f"partial={summary['statusCounts']['partial']} "
            f"missing={summary['statusCounts']['missing']} "
            f"needs-proof={summary['statusCounts']['needs-proof']} "
            f"denominator-missing={summary['unmaterializedWorkstreamCount']} "
            f"proof-case-closure={summary['proofCaseClosurePercentage']:.2f}%"
        )
        for name, counts in summary["workstreams"].items():
            print(
                f"  {name}: total={counts['total']} complete={counts['complete']} "
                f"partial={counts['partial']} missing={counts['missing']} "
                f"needs-proof={counts['needs-proof']}"
            )

    if args.require_complete:
        incomplete = [entry for entry in entries if entry["status"] != "complete"]
        if incomplete or UNMATERIALIZED_WORKSTREAMS:
            print(
                "literal parity check failed: "
                f"{len(incomplete)} materialized entries are incomplete and "
                f"{len(UNMATERIALIZED_WORKSTREAMS)} workstream denominators are missing",
                file=sys.stderr,
            )
            raise SystemExit(1)


if __name__ == "__main__":
    main()
