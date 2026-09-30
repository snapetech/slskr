#!/usr/bin/env python3
"""Write and optionally validate local reproducibility metadata."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlsplit


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

CONFIGURATION_FILES = ("Cargo.toml", "rust-toolchain.toml", ".cargo/config.toml")


def github_context(artifact_name: str | None) -> dict[str, object] | None:
    if os.environ.get("GITHUB_ACTIONS") != "true":
        return None
    server = os.environ.get("GITHUB_SERVER_URL", "https://github.com").rstrip("/")
    repository = os.environ.get("GITHUB_REPOSITORY", "")
    run_id = os.environ.get("GITHUB_RUN_ID", "")
    context = {
        "provider": "github-actions",
        "repository": repository,
        "runId": run_id,
        "runAttempt": os.environ.get("GITHUB_RUN_ATTEMPT", ""),
        "job": os.environ.get("GITHUB_JOB", ""),
        "sourceCommit": os.environ.get("GITHUB_SHA", ""),
        "serverUrl": server,
        "runUrl": f"{server}/{repository}/actions/runs/{run_id}",
    }
    if artifact_name is not None:
        context["artifactName"] = artifact_name
    return context


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


def collect(artifact_paths: list[str], artifact_name: str | None = None) -> dict[str, object]:
    status = git_output("status", "--porcelain")
    source_commit = git_output("rev-parse", "HEAD")
    return {
        "artifacts": [
            file_record((ROOT / artifact).resolve() if not Path(artifact).is_absolute() else Path(artifact))
            for artifact in artifact_paths
        ],
        "ci": github_context(artifact_name),
        "configurationFiles": [file_record(ROOT / relative) for relative in CONFIGURATION_FILES],
        "environment": {"system": platform.system(), "architecture": platform.machine()},
        "generatedAt": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "lockFiles": [file_record(ROOT / relative) for relative in PROVENANCE_FILES],
        "repository": {
            "sourceCommit": source_commit,
            "worktreeDirty": bool(status) if status is not None else None,
            "worktreeStatusEntries": len(status.splitlines()) if status is not None else None,
        },
        "schemaVersion": 2,
        "toolchain": {tool: command_version(tool) for tool in REQUIRED_TOOLS},
    }


def validate_file_records(records: object, label: str, required: tuple[str, ...] = ()) -> list[str]:
    if not isinstance(records, list):
        return [f"{label} metadata is missing"]
    errors: list[str] = []
    paths: set[str] = set()
    for entry in records:
        if not isinstance(entry, dict):
            errors.append(f"{label} contains an invalid file record")
            continue
        path = entry.get("path")
        if not isinstance(path, str) or not path or path in paths:
            errors.append(f"{label} contains an invalid or duplicate path")
        else:
            paths.add(path)
        if entry.get("present") is not True:
            errors.append(f"provenance file is missing: {path}")
        elif not isinstance(entry.get("sha256"), str) or not re.fullmatch(r"[0-9a-f]{64}", entry["sha256"]):
            errors.append(f"provenance file has no SHA-256: {path}")
        size = entry.get("bytes")
        if type(size) is not int or size < 0:
            errors.append(f"provenance file has an invalid size: {path}")
    for path in required:
        if path not in paths:
            errors.append(f"{label} is missing required path: {path}")
    return errors


def validate(metadata: dict[str, object]) -> list[str]:
    errors: list[str] = []
    repository = metadata.get("repository")
    source_commit = repository.get("sourceCommit") if isinstance(repository, dict) else None
    if not isinstance(source_commit, str) or not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        errors.append("repository.sourceCommit is not a full hexadecimal Git object id")
    if type(metadata.get("schemaVersion")) is not int or metadata["schemaVersion"] not in (1, 2):
        errors.append("schemaVersion is unsupported")
    if not isinstance(repository, dict) or type(repository.get("worktreeDirty")) is not bool:
        errors.append("repository.worktreeDirty is unknown or invalid")
    if not isinstance(repository, dict) or type(repository.get("worktreeStatusEntries")) is not int or repository["worktreeStatusEntries"] < 0:
        errors.append("repository.worktreeStatusEntries is unknown or invalid")
    toolchain = metadata.get("toolchain")
    if not isinstance(toolchain, dict):
        errors.append("toolchain metadata is missing")
    else:
        for tool in REQUIRED_TOOLS:
            if not isinstance(toolchain.get(tool), str) or not toolchain[tool].strip():
                errors.append(f"toolchain.{tool} is missing")
    errors.extend(validate_file_records(metadata.get("lockFiles"), "lockFiles", PROVENANCE_FILES))
    errors.extend(validate_file_records(metadata.get("artifacts"), "artifacts"))
    if metadata.get("schemaVersion") == 2:
        errors.extend(validate_file_records(metadata.get("configurationFiles"), "configurationFiles", CONFIGURATION_FILES))
        environment = metadata.get("environment")
        if not isinstance(environment, dict) or not all(isinstance(environment.get(key), str) and environment[key] for key in ("system", "architecture")):
            errors.append("environment system/architecture is missing")
    generated_at = metadata.get("generatedAt")
    try:
        if not isinstance(generated_at, str) or not generated_at.endswith("Z"):
            raise ValueError()
        datetime.fromisoformat(generated_at.replace("Z", "+00:00"))
    except ValueError:
        errors.append("generatedAt is missing or is not UTC")
    ci = metadata.get("ci")
    if ci is not None:
        if not isinstance(ci, dict):
            errors.append("ci metadata is invalid")
        else:
            if ci.get("provider") != "github-actions":
                errors.append("ci.provider is unsupported")
            ci_repository = ci.get("repository")
            if not isinstance(ci_repository, str) or not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", ci_repository):
                errors.append("ci.repository is invalid")
            for key in ("runId", "runAttempt"):
                if not re.fullmatch(r"[1-9][0-9]*", str(ci.get(key, ""))):
                    errors.append(f"ci.{key} is invalid")
            for key in ("job",) + (("artifactName",) if "artifactName" in ci else ()):
                if not isinstance(ci.get(key), str) or not re.fullmatch(r"[A-Za-z0-9_.-]+", ci[key]):
                    errors.append(f"ci.{key} is invalid")
            if ci.get("sourceCommit") != source_commit:
                errors.append("ci.sourceCommit differs from the checked-out source")
            server = ci.get("serverUrl")
            try:
                parsed = urlsplit(server) if isinstance(server, str) else None
            except ValueError:
                parsed = None
            if not parsed or parsed.scheme != "https" or not parsed.hostname or parsed.username or parsed.password or parsed.path or parsed.query or parsed.fragment:
                errors.append("ci.serverUrl is invalid")
            expected_url = f"{server}/{ci_repository}/actions/runs/{ci.get('runId')}"
            if ci.get("runUrl") != expected_url:
                errors.append("ci.runUrl is inconsistent")
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
    parser.add_argument("--artifact-name", help="name of the retained CI metadata artifact")
    args = parser.parse_args()

    output = Path(args.output)
    if not output.is_absolute():
        output = ROOT / output
    output.parent.mkdir(parents=True, exist_ok=True)
    metadata = collect(args.artifact, args.artifact_name)
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
