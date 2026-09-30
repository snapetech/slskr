"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

import collections
import json
from pathlib import Path
from typing import Any
from parity_audit_shared import (
    REACT_WEB_UI_CASE_COUNT,
    REACT_WEB_UI_ROUTE_COUNT,
    UNMATERIALIZED_WORKSTREAMS,
)
from parity_audit_transport import (
    universal_transport_failures,
)
from parity_audit_ui_evidence import (
    universal_target_ui_comparison_failures,
    universal_ui_scenario_failures,
)


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
