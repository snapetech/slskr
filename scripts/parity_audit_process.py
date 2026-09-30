"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


def run_json(command: list[str], cwd: Path) -> Any:
    command = guarded_process_command(command, cwd)
    completed = subprocess.run(
        command,
        cwd=cwd,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return json.loads(completed.stdout)


def run_logged(command: list[str], cwd: Path, env: dict[str, str] | None = None) -> None:
    """Run a proof command without contaminating the machine-readable manifest."""
    command = guarded_process_command(command, cwd)
    completed = subprocess.run(
        command,
        cwd=cwd,
        check=False,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if completed.stdout:
        print(completed.stdout, file=sys.stderr, end="")
    if completed.stderr:
        print(completed.stderr, file=sys.stderr, end="")
    if completed.returncode:
        raise subprocess.CalledProcessError(
            completed.returncode,
            command,
            output=completed.stdout,
            stderr=completed.stderr,
        )


def guarded_process_command(command: list[str], cwd: Path) -> list[str]:
    """Bound browser and frontend subprocesses before they can allocate."""
    if not command or Path(command[0]).name not in {"node", "nodejs", "npm", "npx"}:
        return command
    return [str(cwd / "scripts" / "with-process-memory-guard.sh"), *command]


def bounded_slskr_test_command(feature: str, selector: str | list[str]) -> list[str]:
    """Run one linked differential workstream without a Cargo test harness.

    The historical ``slskr`` test module is linked into a tiny feature-specific
    runner binary. ``selector`` remains part of this helper's call contract so
    the ledger functions continue to name the proof family they own; the Rust
    runner dispatches that family from its Cargo feature and does not pass the
    selector through to an all-tests harness.
    """
    del selector
    command = [
        "cargo",
        "run",
        "-p",
        "slskr",
        "--bin",
        "slskr-bounded-differential",
        "--no-default-features",
        "--features",
        feature,
    ]
    return command


def fresh_json_evidence_paths(evidence_dir: Path, started_ns: int) -> list[Path]:
    """Return only ledgers written or replaced by the current proof run.

    Retained evidence is explicitly supported only by ``--reuse-evidence``.
    A fresh run must not promote yesterday's JSON merely because the focused
    Cargo profile did not execute the optional monolithic test module.
    """
    if not evidence_dir.is_dir():
        return []
    paths = []
    for path in sorted(evidence_dir.glob("*.json")):
        try:
            if path.stat().st_mtime_ns >= started_ns:
                paths.append(path)
        except FileNotFoundError:
            continue
    return paths


def feature_family(subject: str) -> str:
    value = subject.split(" ", 1)[-1].strip("/")
    parts = [part for part in value.split("/") if part]
    while parts and (parts[0] == "api" or re.fullmatch(r"v(?:\d+|\{version\})", parts[0])):
        parts.pop(0)
    return parts[0].replace(":var", "parameter") if parts else "root"
