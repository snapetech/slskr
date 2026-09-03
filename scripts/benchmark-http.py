#!/usr/bin/env python3
"""Run a real, bounded HTTP benchmark against a running slskR daemon.

The benchmark deliberately uses only the Python standard library so it can be
run in the repository's existing test environments. It keeps one HTTP
connection per worker, records real response status and latency, and emits a
machine-readable JSON artifact. It is a measurement tool, not a parity test:
the endpoint list and status policy are supplied by the caller.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import dataclasses
import datetime as dt
import http.client
import json
import random
import socket
import statistics
import sys
import time
from pathlib import Path
from typing import Iterable
from urllib.parse import urlsplit


DEFAULT_ENDPOINTS = (
    "GET /api/health",
    "GET /api/version",
    "GET /api/capabilities",
    "GET /api/stats",
    "GET /api/transfers/downloads",
)
DEFAULT_MAX_SAMPLES = 20_000
DEFAULT_TIMEOUT_SECONDS = 10.0
StatusRange = tuple[int, int]


@dataclasses.dataclass(frozen=True)
class Endpoint:
    method: str
    target: str

    @property
    def label(self) -> str:
        return f"{self.method} {self.target}"


@dataclasses.dataclass
class Metrics:
    requests: int = 0
    successes: int = 0
    accepted_non_success: int = 0
    failures: int = 0
    response_bytes: int = 0
    status_counts: dict[str, int] = dataclasses.field(default_factory=dict)
    error_counts: dict[str, int] = dataclasses.field(default_factory=dict)
    latencies_ms: list[float] = dataclasses.field(default_factory=list)

    def record_status(
        self,
        status: int,
        latency_ms: float,
        response_bytes: int,
        max_samples: int,
        success_ranges: tuple[StatusRange, ...],
        allowed_statuses: frozenset[int],
    ) -> None:
        self.requests += 1
        self.response_bytes += response_bytes
        self.status_counts[str(status)] = self.status_counts.get(str(status), 0) + 1
        if status_matches(status, success_ranges):
            self.successes += 1
        elif status in allowed_statuses:
            self.accepted_non_success += 1
        else:
            self.failures += 1
        if len(self.latencies_ms) < max_samples:
            self.latencies_ms.append(latency_ms)

    def record_error(self, error: BaseException) -> None:
        self.requests += 1
        self.failures += 1
        key = type(error).__name__
        self.error_counts[key] = self.error_counts.get(key, 0) + 1

    def merge(self, other: "Metrics", max_samples: int) -> None:
        self.requests += other.requests
        self.successes += other.successes
        self.accepted_non_success += other.accepted_non_success
        self.failures += other.failures
        self.response_bytes += other.response_bytes
        for key, value in other.status_counts.items():
            self.status_counts[key] = self.status_counts.get(key, 0) + value
        for key, value in other.error_counts.items():
            self.error_counts[key] = self.error_counts.get(key, 0) + value
        if len(self.latencies_ms) < max_samples:
            remaining = max_samples - len(self.latencies_ms)
            self.latencies_ms.extend(other.latencies_ms[:remaining])


@dataclasses.dataclass
class WorkerResult:
    total: Metrics = dataclasses.field(default_factory=Metrics)
    by_endpoint: dict[str, Metrics] = dataclasses.field(default_factory=dict)

    def record_status(
        self,
        endpoint: Endpoint,
        status: int,
        latency_ms: float,
        response_bytes: int,
        max_samples: int,
        success_ranges: tuple[StatusRange, ...],
        allowed_statuses: frozenset[int],
    ) -> None:
        metrics = self.by_endpoint.setdefault(endpoint.label, Metrics())
        metrics.record_status(
            status,
            latency_ms,
            response_bytes,
            max_samples,
            success_ranges,
            allowed_statuses,
        )
        self.total.record_status(
            status,
            latency_ms,
            response_bytes,
            max_samples,
            success_ranges,
            allowed_statuses,
        )

    def record_error(self, endpoint: Endpoint, error: BaseException) -> None:
        metrics = self.by_endpoint.setdefault(endpoint.label, Metrics())
        metrics.record_error(error)
        self.total.record_error(error)

    def merge(self, other: "WorkerResult", max_samples: int) -> None:
        self.total.merge(other.total, max_samples)
        for label, metrics in other.by_endpoint.items():
            self.by_endpoint.setdefault(label, Metrics()).merge(metrics, max_samples)


def parse_endpoint(raw: str) -> Endpoint:
    fields = raw.strip().split(maxsplit=1)
    if len(fields) != 2:
        raise argparse.ArgumentTypeError(
            f"endpoint must be '<METHOD> <PATH>', got {raw!r}"
        )
    method, target = fields[0].upper(), fields[1]
    if method not in {"GET", "HEAD", "OPTIONS"}:
        raise argparse.ArgumentTypeError(
            f"benchmark endpoint method must be GET, HEAD, or OPTIONS: {method}"
        )
    if not target.startswith("/"):
        raise argparse.ArgumentTypeError(f"endpoint path must start with '/': {target}")
    return Endpoint(method, target)


def parse_status_range(raw: str) -> StatusRange:
    value = raw.strip()
    try:
        if "-" in value:
            first, last = value.split("-", maxsplit=1)
            start, end = int(first), int(last)
        else:
            start = end = int(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError(
            f"status must be a code or range such as 200-399: {raw!r}"
        ) from error
    if not 100 <= start <= end <= 599:
        raise argparse.ArgumentTypeError(f"status range is outside 100-599: {raw!r}")
    return start, end


def status_matches(status: int, ranges: tuple[StatusRange, ...]) -> bool:
    return any(start <= status <= end for start, end in ranges)


def percentile(samples: list[float], fraction: float) -> float | None:
    if not samples:
        return None
    ordered = sorted(samples)
    index = min(len(ordered) - 1, max(0, int((len(ordered) - 1) * fraction)))
    return round(ordered[index], 3)


def make_connection(parts, timeout: float) -> http.client.HTTPConnection:
    connection_type = (
        http.client.HTTPSConnection
        if parts.scheme == "https"
        else http.client.HTTPConnection
    )
    if not parts.hostname:
        raise ValueError("base URL must include a hostname")
    return connection_type(parts.hostname, parts.port, timeout=timeout)


def request_once(
    connection: http.client.HTTPConnection,
    endpoint: Endpoint,
    headers: dict[str, str],
) -> tuple[int, float, int]:
    started = time.perf_counter_ns()
    connection.request(endpoint.method, endpoint.target, headers=headers)
    response = connection.getresponse()
    body = response.read()
    latency_ms = (time.perf_counter_ns() - started) / 1_000_000
    return response.status, latency_ms, len(body)


def run_worker(
    worker_id: int,
    parts,
    endpoints: tuple[Endpoint, ...],
    headers: dict[str, str],
    deadline: float,
    warmup_deadline: float,
    timeout: float,
    max_samples: int,
    success_ranges: tuple[StatusRange, ...],
    allowed_statuses: frozenset[int],
) -> WorkerResult:
    result = WorkerResult()
    connection = make_connection(parts, timeout)
    random_source = random.Random(worker_id + 1)
    endpoint_index = worker_id % len(endpoints)
    try:
        while time.monotonic() < deadline:
            endpoint = endpoints[endpoint_index % len(endpoints)]
            endpoint_index += 1
            try:
                status, latency_ms, response_bytes = request_once(
                    connection, endpoint, headers
                )
                if time.monotonic() >= warmup_deadline:
                    result.record_status(
                        endpoint,
                        status,
                        latency_ms,
                        response_bytes,
                        max_samples,
                        success_ranges,
                        allowed_statuses,
                    )
            except (
                ConnectionError,
                OSError,
                http.client.HTTPException,
                socket.timeout,
            ) as error:
                if time.monotonic() >= warmup_deadline:
                    result.record_error(endpoint, error)
                connection.close()
                time.sleep(random_source.uniform(0.001, 0.01))
                connection = make_connection(parts, timeout)
    finally:
        connection.close()
    return result


def summarize(metrics: Metrics, elapsed_seconds: float, sample_limit: int) -> dict:
    samples = metrics.latencies_ms
    return {
        "requests": metrics.requests,
        "successes": metrics.successes,
        "acceptedNonSuccess": metrics.accepted_non_success,
        "failures": metrics.failures,
        "successRate": round(metrics.successes / metrics.requests, 6)
        if metrics.requests
        else 0.0,
        "nonFailureRate": round(
            (metrics.successes + metrics.accepted_non_success) / metrics.requests,
            6,
        )
        if metrics.requests
        else 0.0,
        "throughputRps": round(metrics.requests / elapsed_seconds, 3)
        if elapsed_seconds > 0
        else 0.0,
        "responseBytes": metrics.response_bytes,
        "latencyMs": {
            "samples": len(samples),
            "sampleLimit": sample_limit,
            "min": round(min(samples), 3) if samples else None,
            "mean": round(statistics.fmean(samples), 3) if samples else None,
            "p50": percentile(samples, 0.50),
            "p95": percentile(samples, 0.95),
            "p99": percentile(samples, 0.99),
            "max": round(max(samples), 3) if samples else None,
        },
        "statusCounts": dict(sorted(metrics.status_counts.items())),
        "errorCounts": dict(sorted(metrics.error_counts.items())),
    }


def parse_headers(raw_headers: Iterable[str]) -> dict[str, str]:
    headers = {"Accept": "application/json", "Connection": "keep-alive"}
    for raw in raw_headers:
        name, separator, value = raw.partition(":")
        if not separator or not name.strip():
            raise argparse.ArgumentTypeError(
                f"header must be '<name>: <value>', got {raw!r}"
            )
        headers[name.strip()] = value.strip()
    return headers


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--base-url",
        required=True,
        help="daemon base URL, e.g. http://127.0.0.1:5030",
    )
    parser.add_argument("--profile", default="unspecified", help="label for the output artifact")
    parser.add_argument(
        "--persistence", default="unspecified", help="label for the output artifact"
    )
    parser.add_argument("--duration", type=float, default=30.0, help="measurement duration in seconds")
    parser.add_argument("--warmup", type=float, default=5.0, help="warmup duration in seconds")
    parser.add_argument("--concurrency", type=int, default=8, help="persistent worker connections")
    parser.add_argument(
        "--timeout", type=float, default=DEFAULT_TIMEOUT_SECONDS, help="per-request socket timeout"
    )
    parser.add_argument(
        "--max-samples",
        type=int,
        default=DEFAULT_MAX_SAMPLES,
        help="latency samples retained per endpoint and aggregate",
    )
    parser.add_argument(
        "--endpoint",
        action="append",
        type=parse_endpoint,
        dest="endpoints",
        help="repeatable '<METHOD> <PATH>' workload entry; defaults to a safe read workload",
    )
    parser.add_argument(
        "--success-status",
        action="append",
        dest="success_status",
        help="accepted status code/range; defaults to 200-399",
    )
    parser.add_argument(
        "--allow-status",
        action="append",
        type=int,
        default=[],
        help="known non-success status to report but not fail the run, e.g. 429",
    )
    parser.add_argument("--header", action="append", default=[], help="additional HTTP header")
    parser.add_argument(
        "--bearer-token",
        help="set Authorization: Bearer <token> without writing it to output",
    )
    parser.add_argument("--output", type=Path, help="write JSON artifact to this path")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if args.duration <= 0 or args.warmup < 0:
        raise SystemExit("duration must be > 0 and warmup must be >= 0")
    if args.concurrency <= 0 or args.max_samples <= 0 or args.timeout <= 0:
        raise SystemExit("concurrency, max-samples, and timeout must be > 0")
    if any(not 100 <= status <= 599 for status in args.allow_status):
        raise SystemExit("allow-status values must be between 100 and 599")

    parts = urlsplit(args.base_url)
    if parts.scheme not in {"http", "https"} or not parts.hostname:
        raise SystemExit("base URL must be an http(s) URL with a hostname")
    if parts.path not in {"", "/"} or parts.query or parts.fragment:
        raise SystemExit("base URL must not contain a path, query, or fragment")

    endpoints = tuple(args.endpoints or [parse_endpoint(raw) for raw in DEFAULT_ENDPOINTS])
    headers = parse_headers(args.header)
    if args.bearer_token:
        headers["Authorization"] = f"Bearer {args.bearer_token}"
    success_ranges = tuple(
        parse_status_range(raw) for raw in (args.success_status or ["200-399"])
    )
    allowed_statuses = frozenset(args.allow_status)

    started_at = dt.datetime.now(dt.timezone.utc).isoformat()
    start = time.monotonic()
    warmup_deadline = start + args.warmup
    deadline = warmup_deadline + args.duration
    aggregate = WorkerResult()
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.concurrency) as executor:
        futures = [
            executor.submit(
                run_worker,
                worker_id,
                parts,
                endpoints,
                headers,
                deadline,
                warmup_deadline,
                args.timeout,
                args.max_samples,
                success_ranges,
                allowed_statuses,
            )
            for worker_id in range(args.concurrency)
        ]
        for future in futures:
            aggregate.merge(future.result(), args.max_samples)

    elapsed = max(0.000001, time.monotonic() - warmup_deadline)
    artifact = {
        "schemaVersion": 1,
        "benchmark": "slskr-http",
        "evidenceMode": "live",
        "startedAt": started_at,
        "baseUrl": f"{parts.scheme}://{parts.netloc}",
        "profile": args.profile,
        "persistence": args.persistence,
        "durationSeconds": args.duration,
        "warmupSeconds": args.warmup,
        "concurrency": args.concurrency,
        "timeoutSeconds": args.timeout,
        "endpoints": [endpoint.label for endpoint in endpoints],
        "statusPolicy": {
            "success": [f"{start}-{end}" for start, end in success_ranges],
            "allowedNonSuccess": sorted(allowed_statuses),
        },
        "summary": summarize(aggregate.total, elapsed, args.max_samples),
        "cases": {
            label: summarize(metrics, elapsed, args.max_samples)
            for label, metrics in sorted(aggregate.by_endpoint.items())
        },
    }
    encoded = json.dumps(artifact, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    else:
        sys.stdout.write(encoded)
    return 0 if aggregate.total.failures == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main())
