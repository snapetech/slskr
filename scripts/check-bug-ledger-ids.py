#!/usr/bin/env python3
"""Require unique, strictly increasing IDs in the canonical bug ledger."""

from __future__ import annotations

import re
import sys
from pathlib import Path


def check(path: Path) -> list[str]:
    errors: list[str] = []
    seen: set[str] = set()
    previous = 0
    row_count = 0

    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        match = re.match(r"^\| (BUG-([0-9]{3,})) \|", line)
        if match is None:
            continue
        bug_id, digits = match.groups()
        value = int(digits)
        row_count += 1
        if bug_id in seen:
            errors.append(f"{path}:{line_number}: duplicate {bug_id}")
        if value <= previous:
            errors.append(f"{path}:{line_number}: {bug_id} is not after BUG-{previous:03d}")
        seen.add(bug_id)
        previous = value

    if row_count == 0:
        errors.append(f"{path}: no bug ledger rows found")
    return errors


def main() -> int:
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("docs/dev/bug-burndown-ledger.md")
    if len(sys.argv) > 2:
        print("usage: check-bug-ledger-ids.py [ledger.md]", file=sys.stderr)
        return 2
    if not path.is_file():
        print(f"bug ledger ID check failed: missing file {path}", file=sys.stderr)
        return 2
    errors = check(path)
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    row_count = sum(
        1
        for line in path.read_text(encoding="utf-8").splitlines()
        if re.match(r"^\| BUG-[0-9]{3,} \|", line)
    )
    print(f"Bug ledger IDs passed: {row_count} rows are unique and increasing.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
