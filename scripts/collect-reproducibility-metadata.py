#!/usr/bin/env python3
"""Write and optionally validate local reproducibility metadata."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REQUIRED_TOOLS = ("rustc", "cargo", "node", "npm", "python3")
PROVENANCE_FILES = (
    "Cargo.lock",
    "package-lock.json",
    "web/package-lock.json",
    "dashboard/package-lock.json",
    "client-ts/package-lock.json",
    "client-go/go.sum",
    "client-python/pyproject.toml",
)


def command_version(command: str) -> str | None:
    executable = shutil.which(command)
    if executable is None:
        return None
    try:
        result = subprocess.run(
            [executable, "--version"],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    output = (result.stdout or result.stderr).strip().splitlines()
    return output[0] if output else None


def git_output(*arguments: str) -> str | None:
    try:
        result = subprocess.run(
            ["git", "-C", str(ROOT), *arguments],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return result.stdout.strip()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def file_record(path: Path) -> dict[str, object]:
    relative = path.relative_to(ROOT).as_posix()
    if not path.is_file():
        return {"path": relative, "present": False}
    return {
        "bytes": path.stat().st_size,
        "path": relative,
        "present": True,
        "sha256": sha256(path),
    }


def collect(artifact_paths: list[str]) -> dict[str, object]:
    status = git_output("status", "--porcelain") or ""
    source_commit = git_output("rev-parse", "HEAD")
    return {
        "artifacts": [
            file_record((ROOT / artifact).resolve() if not Path(artifact).is_absolute() else Path(artifact))
            for artifact in artifact_paths
        ],
        "generatedAt": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "lockFiles": [file_record(ROOT / relative) for relative in PROVENANCE_FILES],
        "repository": {
            "sourceCommit": source_commit,
            "worktreeDirty": bool(status),
            "worktreeStatusEntries": len(status.splitlines()) if status else 0,
        },
        "schemaVersion": 1,
        "toolchain": {tool: command_version(tool) for tool in REQUIRED_TOOLS},
    }


def validate(metadata: dict[str, object]) -> list[str]:
    errors: list[str] = []
    source_commit = metadata.get("repository", {}).get("sourceCommit")
    if not isinstance(source_commit, str) or not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        errors.append("repository.sourceCommit is not a full hexadecimal Git object id")

    toolchain = metadata.get("toolchain")
    if not isinstance(toolchain, dict):
        errors.append("toolchain metadata is missing")
    else:
        for tool in REQUIRED_TOOLS:
            if not isinstance(toolchain.get(tool), str) or not toolchain[tool]:
                errors.append(f"toolchain.{tool} is missing")

    lock_files = metadata.get("lockFiles")
    if not isinstance(lock_files, list):
        errors.append("lockFiles metadata is missing")
    else:
        for entry in lock_files:
            if not isinstance(entry, dict) or not entry.get("present"):
                errors.append(f"provenance file is missing: {entry}")
            elif not re.fullmatch(r"[0-9a-f]{64}", str(entry.get("sha256", ""))):
                errors.append(f"provenance file has no SHA-256: {entry.get('path')}")

    if not isinstance(metadata.get("generatedAt"), str) or not metadata["generatedAt"].endswith("Z"):
        errors.append("generatedAt is missing or is not UTC")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--artifact",
        action="append",
        default=[],
        help="artifact path to hash; may be repeated",
    )
    parser.add_argument(
        "--output",
        default="target/reproducibility/current.json",
        help="JSON output path, relative to the repository by default",
    )
    parser.add_argument(
        "--validate",
        action="store_true",
        help="fail when required provenance fields are incomplete",
    )
    args = parser.parse_args()

    output = Path(args.output)
    if not output.is_absolute():
        output = ROOT / output
    output.parent.mkdir(parents=True, exist_ok=True)
    metadata = collect(args.artifact)
    output.write_text(f"{json.dumps(metadata, indent=2, sort_keys=True)}\n", encoding="utf-8")
    if args.validate:
        errors = validate(metadata)
        if errors:
            for error in errors:
                print(f"reproducibility metadata check failed: {error}", file=sys.stderr)
            return 1
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
