#!/usr/bin/env python3
"""Profile explicit read-only SQLite queries with their real query plans.

This is a measurement tool, not an index generator. Statements are supplied
by the caller so a result can be tied to a known controller operation and
compared against the same database shape before and after a change.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import sqlite3
import statistics
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import quote
from typing import Any


DEFAULT_WARMUP_ITERATIONS = 2
DEFAULT_MEASURED_ITERATIONS = 10
QUERY_NAME = re.compile(r"[A-Za-z][A-Za-z0-9_.-]{0,63}\Z")


@dataclass(frozen=True)
class QueryCase:
    name: str
    statement: str


def parse_query(raw: str) -> QueryCase:
    name, separator, statement = raw.partition("=")
    name = name.strip()
    statement = statement.strip()
    if not separator or not QUERY_NAME.fullmatch(name):
        raise ValueError(
            "query must use NAME=SELECT ... with a 1-64 character name "
            "containing letters, digits, '.', '-', or '_'"
        )
    if not statement or ";" in statement:
        raise ValueError(f"query {name!r} must be one statement without ';'")
    first_token = statement.split(None, 1)[0].upper()
    if first_token != "SELECT":
        raise ValueError(f"query {name!r} must start with SELECT")
    return QueryCase(name, statement)


def percentile(samples: list[float], fraction: float) -> float:
    ordered = sorted(samples)
    index = min(len(ordered) - 1, max(0, int((len(ordered) - 1) * fraction)))
    return round(ordered[index], 3)


def explain_query_plan(connection: sqlite3.Connection, statement: str) -> list[dict[str, Any]]:
    # SQLite cannot bind a complete statement as a parameter. `parse_query`
    # restricts callers to one SELECT without semicolons, while the database
    # connection is opened read-only and has query_only enabled below.
    # nosemgrep: python.lang.security.audit.formatted-sql-query.formatted-sql-query,python.sqlalchemy.security.sqlalchemy-execute-raw-query.sqlalchemy-execute-raw-query
    explain_statement = f"EXPLAIN QUERY PLAN {statement}"
    rows = connection.execute(explain_statement).fetchall()  # nosemgrep: python.lang.security.audit.formatted-sql-query.formatted-sql-query,python.sqlalchemy.security.sqlalchemy-execute-raw-query.sqlalchemy-execute-raw-query
    return [
        {
            "id": row[0],
            "parent": row[1],
            "notUsed": row[2],
            "detail": row[3],
        }
        for row in rows
    ]


def profile_case(
    connection: sqlite3.Connection,
    case: QueryCase,
    *,
    warmup_iterations: int,
    measured_iterations: int,
) -> dict[str, Any]:
    plan = explain_query_plan(connection, case.statement)
    for _ in range(warmup_iterations):
        connection.execute(case.statement).fetchall()

    durations: list[float] = []
    row_counts: list[int] = []
    for _ in range(measured_iterations):
        started = time.perf_counter_ns()
        rows = connection.execute(case.statement).fetchall()
        durations.append((time.perf_counter_ns() - started) / 1_000_000)
        row_counts.append(len(rows))

    return {
        "statement": case.statement,
        "plan": plan,
        "rows": {
            "first": row_counts[0],
            "minimum": min(row_counts),
            "maximum": max(row_counts),
        },
        "latencyMs": {
            "minimum": round(min(durations), 3),
            "median": round(statistics.median(durations), 3),
            "p95": percentile(durations, 0.95),
            "maximum": round(max(durations), 3),
        },
    }


def profile_database(
    database: Path,
    cases: list[QueryCase],
    *,
    warmup_iterations: int = DEFAULT_WARMUP_ITERATIONS,
    measured_iterations: int = DEFAULT_MEASURED_ITERATIONS,
) -> dict[str, Any]:
    if warmup_iterations < 0:
        raise ValueError("warmup iterations must be non-negative")
    if measured_iterations <= 0:
        raise ValueError("measured iterations must be greater than zero")
    if not cases:
        raise ValueError("at least one query is required")
    names = [case.name for case in cases]
    if len(names) != len(set(names)):
        raise ValueError("query names must be unique")

    resolved_database = database.resolve(strict=True)
    if not resolved_database.is_file():
        raise ValueError(f"database is not a regular file: {resolved_database}")
    database_uri = f"file:{quote(str(resolved_database), safe='/')}?mode=ro"
    connection = sqlite3.connect(database_uri, uri=True)
    try:
        connection.execute("PRAGMA busy_timeout = 5000")
        connection.execute("PRAGMA query_only = ON")
        started_at = dt.datetime.now(dt.timezone.utc).isoformat()
        results = {
            case.name: profile_case(
                connection,
                case,
                warmup_iterations=warmup_iterations,
                measured_iterations=measured_iterations,
            )
            for case in cases
        }
        return {
            "schemaVersion": 1,
            "benchmark": "slskr-sqlite",
            "evidenceMode": "live",
            "startedAt": started_at,
            "database": str(resolved_database),
            "sqliteVersion": sqlite3.sqlite_version,
            "warmupIterations": warmup_iterations,
            "measuredIterations": measured_iterations,
            "cases": results,
        }
    finally:
        connection.close()


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Profile explicit read-only SQLite statements and query plans."
    )
    parser.add_argument("--database", type=Path, required=True)
    parser.add_argument(
        "--query",
        action="append",
        required=True,
        metavar="NAME=SELECT",
        help="read-only SELECT statement; repeat for each measured case",
    )
    parser.add_argument("--warmup", type=int, default=DEFAULT_WARMUP_ITERATIONS)
    parser.add_argument(
        "--iterations", type=int, default=DEFAULT_MEASURED_ITERATIONS
    )
    parser.add_argument("--output", type=Path)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    try:
        cases = [parse_query(raw) for raw in args.query]
        artifact = profile_database(
            args.database,
            cases,
            warmup_iterations=args.warmup,
            measured_iterations=args.iterations,
        )
    except (OSError, sqlite3.Error, ValueError) as error:
        print(f"SQLite profiling failed: {error}", file=sys.stderr)
        return 2

    encoded = json.dumps(artifact, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    else:
        sys.stdout.write(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
