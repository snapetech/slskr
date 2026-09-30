#!/usr/bin/env python3
"""Compare query plans and page latency at deterministic SQLite cardinalities.

The fixture uses the production table columns and read projections, but its
rows are synthetic. It compares existing indexes, all three original RF-021
composite indexes, and the retained pair after removing the redundant
search-result index.
"""

from __future__ import annotations

import argparse
import json
import platform
import sqlite3
import sys
import tempfile
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
BATCH_SIZE = 10_000
SEARCH_GROUPS = 1_000
WEBHOOK_GROUPS = 100
QUEUED_INTERVAL = 100

SCHEMA = """
CREATE TABLE search_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    search_id TEXT NOT NULL,
    peer_username TEXT,
    filename TEXT NOT NULL,
    size INTEGER NOT NULL,
    extension TEXT NOT NULL,
    bit_rate INTEGER,
    sample_rate INTEGER,
    bit_depth INTEGER,
    length_seconds INTEGER,
    locked INTEGER NOT NULL,
    slot_free INTEGER,
    average_speed INTEGER,
    queue_length INTEGER,
    created_at INTEGER NOT NULL
);
CREATE TABLE transfers (
    id TEXT PRIMARY KEY,
    direction TEXT NOT NULL,
    filename TEXT NOT NULL,
    peer_username TEXT NOT NULL,
    filesize INTEGER NOT NULL,
    progress INTEGER DEFAULT 0,
    status TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    request_id TEXT,
    wishlist_item_id TEXT,
    request_name TEXT,
    destination_directory TEXT,
    local_path TEXT,
    batch_id TEXT,
    reason TEXT,
    bit_rate INTEGER,
    sample_rate INTEGER,
    bit_depth INTEGER,
    length_seconds INTEGER,
    artist TEXT,
    album TEXT,
    title TEXT,
    track_number INTEGER,
    year INTEGER,
    attempts INTEGER NOT NULL DEFAULT 1,
    auto_replace_attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at INTEGER,
    updated_at_ms INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE webhook_logs (
    id TEXT PRIMARY KEY,
    webhook_id TEXT NOT NULL,
    event TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    status TEXT NOT NULL,
    request_body TEXT NOT NULL,
    response_status INTEGER,
    response_body TEXT,
    error_message TEXT,
    attempt INTEGER DEFAULT 1,
    timestamp INTEGER NOT NULL
);
CREATE INDEX idx_search_results_search ON search_results(search_id);
CREATE INDEX idx_transfers_started ON transfers(started_at DESC);
CREATE INDEX idx_webhook_logs_webhook ON webhook_logs(webhook_id);
CREATE INDEX idx_webhook_logs_timestamp ON webhook_logs(timestamp DESC);
"""

ALL_COMPOSITE_INDEXES = (
    "CREATE INDEX idx_search_results_search_id "
    "ON search_results(search_id, id)",
    "CREATE INDEX idx_transfers_status_started "
    "ON transfers(status, started_at DESC)",
    "CREATE INDEX idx_webhook_logs_webhook_timestamp "
    "ON webhook_logs(webhook_id, timestamp DESC)",
)
RETAINED_COMPOSITE_INDEXES = ALL_COMPOSITE_INDEXES[1:]

QUERIES = (
    (
        "search-results-first-page",
        "SELECT id, search_id, peer_username, filename, size, extension, "
        "bit_rate, sample_rate, bit_depth, length_seconds, locked, slot_free, "
        "average_speed, queue_length, created_at FROM search_results "
        "WHERE search_id = 'search-0000' ORDER BY id LIMIT 100 OFFSET 0",
    ),
    (
        "search-results-deep-page",
        "SELECT id, search_id, peer_username, filename, size, extension, "
        "bit_rate, sample_rate, bit_depth, length_seconds, locked, slot_free, "
        "average_speed, queue_length, created_at FROM search_results "
        "WHERE search_id = 'search-0000' ORDER BY id LIMIT 100 OFFSET 500",
    ),
    (
        "queued-transfers-first-page",
        "SELECT id, direction, filename, peer_username, filesize, progress, "
        "status, started_at, completed_at, request_id, wishlist_item_id, "
        "request_name, destination_directory, local_path, batch_id, reason, "
        "bit_rate, sample_rate, bit_depth, length_seconds, artist, album, "
        "title, track_number, year, attempts, auto_replace_attempts, "
        "next_attempt_at, updated_at_ms FROM transfers WHERE status = 'queued' "
        "ORDER BY started_at DESC LIMIT 100 OFFSET 0",
    ),
    (
        "queued-transfers-deep-page",
        "SELECT id, direction, filename, peer_username, filesize, progress, "
        "status, started_at, completed_at, request_id, wishlist_item_id, "
        "request_name, destination_directory, local_path, batch_id, reason, "
        "bit_rate, sample_rate, bit_depth, length_seconds, artist, album, "
        "title, track_number, year, attempts, auto_replace_attempts, "
        "next_attempt_at, updated_at_ms FROM transfers WHERE status = 'queued' "
        "ORDER BY started_at DESC LIMIT 100 OFFSET 1000",
    ),
    (
        "webhook-logs-first-page",
        "SELECT id, webhook_id, event, correlation_id, status, request_body, "
        "response_status, response_body, error_message, attempt, timestamp "
        "FROM webhook_logs WHERE webhook_id = 'webhook-000' "
        "ORDER BY timestamp DESC LIMIT 100 OFFSET 0",
    ),
    (
        "webhook-logs-deep-page",
        "SELECT id, webhook_id, event, correlation_id, status, request_body, "
        "response_status, response_body, error_message, attempt, timestamp "
        "FROM webhook_logs WHERE webhook_id = 'webhook-000' "
        "ORDER BY timestamp DESC LIMIT 100 OFFSET 1000",
    ),
)


def load_profiler() -> Any:
    path = ROOT / "scripts" / "profile-sqlite.py"
    spec = spec_from_file_location("slskr_profile_sqlite", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not load SQLite profiler at {path}")
    module = module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def insert_batches(
    connection: sqlite3.Connection,
    count: int,
    make_row: Any,
    statement: str,
) -> None:
    for start in range(0, count, BATCH_SIZE):
        end = min(count, start + BATCH_SIZE)
        connection.executemany(
            statement,
            (make_row(index) for index in range(start, end)),
        )


def search_result(index: int) -> tuple[Any, ...]:
    return (
        f"search-{index % SEARCH_GROUPS:04d}",
        f"peer-{index % 10_000:05d}",
        f"Music/Track-{index:09d}.flac",
        4_000_000 + index % 500_000,
        "flac",
        320,
        44_100,
        16,
        240,
        index % 2,
        1,
        256_000,
        index % 8,
        1_700_000_000 + index,
    )


def transfer(index: int) -> tuple[Any, ...]:
    queued = index % QUEUED_INTERVAL == 0
    return (
        f"transfer-{index:09d}",
        "download" if index % 2 == 0 else "upload",
        f"Music/Transfer-{index:09d}.flac",
        f"peer-{index % 10_000:05d}",
        4_000_000 + index % 500_000,
        0 if queued else 100,
        "queued" if queued else "completed",
        index,
        None if queued else index + 240,
        f"request-{index:09d}",
        None,
        None,
        "/music",
        f"/music/Transfer-{index:09d}.flac",
        None,
        None,
        320,
        44_100,
        16,
        240,
        "Artist",
        "Album",
        f"Track {index}",
        index % 12 + 1,
        2024,
        1,
        0,
        None,
        index,
    )


def webhook_log(index: int) -> tuple[Any, ...]:
    return (
        f"log-{index:09d}",
        f"webhook-{index % WEBHOOK_GROUPS:03d}",
        "transfer.completed",
        f"correlation-{index:09d}",
        "delivered" if index % 20 else "failed",
        '{"event":"transfer.completed","payload":"' + ("x" * 160) + '"}',
        200 if index % 20 else 503,
        '{"status":"accepted","detail":"' + ("y" * 64) + '"}',
        None if index % 20 else "temporary delivery failure",
        1,
        index,
    )


def create_baseline_database(
    path: Path,
    *,
    search_results: int,
    transfers: int,
    webhook_logs: int,
) -> None:
    connection = sqlite3.connect(path)
    try:
        connection.execute("PRAGMA journal_mode = OFF")
        connection.execute("PRAGMA synchronous = OFF")
        connection.executescript(SCHEMA)
        insert_batches(
            connection,
            search_results,
            search_result,
            "INSERT INTO search_results "
            "(search_id, peer_username, filename, size, extension, bit_rate, "
            "sample_rate, bit_depth, length_seconds, locked, slot_free, "
            "average_speed, queue_length, created_at) "
            "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        insert_batches(
            connection,
            transfers,
            transfer,
            "INSERT INTO transfers "
            "(id, direction, filename, peer_username, filesize, progress, "
            "status, started_at, completed_at, request_id, wishlist_item_id, "
            "request_name, destination_directory, local_path, batch_id, reason, "
            "bit_rate, sample_rate, bit_depth, length_seconds, artist, album, "
            "title, track_number, year, attempts, auto_replace_attempts, "
            "next_attempt_at, updated_at_ms) "
            "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, "
            "?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        insert_batches(
            connection,
            webhook_logs,
            webhook_log,
            "INSERT INTO webhook_logs "
            "(id, webhook_id, event, correlation_id, status, request_body, "
            "response_status, response_body, error_message, attempt, timestamp) "
            "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        connection.execute("ANALYZE")
        connection.commit()
    finally:
        connection.close()


def copy_indexed_database(
    baseline_path: Path,
    indexed_path: Path,
    indexes: tuple[str, ...],
) -> None:
    with sqlite3.connect(baseline_path) as baseline:
        with sqlite3.connect(indexed_path) as indexed:
            baseline.backup(indexed)
            indexed.execute("PRAGMA journal_mode = OFF")
            for statement in indexes:
                indexed.execute(statement)
            indexed.execute("ANALYZE")


def database_size(path: Path) -> dict[str, int]:
    with sqlite3.connect(path) as connection:
        page_count = connection.execute("PRAGMA page_count").fetchone()[0]
        page_size = connection.execute("PRAGMA page_size").fetchone()[0]
    return {
        "bytes": page_count * page_size,
        "pageCount": page_count,
        "pageSizeBytes": page_size,
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Benchmark RF-021 indexes using deterministic synthetic SQLite data."
        )
    )
    parser.add_argument("--search-results", type=int, default=1_000_000)
    parser.add_argument("--transfers", type=int, default=500_000)
    parser.add_argument("--webhook-logs", type=int, default=500_000)
    parser.add_argument("--warmup", type=int, default=3)
    parser.add_argument("--iterations", type=int, default=20)
    parser.add_argument("--output", type=Path)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if min(args.search_results, args.transfers, args.webhook_logs) <= 0:
        print("row counts must be positive", file=sys.stderr)
        return 2
    if (
        args.transfers < 110_000
        or args.search_results < 600_000
        or args.webhook_logs < 110_000
    ):
        print(
            "use at least 600,000 search results, 110,000 transfers, and "
            "110,000 webhook logs so every selected query returns a full page",
            file=sys.stderr,
        )
        return 2
    if args.warmup < 0 or args.iterations <= 0:
        print(
            "warmup must be non-negative and iterations must be positive",
            file=sys.stderr,
        )
        return 2

    profiler = load_profiler()
    cases = [profiler.QueryCase(name, statement) for name, statement in QUERIES]
    with tempfile.TemporaryDirectory(prefix="slskr-sqlite-cardinality-") as temporary:
        directory = Path(temporary)
        baseline_path = directory / "legacy-indexes.db"
        all_indexes_path = directory / "all-composite-indexes.db"
        retained_indexes_path = directory / "retained-composite-indexes.db"
        create_baseline_database(
            baseline_path,
            search_results=args.search_results,
            transfers=args.transfers,
            webhook_logs=args.webhook_logs,
        )
        copy_indexed_database(baseline_path, all_indexes_path, ALL_COMPOSITE_INDEXES)
        copy_indexed_database(
            baseline_path,
            retained_indexes_path,
            RETAINED_COMPOSITE_INDEXES,
        )

        before = profiler.profile_database(
            baseline_path,
            cases,
            warmup_iterations=args.warmup,
            measured_iterations=args.iterations,
        )
        all_composites = profiler.profile_database(
            all_indexes_path,
            cases,
            warmup_iterations=args.warmup,
            measured_iterations=args.iterations,
        )
        retained = profiler.profile_database(
            retained_indexes_path,
            cases,
            warmup_iterations=args.warmup,
            measured_iterations=args.iterations,
        )

        before["database"] = (
            "synthetic production-shaped database; existing indexes only"
        )
        all_composites["database"] = (
            "same synthetic rows; all original RF-021 composite indexes added"
        )
        retained["database"] = (
            "same synthetic rows; transfer and webhook RF-021 indexes retained"
        )
        before["evidenceMode"] = "synthetic-cardinality"
        all_composites["evidenceMode"] = "synthetic-cardinality"
        retained["evidenceMode"] = "synthetic-cardinality"
        before_size = database_size(baseline_path)
        all_composites_size = database_size(all_indexes_path)
        retained_size = database_size(retained_indexes_path)

        deltas = {}
        for name, _ in QUERIES:
            before_median = before["cases"][name]["latencyMs"]["median"]
            deltas[name] = {}
            for label, profile in (
                ("allCompositeIndexes", all_composites),
                ("retainedCompositeIndexes", retained),
            ):
                after_median = profile["cases"][name]["latencyMs"]["median"]
                deltas[name][label] = {
                    "beforeMedianMs": before_median,
                    "afterMedianMs": after_median,
                    "medianDeltaPercent": round(
                        (after_median / before_median - 1.0) * 100.0, 3
                    )
                    if before_median
                    else None,
                }

        result = {
            "schemaVersion": 1,
            "benchmark": "slskr-sqlite-index-cardinality",
            "evidenceMode": "synthetic-cardinality",
            "host": {
                "os": platform.system(),
                "release": platform.release(),
                "architecture": platform.machine(),
                "python": platform.python_version(),
                "sqlite": sqlite3.sqlite_version,
            },
            "dataset": {
                "searchResults": args.search_results,
                "searchIds": SEARCH_GROUPS,
                "resultsPerSearchAverage": args.search_results / SEARCH_GROUPS,
                "transfers": args.transfers,
                "queuedTransferPercent": 100 / QUEUED_INTERVAL,
                "webhookLogs": args.webhook_logs,
                "webhookIds": WEBHOOK_GROUPS,
                "logsPerWebhookAverage": args.webhook_logs / WEBHOOK_GROUPS,
                "limit": 100,
                "deepPageOffset": 1000,
            },
            "warmupIterations": args.warmup,
            "measuredIterations": args.iterations,
            "databaseSize": {
                "existingIndexes": before_size,
                "withAllCompositeIndexes": all_composites_size,
                "withRetainedCompositeIndexes": retained_size,
                "allIndexesAddedBytes": all_composites_size["bytes"]
                - before_size["bytes"],
                "redundantSearchIndexBytes": all_composites_size["bytes"]
                - retained_size["bytes"],
            },
            "medianDeltas": deltas,
            "existingIndexes": before,
            "withAllCompositeIndexes": all_composites,
            "withRetainedCompositeIndexes": retained,
        }

    encoded = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    sys.stdout.write(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
