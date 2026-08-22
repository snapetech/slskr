#!/usr/bin/env python3
"""Derive the strict transport artifact from explicit live TSV evidence.

This tool is deliberately conservative.  It maps only checks that exercise a
known direction and records every missing direction as a failed check.  It
never turns a local unit test, a one-sided probe, or a target-local endpoint
call into bidirectional transport evidence.  A direction may be classified as
not applicable only by a separate, source-bound capability artifact.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import json
from pathlib import Path
from typing import Any


TRANSPORTS: dict[str, dict[str, Any]] = {
    "soulseek-peer-bidirectional": {
        "targets": {"slskd", "slskdn"},
        "rows": {
            "slskd": {
                "slskr-to-target": ("protocol-slskr-message-dispatch-slskd",),
                "target-to-slskr": ("protocol-slskd-message-dispatch",),
            },
            "slskdn": {
                "slskr-to-target": ("protocol-slskr-message-dispatch",),
                "target-to-slskr": ("protocol-slskdn-message-dispatch",),
            },
        },
    },
    "obfuscated-peer-bidirectional": {
        "targets": {"slskdn"},
        "rows": {
            "slskdn": {
                "slskr-to-target": ("protocol-slskr-obfuscated-peer-slskdn",),
                "target-to-slskr": ("protocol-slskdn-obfuscated-peer-slskr",),
            }
        },
    },
            "distributed-dht-bidirectional": {
                "targets": {"slskd", "slskdn"},
                "rows": {
                    "slskd": {
                        "slskr-to-target": ("protocol-slskr-distributed-peer-slskd",),
                        "target-to-slskr": ("protocol-slskd-distributed-peer-slskr",),
                    },
                    "slskdn": {
                        "slskr-to-target": ("protocol-slskr-distributed-peer-slskdn",),
                        "target-to-slskr": ("protocol-slskdn-distributed-peer-slskr",),
                    },
                },
            },
    "overlay-udp-bidirectional": {
        "targets": {"slskdn"},
        "rows": {
            "slskdn": {
                "slskr-to-target": ("protocol-slskr-overlay-udp-slskdn",),
                "target-to-slskr": (),
            }
        },
    },
    "overlay-quic-control-bidirectional": {
        "targets": {"slskdn"},
        "rows": {
            "slskdn": {
                "slskr-to-target": ("protocol-slskr-overlay-quic-control-slskdn",),
                "target-to-slskr": (),
            }
        },
    },
    "quic-data-bidirectional": {
        "targets": {"slskdn"},
        "rows": {
            "slskdn": {
                "slskr-to-target": ("protocol-slskr-quic-data-slskdn",),
                "target-to-slskr": (),
            }
        },
    },
    "relay-gateway-bidirectional": {
        "targets": {"slskdn"},
        "rows": {
            "slskdn": {
                "slskr-to-target": (
                    "protocol-slskr-gateway-open-slskdn",
                    "protocol-slskr-gateway-send-slskdn",
                    "protocol-slskr-gateway-receive-slskdn",
                    "protocol-slskr-gateway-close-slskdn",
                ),
                "target-to-slskr": (),
            }
        },
    },
    "mesh-sync-bidirectional": {
        "targets": {"slskdn"},
        "rows": {
            "slskdn": {
                "slskr-to-target": (
                    "protocol-ksdn-probe-dispatch",
                    "protocol-ksdn-slskdn-receives-hello",
                    "protocol-ksdn-slskdn-persists-slskr-descriptor",
                ),
                "target-to-slskr": (
                    "protocol-ksdn-slskr-receives-ack",
                    "protocol-ksdn-slskr-verifies-slskdn-descriptor",
                ),
            }
        },
    },
    "virtualsoulfind-bidirectional": {
        "targets": {"slskdn"},
        "rows": {"slskdn": {"slskr-to-target": (), "target-to-slskr": ()}},
    },
    "file-stream-transfer-bidirectional": {
        "targets": {"slskd", "slskdn"},
        "rows": {
            "slskd": {
                "slskr-to-target": ("slskr-to-slskd-download",),
                "target-to-slskr": ("slskd-to-slskr-download",),
            },
            "slskdn": {
                "slskr-to-target": ("slskr-to-slskdn-download",),
                "target-to-slskr": ("slskdn-to-slskr-download",),
            },
        },
    },
}

LIFECYCLE_CHECK = "failure-restart-lifecycle-matrix"
LIFECYCLE_TARGETS = ("slskd", "slskdn")
LIFECYCLE_SCENARIOS = (
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
)

# Some frozen transports have a controller-visible lifecycle contract in
# addition to the directional exchange.  Keep those rows explicit so a
# generic lifecycle matrix cannot silently stand in for the transport's own
# retry/reconnect behavior.  The pinned slskdN mesh-sync service has no
# outbound transport and returns the same generic 400 after repeated attempts
# in both profiles.
TRANSPORT_LIFECYCLE_REQUIREMENTS: dict[str, dict[str, dict[str, tuple[str, ...]]]] = {
    "mesh-sync-bidirectional": {
        "slskdn": {
            "reconnect-retry-and-failure": (
                "protocol-ksdn-mesh-sync-reconnect-retry",
            )
        }
    }
}
TRANSPORT_LIFECYCLE_DETAIL_TOKENS = {
    "protocol-ksdn-mesh-sync-reconnect-retry": (
        'expected-target-negative status=400 body={"error":"Failed to sync with peer"}'
    )
}


def read_tsv(path: Path) -> dict[str, dict[str, str]]:
    if not path.is_file():
        raise SystemExit(f"live interop TSV does not exist: {path}")
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        expected = ["timestamp", "check", "status", "detail"]
        if reader.fieldnames != expected:
            raise SystemExit(
                f"{path} must have TSV columns: {', '.join(expected)}"
            )
        rows: dict[str, dict[str, str]] = {}
        for row in reader:
            check = row.get("check", "")
            if not check:
                raise SystemExit(f"{path} contains a row without a check name")
            if check in rows:
                raise SystemExit(f"{path} contains duplicate check: {check}")
            rows[check] = row
        return rows


def read_tsv_with_supplements(
    path: Path, supplements: list[Path] | None = None
) -> tuple[dict[str, dict[str, str]], list[Path]]:
    """Read one authoritative TSV and fill absent checks from supplements.

    Supplements are intentionally fill-only: an existing authoritative row is
    never replaced by a later run.  This lets a focused rerun add a newly
    introduced lifecycle contract without allowing unrelated transient failures
    to overwrite a previously complete transport ledger.
    """
    rows = read_tsv(path)
    source_paths = [path]
    for supplement in supplements or []:
        for check, row in read_tsv(supplement).items():
            rows.setdefault(check, row)
        source_paths.append(supplement)
    return rows, source_paths


def direction_result(
    rows: dict[str, dict[str, str]], candidates: tuple[str, ...]
) -> tuple[bool, list[str], str]:
    if not candidates:
        return False, [], "no mapped live transaction"
    observed = [candidate for candidate in candidates if candidate in rows]
    passed = [candidate for candidate in observed if rows[candidate].get("status") == "ok"]
    if len(passed) == len(candidates):
        return True, passed, "all mapped live transactions passed"
    if observed:
        failed = [candidate for candidate in observed if candidate not in passed]
        return False, observed, "failed live transaction(s): " + ", ".join(failed)
    return False, [], "mapped live transaction is absent"


def transport_lifecycle_result(
    rows: dict[str, dict[str, str]], candidates: tuple[str, ...]
) -> tuple[bool, list[str], str]:
    if not candidates:
        return False, [], "no mapped transport lifecycle transaction"
    observed = [candidate for candidate in candidates if candidate in rows]
    passed = [
        candidate
        for candidate in observed
        if rows[candidate].get("status") == "fail"
        and TRANSPORT_LIFECYCLE_DETAIL_TOKENS.get(candidate, "")
        in rows[candidate].get("detail", "")
    ]
    if len(passed) == len(candidates):
        return True, passed, "all mapped transport lifecycle transactions passed"
    if observed:
        failed = [candidate for candidate in candidates if candidate not in passed]
        return False, observed, "failed transport lifecycle transaction(s): " + ", ".join(failed)
    return False, [], "mapped transport lifecycle transaction is absent"


def read_capability_evidence(
    path: Path | None,
) -> dict[tuple[str, str, str], dict[str, Any]]:
    """Read explicit target-capability exceptions for strict evidence.

    This file cannot grant an exception by itself.  The strict auditor binds
    every exception to the exact frozen target source and to the live rows it
    names.  Keeping the file explicit prevents an absent probe from silently
    becoming a green transport direction.
    """
    if path is None:
        return {}
    if not path.is_file():
        raise SystemExit(f"transport capability evidence does not exist: {path}")
    try:
        evidence = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"transport capability evidence is not valid JSON: {error}") from error
    if not isinstance(evidence, dict) or evidence.get("evidenceMode") != "live":
        raise SystemExit("transport capability evidence must declare evidenceMode=live")
    if evidence.get("schemaVersion") != 1:
        raise SystemExit("transport capability evidence must declare schemaVersion=1")
    records = evidence.get("checks")
    if not isinstance(records, list) or not records:
        raise SystemExit("transport capability evidence must contain checks")

    expanded: dict[tuple[str, str, str], dict[str, Any]] = {}
    required_directions = {"slskr-to-target", "target-to-slskr"}
    for index, record in enumerate(records):
        if not isinstance(record, dict):
            raise SystemExit(f"transport capability check {index} must be an object")
        check_id = record.get("id")
        target = record.get("target")
        directions = record.get("directions")
        if not isinstance(check_id, str) or not check_id:
            raise SystemExit(f"transport capability check {index} has no id")
        if not isinstance(target, str) or not target:
            raise SystemExit(f"transport capability check {check_id} has no target")
        if (
            record.get("status") != "not-applicable"
            or not isinstance(directions, list)
            or not directions
            or not set(directions).issubset(required_directions)
        ):
            raise SystemExit(
                f"transport capability check {check_id}/{target} must explicitly declare status=not-applicable and valid directions"
            )
        reason = record.get("reason")
        evidence_checks = record.get("evidenceChecks")
        evidence_artifacts = record.get("evidenceArtifacts")
        if not isinstance(reason, str) or not reason.strip():
            raise SystemExit(f"transport capability check {check_id}/{target} has no reason")
        if not isinstance(evidence_checks, list) or not evidence_checks:
            raise SystemExit(f"transport capability check {check_id}/{target} has no evidenceChecks")
        if not all(isinstance(item, str) and item for item in evidence_checks):
            raise SystemExit(f"transport capability check {check_id}/{target} has invalid evidenceChecks")
        if not isinstance(evidence_artifacts, list) or not evidence_artifacts:
            raise SystemExit(f"transport capability check {check_id}/{target} has no evidenceArtifacts")
        for artifact in evidence_artifacts:
            if not isinstance(artifact, str) or not Path(artifact).is_file():
                raise SystemExit(
                    f"transport capability check {check_id}/{target} names a missing evidence artifact: {artifact}"
                )
        for direction in directions:
            key = (check_id, target, direction)
            if key in expanded:
                raise SystemExit(
                    f"transport capability evidence contains duplicate: {check_id}/{target}/{direction}"
                )
            expanded[key] = record
    return expanded


def read_lifecycle(path: Path) -> dict[str, Any]:
    if not path.is_file():
        raise SystemExit(f"live lifecycle evidence does not exist: {path}")
    try:
        evidence = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"live lifecycle evidence is not valid JSON: {error}") from error
    if not isinstance(evidence, dict):
        raise SystemExit("live lifecycle evidence must contain a JSON object")
    if evidence.get("evidenceMode") != "live":
        raise SystemExit("live lifecycle evidence must declare evidenceMode=live")
    if evidence.get("id") != LIFECYCLE_CHECK:
        raise SystemExit(f"live lifecycle evidence must have id={LIFECYCLE_CHECK}")
    if evidence.get("status") != "pass":
        raise SystemExit("live lifecycle evidence must declare status=pass")
    if set(evidence.get("targets", [])) != set(LIFECYCLE_TARGETS):
        raise SystemExit("live lifecycle evidence must cover slskd and slskdn")
    if set(evidence.get("scenarios", [])) != set(LIFECYCLE_SCENARIOS):
        raise SystemExit("live lifecycle evidence must cover all required scenarios")
    target_scenarios = evidence.get("targetScenarios")
    if not isinstance(target_scenarios, dict):
        raise SystemExit("live lifecycle evidence must declare targetScenarios")
    for target in LIFECYCLE_TARGETS:
        if set(target_scenarios.get(target, [])) != set(LIFECYCLE_SCENARIOS):
            raise SystemExit(f"live lifecycle evidence must cover every scenario for {target}")

    artifacts = evidence.get("evidenceArtifacts")
    if not isinstance(artifacts, list) or not artifacts:
        raise SystemExit("live lifecycle evidence must name evidenceArtifacts")
    for artifact in artifacts:
        if not isinstance(artifact, str) or not Path(artifact).is_file():
            raise SystemExit(f"live lifecycle evidence names a missing artifact: {artifact}")

    cases = evidence.get("cases")
    expected_cases = {(target, scenario) for target in LIFECYCLE_TARGETS for scenario in LIFECYCLE_SCENARIOS}
    observed_cases: set[tuple[str, str]] = set()
    if not isinstance(cases, list):
        raise SystemExit("live lifecycle evidence must contain per-case records")
    for case in cases:
        if not isinstance(case, dict):
            raise SystemExit("live lifecycle evidence contains a non-object case")
        target = case.get("target")
        scenario = case.get("scenario")
        pair = (target, scenario)
        if pair in observed_cases:
            raise SystemExit(f"live lifecycle evidence contains duplicate case: {target}/{scenario}")
        if pair not in expected_cases:
            raise SystemExit(f"live lifecycle evidence contains an unknown case: {target}/{scenario}")
        observed_cases.add(pair)
        if case.get("status") != "pass":
            raise SystemExit(f"live lifecycle case is not pass: {target}/{scenario}")
        case_artifacts = case.get("evidenceArtifacts")
        if not isinstance(case_artifacts, list) or not case_artifacts:
            raise SystemExit(f"live lifecycle case must name evidence artifacts: {target}/{scenario}")
        for artifact in case_artifacts:
            if not isinstance(artifact, str) or not Path(artifact).is_file():
                raise SystemExit(
                    f"live lifecycle case names a missing evidence artifact: {target}/{scenario}: {artifact}"
                )
    missing_cases = sorted(expected_cases - observed_cases)
    if missing_cases:
        raise SystemExit(
            "live lifecycle evidence is missing cases: "
            + ", ".join(f"{target}/{scenario}" for target, scenario in missing_cases)
        )
    return evidence


def derive(
    tsv_by_target: dict[str, Path],
    lifecycle_evidence: Path | None = None,
    capability_evidence: Path | None = None,
    supplemental_tsv_by_target: dict[str, list[Path]] | None = None,
) -> dict[str, Any]:
    supplemental_tsv_by_target = supplemental_tsv_by_target or {}
    rows_by_target: dict[str, dict[str, dict[str, str]]] = {}
    source_tsv_paths: list[Path] = []
    for target, path in tsv_by_target.items():
        rows_by_target[target], source_paths = read_tsv_with_supplements(
            path, supplemental_tsv_by_target.get(target)
        )
        source_tsv_paths.extend(source_paths)
    capability_by_direction = read_capability_evidence(capability_evidence)
    checks: list[dict[str, Any]] = []
    for check_id, contract in TRANSPORTS.items():
        required_targets = set(contract["targets"])
        target_directions: dict[str, list[str]] = {}
        not_applicable_directions: dict[str, list[str]] = {}
        not_applicable_reasons: dict[str, dict[str, str]] = {}
        not_applicable_evidence_checks: dict[str, dict[str, list[str]]] = {}
        evidence: list[str] = []
        details: list[str] = []
        complete = True
        for target in sorted(required_targets):
            rows = rows_by_target[target]
            target_directions[target] = []
            for direction in ("slskr-to-target", "target-to-slskr"):
                passed, observed, detail = direction_result(
                    rows, contract["rows"][target][direction]
                )
                if passed:
                    target_directions[target].append(direction)
                elif (check_id, target, direction) in capability_by_direction:
                    capability = capability_by_direction[(check_id, target, direction)]
                    not_applicable_directions.setdefault(target, []).append(direction)
                    not_applicable_reasons.setdefault(target, {})[direction] = capability["reason"]
                    not_applicable_evidence_checks.setdefault(target, {})[direction] = list(
                        capability["evidenceChecks"]
                    )
                    evidence.extend(capability["evidenceChecks"])
                    details.append(
                        f"{target}/{direction}: not applicable: {capability['reason']}"
                    )
                else:
                    complete = False
                details.append(f"{target}/{direction}: {detail}")
                evidence.extend(observed)

        lifecycle_requirements = TRANSPORT_LIFECYCLE_REQUIREMENTS.get(check_id, {})
        lifecycle_records: dict[str, dict[str, Any]] = {}
        lifecycle_complete = True
        for target, scenarios in lifecycle_requirements.items():
            rows = rows_by_target[target]
            target_records: dict[str, Any] = {}
            for scenario, candidates in scenarios.items():
                passed, observed, detail = transport_lifecycle_result(rows, candidates)
                target_records[scenario] = {
                    "status": "pass" if passed else "fail",
                    "evidenceChecks": observed,
                    "detail": detail,
                }
                lifecycle_complete = lifecycle_complete and passed
                details.append(f"{target}/{scenario}: {detail}")
                evidence.extend(observed)
            lifecycle_records[target] = target_records
        complete = complete and lifecycle_complete

        capability_artifacts = [str(capability_evidence)] if capability_evidence else []
        for (capability_check, capability_target, _), capability in capability_by_direction.items():
            if capability_check == check_id and capability_target in required_targets:
                capability_artifacts.extend(
                    str(artifact) for artifact in capability["evidenceArtifacts"]
                )
        accepted_directions = {
            direction
            for directions in target_directions.values()
            for direction in directions
        }
        accepted_directions.update(
            direction
            for directions in not_applicable_directions.values()
            for direction in directions
        )
        check_record = {
                "id": check_id,
                "status": "pass" if complete else "fail",
                "targets": sorted(required_targets),
                "notApplicableTargets": sorted({"slskd", "slskdn"} - required_targets),
                "notApplicableDirections": not_applicable_directions,
                "notApplicableReasons": not_applicable_reasons,
                "notApplicableEvidenceChecks": not_applicable_evidence_checks,
                "directions": sorted(accepted_directions),
                "targetDirections": target_directions,
                "evidenceChecks": sorted(set(evidence)),
                "evidenceArtifacts": sorted(
                    set(
                        [str(tsv_by_target[target]) for target in sorted(required_targets)]
                        + capability_artifacts
                    )
                ),
                "detail": "; ".join(details),
            }
        if lifecycle_requirements:
            check_record["lifecycleStatus"] = "pass" if lifecycle_complete else "fail"
            check_record["lifecycleTargets"] = lifecycle_records
        checks.append(check_record)

    if lifecycle_evidence is None:
        checks.append(
            {
                "id": LIFECYCLE_CHECK,
                "status": "fail",
                "targets": [],
                "notApplicableTargets": [],
                "directions": [],
                "targetDirections": {},
                "scenarios": [],
                "targetScenarios": {target: [] for target in LIFECYCLE_TARGETS},
                "evidenceChecks": [],
                "evidenceArtifacts": [],
                "detail": "No live lifecycle runner is mapped; local differential cases cannot certify upgrade, rollback, permissions, or uninstall.",
            }
        )
    else:
        checks.append(read_lifecycle(lifecycle_evidence))
    return {
        "schemaVersion": 1,
        "evidenceMode": "live",
        "generatedAt": dt.datetime.now(dt.timezone.utc).isoformat(),
        "derivation": "scripts/derive-universal-transport-evidence.py",
        "sourceArtifacts": [str(path) for path in source_tsv_paths]
        + ([str(lifecycle_evidence)] if lifecycle_evidence is not None else []),
        "capabilityEvidence": str(capability_evidence) if capability_evidence else None,
        "checks": checks,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--slskd-tsv", type=Path, required=True)
    parser.add_argument("--slskdn-tsv", type=Path, required=True)
    parser.add_argument(
        "--slskdn-supplement",
        type=Path,
        action="append",
        default=[],
        help=(
            "Optional live slskdN TSV whose checks fill only names absent from "
            "--slskdn-tsv; existing authoritative rows are never replaced."
        ),
    )
    parser.add_argument(
        "--lifecycle-evidence",
        type=Path,
        required=True,
        help="Fresh live lifecycle matrix JSON covering both targets and all 22 target/scenario cases.",
    )
    parser.add_argument(
        "--capability-evidence",
        type=Path,
        help=(
            "Optional live source-bound target-capability JSON. It may classify only "
            "explicitly named transport directions as not applicable."
        ),
    )
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    evidence = derive(
        {"slskd": args.slskd_tsv, "slskdn": args.slskdn_tsv},
        args.lifecycle_evidence,
        args.capability_evidence,
        {"slskdn": args.slskdn_supplement},
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    passed = sum(check["status"] == "pass" for check in evidence["checks"])
    total = len(evidence["checks"])
    print(f"transport evidence: {passed}/{total} checks pass; output={args.output}")
    if passed != total:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
