"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any
from parity_audit_shared import (
    UNIVERSAL_TRANSPORT_TARGETS,
    UNIVERSAL_UI_SCENARIOS,
    UNIVERSAL_UI_WORKFLOWS,
)


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
