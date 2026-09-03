#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("compare-benchmark.py")
SPEC = importlib.util.spec_from_file_location("compare_benchmark", SCRIPT)
assert SPEC and SPEC.loader
compare_benchmark = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(compare_benchmark)


def artifact(*, latency: float = 10.0, throughput: float = 100.0, failures: int = 0):
    metrics = {
        "failures": failures,
        "latencyMs": {"p50": latency, "p95": latency, "p99": latency},
        "requests": 100,
        "successes": 100 - failures,
        "throughputRps": throughput,
    }
    return {
        "schemaVersion": 1,
        "benchmark": "slskr-http",
        "evidenceMode": "live",
        "profile": "native",
        "persistence": "disabled",
        "durationSeconds": 30.0,
        "warmupSeconds": 5.0,
        "concurrency": 8,
        "timeoutSeconds": 10.0,
        "endpoints": ["GET /api/health"],
        "statusPolicy": {"success": ["200-399"], "allowedNonSuccess": []},
        "summary": metrics,
        "cases": {"GET /api/health": metrics.copy()},
    }


class CompareBenchmarkTests(unittest.TestCase):
    def test_like_for_like_metrics_pass(self) -> None:
        result = compare_benchmark.compare(
            artifact(),
            artifact(latency=10.5, throughput=95.0),
            max_latency_regression_percent=10.0,
            max_throughput_regression_percent=10.0,
            max_failure_rate_increase_points=0.0,
        )
        self.assertEqual(result["verdict"], "pass")
        self.assertFalse(result["regressions"])

    def test_latency_and_failure_regressions_are_reported(self) -> None:
        result = compare_benchmark.compare(
            artifact(),
            artifact(latency=12.0, failures=1),
            max_latency_regression_percent=10.0,
            max_throughput_regression_percent=10.0,
            max_failure_rate_increase_points=0.0,
        )
        self.assertEqual(result["verdict"], "regression")
        metrics = {(check["scope"], check["metric"]) for check in result["regressions"]}
        self.assertIn(("aggregate", "latencyMs.p50"), metrics)
        self.assertIn(("aggregate", "failureRate"), metrics)

    def test_workload_mismatch_is_invalid(self) -> None:
        candidate = artifact()
        candidate["concurrency"] = 2
        result = compare_benchmark.compare(
            artifact(),
            candidate,
            max_latency_regression_percent=10.0,
            max_throughput_regression_percent=10.0,
            max_failure_rate_increase_points=0.0,
        )
        self.assertEqual(result["verdict"], "invalid")
        self.assertEqual(result["workloadDifferences"][0]["field"], "concurrency")

    def test_cli_writes_machine_readable_result(self) -> None:
        with tempfile.TemporaryDirectory(prefix="slskr-benchmark-compare-") as directory:
            directory = Path(directory)
            baseline_path = directory / "baseline.json"
            candidate_path = directory / "candidate.json"
            output_path = directory / "comparison.json"
            baseline_path.write_text(json.dumps(artifact()), encoding="utf-8")
            candidate_path.write_text(json.dumps(artifact()), encoding="utf-8")
            subprocess.check_call(
                [
                    sys.executable,
                    str(SCRIPT),
                    str(baseline_path),
                    str(candidate_path),
                    "--output",
                    str(output_path),
                ]
            )
            self.assertEqual(json.loads(output_path.read_text())["verdict"], "pass")


if __name__ == "__main__":
    unittest.main()
