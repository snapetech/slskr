"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any
from urllib.parse import unquote
from parity_audit_process import (
    feature_family,
    guarded_process_command,
)


def validate_ui_workflow_audit(audit: dict[str, Any]) -> None:
    count = audit.get("endpointSweepCount")
    if type(count) is not int or count != 0:
        raise RuntimeError("React WebUI workflow evidence must declare zero synthetic endpoint sweeps; regenerate the audit")


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
            validate_ui_workflow_audit(audit)
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
        environment.pop("SLSKR_REACT_WEB_AUDIT_ENDPOINT_SWEEP", None)
        if not environment.get("SLSKR_PLAYWRIGHT_EXECUTABLE_PATH"):
            chromium = shutil.which("chromium") or shutil.which("chromium-browser")
            if chromium:
                environment["SLSKR_PLAYWRIGHT_EXECUTABLE_PATH"] = chromium
        environment.update(
            {
                "SLSKR_REACT_WEB_AUDIT_DIR": str(scenario_dir),
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
        validate_ui_workflow_audit(audit)
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
