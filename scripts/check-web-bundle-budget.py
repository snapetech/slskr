#!/usr/bin/env python3
"""Enforce explicit size budgets for the generated Web UI assets."""

from __future__ import annotations

import argparse
import gzip
import json
import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit


DEFAULT_MAX_KIB = 700
DEFAULT_INITIAL_JS_MAX_KIB = 1150
DEFAULT_GZIP_MAX_KIB = 600
GZIP_COMPRESS_LEVEL = 9
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


def _asset_reference_path(build_dir: Path, assets_dir: Path, reference: str) -> Path:
    parsed = urlsplit(reference)
    if parsed.scheme or parsed.netloc:
        raise ValueError(f"initial JavaScript reference is not local: {reference}")
    candidate = (build_dir / unquote(parsed.path).lstrip("/")).resolve()
    if candidate.parent != assets_dir.resolve():
        raise ValueError(f"initial JavaScript reference is outside assets: {reference}")
    return candidate


def initial_js_assets(build_dir: Path, assets: list[Path]) -> list[Path]:
    """Return the JavaScript files requested by the built HTML entry point."""

    js_assets = [path for path in assets if path.suffix == ".js"]
    index_path = build_dir / "index.html"
    if not index_path.is_file():
        # Keep direct callers and the historical fixture shape useful while real
        # builds still use the explicit entry/preload graph below.
        return js_assets

    html = index_path.read_text(encoding="utf-8")
    references = re.findall(r"(?:src|href)\s*=\s*['\"]([^'\"]+)['\"]", html)
    assets_by_path = {path.resolve(): path for path in js_assets}
    selected = []
    seen = set()
    for reference in references:
        if not urlsplit(reference).path.endswith(".js"):
            continue
        candidate = _asset_reference_path(build_dir, build_dir / "assets", reference)
        asset = assets_by_path.get(candidate)
        if asset is None:
            raise ValueError(f"initial JavaScript asset is missing: {reference}")
        if candidate not in seen:
            selected.append(asset)
            seen.add(candidate)
    return selected or js_assets


def gzip_size(path: Path) -> int:
    """Return a reproducible gzip size independent of the file mtime."""

    return len(
        gzip.compress(
            path.read_bytes(),
            compresslevel=GZIP_COMPRESS_LEVEL,
            mtime=0,
        )
    )


def inspect_build(
    build_dir: Path,
    *,
    max_kib: int = DEFAULT_MAX_KIB,
    budgets: dict[str, int] | None = None,
    initial_js_max_kib: int = DEFAULT_INITIAL_JS_MAX_KIB,
    gzip_max_kib: int = DEFAULT_GZIP_MAX_KIB,
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
    if max_kib <= 0 or initial_js_max_kib <= 0 or gzip_max_kib <= 0:
        raise ValueError("all bundle budgets must be greater than zero")
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
    javascript_assets = [path for path in assets if path.suffix == ".js"]
    initial_assets = initial_js_assets(build_dir, assets)
    initial_js_bytes = sum(path.stat().st_size for path in initial_assets)
    initial_js_gzip_bytes = sum(gzip_size(path) for path in initial_assets)
    gzip_bytes = sum(gzip_size(path) for path in javascript_assets)
    initial_js_violation = {
        "check": "initial-js",
        "bytes": initial_js_bytes,
        "sizeKiB": round(initial_js_bytes / 1024, 2),
        "budgetKiB": initial_js_max_kib,
        "assets": [path.name for path in initial_assets],
    }
    gzip_violation = {
        "check": "gzip-js",
        "bytes": gzip_bytes,
        "sizeKiB": round(gzip_bytes / 1024, 2),
        "budgetKiB": gzip_max_kib,
        "assets": [path.name for path in javascript_assets],
    }
    aggregate_violations = []
    if initial_js_bytes / 1024 > initial_js_max_kib:
        aggregate_violations.append(initial_js_violation)
    if gzip_bytes / 1024 > gzip_max_kib:
        aggregate_violations.append(gzip_violation)
    return {
        "schemaVersion": 1,
        "build": str(build_dir.resolve()),
        "maxBudgetKiB": max_kib,
        "budgets": budgets,
        "initialJsAssets": [path.name for path in initial_assets],
        "initialJsBytes": initial_js_bytes,
        "initialJsKiB": round(initial_js_bytes / 1024, 2),
        "initialJsBudgetKiB": initial_js_max_kib,
        "initialJsGzipBytes": initial_js_gzip_bytes,
        "initialJsGzipKiB": round(initial_js_gzip_bytes / 1024, 2),
        "gzipCompressionLevel": GZIP_COMPRESS_LEVEL,
        "gzipBytes": gzip_bytes,
        "gzipKiB": round(gzip_bytes / 1024, 2),
        "gzipBudgetKiB": gzip_max_kib,
        "assets": records,
        "violations": violations,
        "aggregateViolations": aggregate_violations,
        "passed": not violations and not aggregate_violations,
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-dir", type=Path, required=True)
    parser.add_argument("--max-kib", type=int, default=DEFAULT_MAX_KIB)
    parser.add_argument(
        "--initial-js-max-kib",
        type=int,
        default=DEFAULT_INITIAL_JS_MAX_KIB,
        help="maximum aggregate raw size for JavaScript loaded by index.html",
    )
    parser.add_argument(
        "--gzip-max-kib",
        type=int,
        default=DEFAULT_GZIP_MAX_KIB,
        help="maximum aggregate gzip size for all JavaScript assets",
    )
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
    if args.max_kib <= 0 or args.initial_js_max_kib <= 0 or args.gzip_max_kib <= 0:
        raise SystemExit("all bundle budgets must be greater than zero")
    try:
        report = inspect_build(
            args.build_dir,
            max_kib=args.max_kib,
            budgets=dict(args.budget),
            initial_js_max_kib=args.initial_js_max_kib,
            gzip_max_kib=args.gzip_max_kib,
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
    for violation in report["aggregateViolations"]:
        print(
            f"{violation['check']}: {violation['sizeKiB']} KiB exceeds "
            f"{violation['budgetKiB']} KiB",
            file=sys.stderr,
        )
    return 2 if not report["passed"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
