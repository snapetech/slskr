#!/usr/bin/env python3
"""Hash the tracked inputs read by the candidate scan."""

from __future__ import annotations

import hashlib
import os
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
INPUTS = (
    "crates",
    "web",
    "dashboard",
    "client-ts",
    "client-python",
    "client-go",
    "docs",
    "README.md",
)
GENERATED_RECORDS = {
    "docs/dev/council-scan-inventory.md",
    "docs/dev/bug-council-active-backlog.md",
}


def main() -> None:
    paths = subprocess.check_output(
        ["git", "ls-files", "-z", "--", *INPUTS], cwd=ROOT
    ).split(b"\0")
    digest = hashlib.sha256()
    for raw_path in paths:
        if not raw_path:
            continue
        path = os.fsdecode(raw_path)
        if path in GENERATED_RECORDS:
            continue
        file = ROOT / path
        digest.update(len(raw_path).to_bytes(8, "big"))
        digest.update(raw_path)
        if file.is_symlink():
            content = os.fsencode(os.readlink(file))
            digest.update(len(content).to_bytes(8, "big"))
            digest.update(content)
            continue
        size = file.stat().st_size
        digest.update(size.to_bytes(8, "big"))
        with file.open("rb") as handle:
            while chunk := handle.read(1024 * 1024):
                digest.update(chunk)
    print(digest.hexdigest())


if __name__ == "__main__":
    main()
