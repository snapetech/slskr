"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

import csv
import json
from pathlib import Path
from typing import Any
from parity_audit_shared import (
    UNIVERSAL_BIDIRECTIONAL_TRANSPORTS,
    UNIVERSAL_LIFECYCLE_CHECK,
    UNIVERSAL_LIFECYCLE_SCENARIOS,
    UNIVERSAL_TRANSPORT_LIFECYCLE_DETAIL_TOKENS,
    UNIVERSAL_TRANSPORT_LIFECYCLE_REQUIREMENTS,
    UNIVERSAL_TRANSPORT_TARGETS,
)


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
