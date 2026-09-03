#!/usr/bin/env python3
"""Compare two live slskR HTTP benchmark artifacts.

The comparison only evaluates like-for-like workloads. Thresholds are an
explicit policy supplied by the caller; this tool does not turn an arbitrary
run into a performance claim. Exit status is zero when all available checks
are within policy and two for invalid input or a detected regression.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any


REQUIRED_ARTIFACT_FIELDS = (
    "schemaVersion",
    "benchmark",
    "evidenceMode",
    "profile",
    "persistence",
    "durationSeconds",
    "warmupSeconds",
    "concurrency",
    "timeoutSeconds",
    "endpoints",
    "statusPolicy",
    "summary",
    "cases",
)
WORKLOAD_FIELDS = (
    "schemaVersion",
    "benchmark",
    "evidenceMode",
    "profile",
    "persistence",
    "durationSeconds",
    "warmupSeconds",
    "concurrency",
    "timeoutSeconds",
    "endpoints",
    "statusPolicy",
)
LATENCY_FIELDS = ("p50", "p95", "p99")


def load_artifact(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read benchmark artifact {path}: {error}") from error
    if not isinstance(value, dict):
        raise ValueError(f"benchmark artifact {path} must contain a JSON object")
    missing = [field for field in REQUIRED_ARTIFACT_FIELDS if field not in value]
    if missing:
        raise ValueError(f"benchmark artifact {path} is missing: {', '.join(missing)}")
    if value["schemaVersion"] != 1 or value["benchmark"] != "slskr-http":
        raise ValueError(f"benchmark artifact {path} is not schemaVersion 1 slskr-http")
    if value["evidenceMode"] != "live":
        raise ValueError(f"benchmark artifact {path} is not live evidence")
    if not isinstance(value["endpoints"], list) or not isinstance(value["statusPolicy"], dict):
        raise ValueError(f"benchmark artifact {path} has invalid workload metadata")
    if not isinstance(value["summary"], dict) or not isinstance(value["cases"], dict):
        raise ValueError(f"benchmark artifact {path} has invalid metrics")
    validate_metrics(value["summary"], f"{path}:summary")
    for label, metrics in value["cases"].items():
        if not isinstance(label, str) or not isinstance(metrics, dict):
            raise ValueError(f"benchmark artifact {path} has an invalid endpoint case")
        validate_metrics(metrics, f"{path}:case:{label}")
    return value


def validate_metrics(metrics: dict[str, Any], label: str) -> None:
    if not isinstance(metrics.get("latencyMs"), dict):
        raise ValueError(f"benchmark artifact {label} is missing latency metrics")
    for field in ("requests", "successes", "failures", "throughputRps"):
        if numeric(metrics.get(field)) is None:
            raise ValueError(f"benchmark artifact {label} has invalid {field}")


def workload_differences(
    baseline: dict[str, Any], candidate: dict[str, Any]
) -> list[dict[str, Any]]:
    differences = []
    for field in WORKLOAD_FIELDS:
        if baseline.get(field) != candidate.get(field):
            differences.append(
                {
                    "field": field,
                    "baseline": baseline.get(field),
                    "candidate": candidate.get(field),
                }
            )
    return differences


def numeric(value: Any) -> float | None:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    value = float(value)
    return value if math.isfinite(value) else None


def rate(metrics: dict[str, Any], field: str) -> float | None:
    requests = numeric(metrics.get("requests"))
    count = numeric(metrics.get(field))
    if requests is None or count is None or requests <= 0:
        return None
    return count / requests


def relative_change(baseline: float, candidate: float) -> float | None:
    if baseline == 0:
        return None
    return (candidate - baseline) / abs(baseline) * 100.0


def append_metric_check(
    checks: list[dict[str, Any]],
    regressions: list[dict[str, Any]],
    *,
    scope: str,
    metric: str,
    baseline: float | None,
    candidate: float | None,
    direction: str,
    threshold_percent: float,
) -> None:
    check: dict[str, Any] = {
        "scope": scope,
        "metric": metric,
        "direction": direction,
        "thresholdPercent": threshold_percent,
        "baseline": baseline,
        "candidate": candidate,
    }
    if baseline is None or candidate is None:
        check["status"] = "invalid"
        check["reason"] = "metric is absent or non-numeric in one artifact"
        checks.append(check)
        regressions.append(check)
        return
    change = relative_change(baseline, candidate)
    check["changePercent"] = round(change, 3) if change is not None else None
    if change is None:
        check["status"] = "invalid"
        check["reason"] = "baseline is zero"
        checks.append(check)
        regressions.append(check)
        return
    regression = (
        change > threshold_percent
        if direction == "lower-is-better"
        else change < -threshold_percent
    )
    check["status"] = "regression" if regression else "pass"
    checks.append(check)
    if regression:
        regressions.append(check)


def append_failure_check(
    checks: list[dict[str, Any]],
    regressions: list[dict[str, Any]],
    *,
    scope: str,
    baseline: float | None,
    candidate: float | None,
    threshold_points: float,
) -> None:
    check: dict[str, Any] = {
        "scope": scope,
        "metric": "failureRate",
        "direction": "lower-is-better",
        "thresholdPercentagePoints": threshold_points,
        "baseline": baseline,
        "candidate": candidate,
    }
    if baseline is None or candidate is None:
        check["status"] = "invalid"
        check["reason"] = "request or failure count is absent"
        checks.append(check)
        regressions.append(check)
        return
    increase_points = (candidate - baseline) * 100.0
    check["changePercentagePoints"] = round(increase_points, 3)
    regression = increase_points > threshold_points
    check["status"] = "regression" if regression else "pass"
    checks.append(check)
    if regression:
        regressions.append(check)


def compare(
    baseline: dict[str, Any],
    candidate: dict[str, Any],
    *,
    max_latency_regression_percent: float,
    max_throughput_regression_percent: float,
    max_failure_rate_increase_points: float,
) -> dict[str, Any]:
    differences = workload_differences(baseline, candidate)
    if differences:
        return {
            "schemaVersion": 1,
            "benchmark": "slskr-http-comparison",
            "evidenceMode": "comparison",
            "verdict": "invalid",
            "workloadDifferences": differences,
            "checks": [],
            "regressions": [],
        }

    checks: list[dict[str, Any]] = []
    regressions: list[dict[str, Any]] = []
    scopes = [("aggregate", baseline["summary"], candidate["summary"])]
    baseline_cases = baseline["cases"]
    candidate_cases = candidate["cases"]
    for label in sorted(set(baseline_cases) | set(candidate_cases)):
        if label not in baseline_cases or label not in candidate_cases:
            regressions.append(
                {
                    "scope": f"endpoint:{label}",
                    "metric": "casePresent",
                    "status": "regression",
                    "reason": "endpoint case is missing from one artifact",
                }
            )
            continue
        scopes.append((f"endpoint:{label}", baseline_cases[label], candidate_cases[label]))

    for scope, before, after in scopes:
        before_latency = before.get("latencyMs", {})
        after_latency = after.get("latencyMs", {})
        for field in LATENCY_FIELDS:
            append_metric_check(
                checks,
                regressions,
                scope=scope,
                metric=f"latencyMs.{field}",
                baseline=numeric(before_latency.get(field)),
                candidate=numeric(after_latency.get(field)),
                direction="lower-is-better",
                threshold_percent=max_latency_regression_percent,
            )
        append_metric_check(
            checks,
            regressions,
            scope=scope,
            metric="throughputRps",
            baseline=numeric(before.get("throughputRps")),
            candidate=numeric(after.get("throughputRps")),
            direction="higher-is-better",
            threshold_percent=max_throughput_regression_percent,
        )
        append_failure_check(
            checks,
            regressions,
            scope=scope,
            baseline=rate(before, "failures"),
            candidate=rate(after, "failures"),
            threshold_points=max_failure_rate_increase_points,
        )

    return {
        "schemaVersion": 1,
        "benchmark": "slskr-http-comparison",
        "evidenceMode": "comparison",
        "profile": baseline["profile"],
        "persistence": baseline["persistence"],
        "verdict": (
            "invalid"
            if any(check.get("status") == "invalid" for check in regressions)
            else "regression"
            if regressions
            else "pass"
        ),
        "workload": {field: baseline[field] for field in WORKLOAD_FIELDS},
        "thresholds": {
            "maxLatencyRegressionPercent": max_latency_regression_percent,
            "maxThroughputRegressionPercent": max_throughput_regression_percent,
            "maxFailureRateIncreasePercentagePoints": max_failure_rate_increase_points,
        },
        "checks": checks,
        "regressions": regressions,
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path, help="baseline live benchmark JSON")
    parser.add_argument("candidate", type=Path, help="candidate live benchmark JSON")
    parser.add_argument(
        "--max-latency-regression-percent",
        type=float,
        default=10.0,
        help="allowed relative p50/p95/p99 latency increase (default: 10)",
    )
    parser.add_argument(
        "--max-throughput-regression-percent",
        type=float,
        default=10.0,
        help="allowed relative throughput decrease (default: 10)",
    )
    parser.add_argument(
        "--max-failure-rate-increase-points",
        type=float,
        default=0.0,
        help="allowed failure-rate increase in percentage points (default: 0)",
    )
    parser.add_argument("--output", type=Path, help="write comparison JSON to this path")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    thresholds = (
        args.max_latency_regression_percent,
        args.max_throughput_regression_percent,
        args.max_failure_rate_increase_points,
    )
    if any(not math.isfinite(value) or value < 0 for value in thresholds):
        raise SystemExit("thresholds must be finite and non-negative")

    try:
        baseline = load_artifact(args.baseline)
        candidate = load_artifact(args.candidate)
        result = compare(
            baseline,
            candidate,
            max_latency_regression_percent=thresholds[0],
            max_throughput_regression_percent=thresholds[1],
            max_failure_rate_increase_points=thresholds[2],
        )
    except ValueError as error:
        print(f"benchmark comparison failed: {error}", file=sys.stderr)
        return 2

    encoded = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    else:
        sys.stdout.write(encoded)
    return 0 if result["verdict"] == "pass" else 2


if __name__ == "__main__":
    raise SystemExit(main())
