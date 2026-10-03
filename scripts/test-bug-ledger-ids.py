#!/usr/bin/env python3
"""Regression checks for duplicate and out-of-order bug ledger IDs."""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts" / "check-bug-ledger-ids.py"


def run(contents: str) -> subprocess.CompletedProcess[str]:
    with tempfile.TemporaryDirectory(prefix="slskr-bug-ledger-") as directory:
        ledger = Path(directory) / "ledger.md"
        ledger.write_text(contents, encoding="utf-8")
        return subprocess.run(
            [sys.executable, str(CHECKER), str(ledger)],
            check=False,
            capture_output=True,
            text=True,
        )


def main() -> int:
    header = "| ID | Class |\n| --- | --- |\n"
    valid = run(header + "| BUG-001 | first |\n| BUG-003 | second |\n")
    if valid.returncode != 0:
        raise SystemExit(f"valid ledger was rejected: {valid.stderr}")

    duplicate = run(header + "| BUG-001 | first |\n| BUG-001 | second |\n")
    if duplicate.returncode != 1 or "duplicate BUG-001" not in duplicate.stderr:
        raise SystemExit("duplicate ledger ID was not rejected")

    out_of_order = run(header + "| BUG-003 | first |\n| BUG-002 | second |\n")
    if out_of_order.returncode != 1 or "not after BUG-003" not in out_of_order.stderr:
        raise SystemExit("out-of-order ledger ID was not rejected")

    print("Bug ledger ID uniqueness/order regression passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
