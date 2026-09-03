#!/usr/bin/env python3
"""Enforce explicit size budgets for the generated Web UI assets."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


DEFAULT_MAX_KIB = 700
DEFAULT_BUDGETS = {
    "System": 360,
    "MediaCore": 450,
    "index": 250,
    "vendor": 700,
}


def asset_label(path: Path) -> str:
    return path.name.split("-", maxsplit=1)[0]


def parse_budget(raw: str) -> tuple[str, int]:
    name, separator, value = raw.partition("=")
    if not separator or not name or not value:
        raise argparse.ArgumentTypeError("budget must use NAME=KIB")
    try:
        kib = int(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("budget must use an integer KiB value") from error
    if kib <= 0:
        raise argparse.ArgumentTypeError("budget must be greater than zero")
    return name, kib


def inspect_build(
    build_dir: Path,
    *,
    max_kib: int = DEFAULT_MAX_KIB,
    budgets: dict[str, int] | None = None,
) -> dict:
    assets_dir = build_dir / "assets"
    if not assets_dir.is_dir():
        raise ValueError(f"missing build assets directory: {assets_dir}")
    assets = sorted(
        path
        for path in assets_dir.iterdir()
        if path.is_file() and path.suffix in {".js", ".css"}
    )
    if not assets:
        raise ValueError(f"no JavaScript or CSS assets found in {assets_dir}")
    budgets = {**DEFAULT_BUDGETS, **(budgets or {})}
    records = []
    violations = []
    for path in assets:
        label = asset_label(path)
        limit_kib = budgets.get(label, max_kib)
        size_bytes = path.stat().st_size
        size_kib = size_bytes / 1024
        record = {
            "asset": path.name,
            "label": label,
            "bytes": size_bytes,
            "sizeKiB": round(size_kib, 2),
            "budgetKiB": limit_kib,
        }
        records.append(record)
        if size_kib > limit_kib:
            violations.append(record)
    records.sort(key=lambda record: record["bytes"], reverse=True)
    return {
        "schemaVersion": 1,
        "build": str(build_dir.resolve()),
        "maxBudgetKiB": max_kib,
        "budgets": budgets,
        "assets": records,
        "violations": violations,
        "passed": not violations,
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-dir", type=Path, required=True)
    parser.add_argument("--max-kib", type=int, default=DEFAULT_MAX_KIB)
    parser.add_argument(
        "--budget",
        action="append",
        type=parse_budget,
        default=[],
        metavar="NAME=KIB",
        help="override the budget for an asset prefix; repeatable",
    )
    parser.add_argument("--output", type=Path)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if args.max_kib <= 0:
        raise SystemExit("max-kib must be greater than zero")
    try:
        report = inspect_build(
            args.build_dir,
            max_kib=args.max_kib,
            budgets=dict(args.budget),
        )
    except (OSError, ValueError) as error:
        print(f"Web bundle budget check failed: {error}", file=sys.stderr)
        return 2

    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")
    if report["violations"]:
        for violation in report["violations"]:
            print(
                f"{violation['asset']}: {violation['sizeKiB']} KiB exceeds "
                f"{violation['budgetKiB']} KiB",
                file=sys.stderr,
            )
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
