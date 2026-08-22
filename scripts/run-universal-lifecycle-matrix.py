#!/usr/bin/env python3
"""Run and record the strict frozen-target lifecycle matrix.

The universal gate is intentionally stricter than the ordinary differential
ledger.  This runner invokes one externally supplied, real case runner for
each frozen profile and lifecycle scenario, serially, and records the result
and artifacts without converting a missing runner or stale binary into a
pass.

Case-runner contract::

    <case-runner> <target> <scenario> <case-directory>

The case runner must perform the lifecycle operation against the frozen
profile and slskR, compare the externally observable result, and write at
least one evidence file below ``case-directory`` before returning zero.
The matrix runner owns the result envelope and never manufactures a passing
case artifact on behalf of a no-op command.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import stat
import subprocess
import sys
import time
from pathlib import Path
from typing import Any


TARGETS = ("slskd", "slskdn")
SCENARIOS = (
    "restart",
    "corrupt-state",
    "cancel",
    "timeout",
    "retry",
    "resume",
    "concurrent-mutation",
    "upgrade",
    "rollback",
    "permissions",
    "uninstall",
)
EVIDENCE_MODE = "live"
MATRIX_ID = "failure-restart-lifecycle-matrix"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def source_files(source_root: Path) -> list[Path]:
    """Return the source files that can change the slskr daemon binary."""

    files: set[Path] = set()
    for relative in ("Cargo.toml", "Cargo.lock", ".cargo/config.toml"):
        candidate = source_root / relative
        if candidate.is_file():
            files.add(candidate)
    for relative in ("crates/slskr", "crates/slskr-client", "crates/slskr-web"):
        directory = source_root / relative
        if not directory.is_dir():
            continue
        for candidate in directory.rglob("*"):
            if candidate.is_file() and ".git" not in candidate.parts:
                files.add(candidate)
    return sorted(files)


def newest_source_mtime_ns(source_root: Path) -> int:
    files = source_files(source_root)
    if not files:
        raise ValueError(f"no slskr source files found below {source_root}")
    return max(path.stat().st_mtime_ns for path in files)


def validate_preflight(
    *,
    replacement_binary: Path,
    source_root: Path,
    slskd_root: Path | None,
    slskdn_root: Path | None,
    slskd_binary: Path | None,
    slskdn_binary: Path | None,
    case_runner: Path | None,
) -> list[str]:
    failures: list[str] = []
    if not source_root.is_dir():
        failures.append(f"replacement source root does not exist: {source_root}")
    else:
        try:
            newest = newest_source_mtime_ns(source_root)
        except (OSError, ValueError) as error:
            failures.append(str(error))
        else:
            if not replacement_binary.is_file():
                failures.append(f"replacement binary does not exist: {replacement_binary}")
            else:
                mode = replacement_binary.stat().st_mode
                if not mode & stat.S_IXUSR:
                    failures.append(f"replacement binary is not executable: {replacement_binary}")
                if replacement_binary.stat().st_mtime_ns < newest:
                    failures.append(
                        "replacement binary predates current slskr sources; refusing stale-binary evidence: "
                        f"{replacement_binary}"
                    )
    for label, root in (("slskd", slskd_root), ("slskdn", slskdn_root)):
        if root is not None and not root.is_dir():
            failures.append(f"{label} frozen source root does not exist: {root}")
    for label, binary in (("slskd", slskd_binary), ("slskdn", slskdn_binary)):
        if binary is not None:
            if not binary.is_file():
                failures.append(f"{label} frozen binary does not exist: {binary}")
            elif not binary.stat().st_mode & stat.S_IXUSR:
                failures.append(f"{label} frozen binary is not executable: {binary}")
    if case_runner is None:
        failures.append(
            "no lifecycle case runner supplied; pass --case-runner with a real differential runner"
        )
    elif not case_runner.is_file():
        failures.append(f"lifecycle case runner does not exist: {case_runner}")
    elif not case_runner.stat().st_mode & stat.S_IXUSR:
        failures.append(f"lifecycle case runner is not executable: {case_runner}")
    return failures


def case_pairs() -> list[tuple[str, str]]:
    return [(target, scenario) for target in TARGETS for scenario in SCENARIOS]


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def run_case(
    *,
    case_runner: Path,
    target: str,
    scenario: str,
    case_directory: Path,
    replacement_binary: Path,
    source_root: Path,
    slskd_root: Path | None,
    slskdn_root: Path | None,
    slskd_binary: Path | None,
    slskdn_binary: Path | None,
    timeout_seconds: float,
) -> dict[str, Any]:
    case_directory.mkdir(parents=True, exist_ok=False)
    stdout_path = case_directory / "runner.stdout"
    stderr_path = case_directory / "runner.stderr"
    command = [str(case_runner), target, scenario, str(case_directory)]
    environment = os.environ.copy()
    environment.update(
        {
            "SLSKR_LIFECYCLE_TARGET": target,
            "SLSKR_LIFECYCLE_SCENARIO": scenario,
            "SLSKR_LIFECYCLE_CASE_DIR": str(case_directory),
            "SLSKR_REPLACEMENT_BINARY": str(replacement_binary),
            "SLSKR_REPLACEMENT_SOURCE_ROOT": str(source_root),
        }
    )
    if slskd_root is not None:
        environment["SLSKR_FROZEN_SLSKD_ROOT"] = str(slskd_root)
    if slskdn_root is not None:
        environment["SLSKR_FROZEN_SLSKDN_ROOT"] = str(slskdn_root)
    if slskd_binary is not None:
        environment["SLSKR_FROZEN_SLSKD_BINARY"] = str(slskd_binary)
    if slskdn_binary is not None:
        environment["SLSKR_FROZEN_SLSKDN_BINARY"] = str(slskdn_binary)

    started = time.monotonic()
    status = "fail"
    detail = "case runner did not execute"
    return_code: int | None = None
    try:
        with stdout_path.open("w", encoding="utf-8") as stdout, stderr_path.open(
            "w", encoding="utf-8"
        ) as stderr:
            completed = subprocess.run(
                command,
                cwd=source_root,
                env=environment,
                stdout=stdout,
                stderr=stderr,
                check=False,
                timeout=timeout_seconds,
            )
        return_code = completed.returncode
        if return_code == 0:
            external_artifacts = [
                path
                for path in sorted(case_directory.rglob("*"))
                if path.is_file() and path not in {stdout_path, stderr_path}
            ]
            if external_artifacts:
                status = "pass"
                detail = "case runner exited successfully with independent evidence"
            else:
                detail = "case runner exited successfully without an evidence artifact"
        else:
            detail = f"case runner exited with status {return_code}"
    except subprocess.TimeoutExpired:
        detail = f"case runner exceeded timeout of {timeout_seconds:g} seconds"
    except OSError as error:
        detail = f"could not execute lifecycle case runner: {error}"

    observation_path = case_directory / "case-observation.json"
    observation = {
        "target": target,
        "scenario": scenario,
        "status": status,
        "detail": detail,
        "command": command,
        "returnCode": return_code,
        "durationSeconds": round(time.monotonic() - started, 3),
        "runnerStdout": str(stdout_path),
        "runnerStderr": str(stderr_path),
        "evidenceMode": EVIDENCE_MODE,
    }
    write_json(observation_path, observation)
    artifacts = [str(path) for path in sorted(case_directory.rglob("*")) if path.is_file()]
    return {
        "target": target,
        "scenario": scenario,
        "status": status,
        "detail": detail,
        "evidenceArtifacts": artifacts,
    }


def failed_case(
    *, target: str, scenario: str, case_directory: Path, detail: str
) -> dict[str, Any]:
    case_directory.mkdir(parents=True, exist_ok=False)
    observation_path = case_directory / "case-observation.json"
    write_json(
        observation_path,
        {
            "target": target,
            "scenario": scenario,
            "status": "fail",
            "detail": detail,
            "evidenceMode": EVIDENCE_MODE,
        },
    )
    return {
        "target": target,
        "scenario": scenario,
        "status": "fail",
        "detail": detail,
        "evidenceArtifacts": [str(observation_path)],
    }


def build_evidence(
    *,
    output: Path,
    case_root: Path,
    cases: list[dict[str, Any]],
    preflight_failures: list[str],
    replacement_binary: Path,
    source_root: Path,
    slskd_root: Path | None,
    slskdn_root: Path | None,
    slskd_binary: Path | None,
    slskdn_binary: Path | None,
    case_runner: Path | None,
) -> dict[str, Any]:
    return {
        "id": MATRIX_ID,
        "schemaVersion": 1,
        "evidenceMode": EVIDENCE_MODE,
        "generatedAt": dt.datetime.now(dt.timezone.utc).isoformat(),
        "status": "pass" if not preflight_failures and all(case["status"] == "pass" for case in cases) else "fail",
        "targets": list(TARGETS),
        "scenarios": list(SCENARIOS),
        "targetScenarios": {target: list(SCENARIOS) for target in TARGETS},
        "preflightFailures": preflight_failures,
        "executables": {
            "replacementBinary": str(replacement_binary),
            "replacementSourceRoot": str(source_root),
            "slskdRoot": str(slskd_root) if slskd_root else None,
            "slskdnRoot": str(slskdn_root) if slskdn_root else None,
            "slskdBinary": str(slskd_binary) if slskd_binary else None,
            "slskdnBinary": str(slskdn_binary) if slskdn_binary else None,
            "caseRunner": str(case_runner) if case_runner else None,
        },
        "caseRoot": str(case_root),
        "evidenceArtifacts": [str(output), *[artifact for case in cases for artifact in case["evidenceArtifacts"]]],
        "cases": cases,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--case-runner", type=Path)
    parser.add_argument(
        "--replacement-binary",
        type=Path,
        default=Path("target/debug/slskr"),
    )
    parser.add_argument(
        "--replacement-source-root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
    )
    parser.add_argument("--slskd-root", type=Path)
    parser.add_argument("--slskdn-root", type=Path)
    parser.add_argument("--slskd-binary", type=Path)
    parser.add_argument("--slskdn-binary", type=Path)
    parser.add_argument(
        "--case-timeout-seconds",
        type=float,
        default=900.0,
    )
    args = parser.parse_args()
    if not 1 <= args.case_timeout_seconds <= 3600:
        parser.error("--case-timeout-seconds must be between 1 and 3600")

    output = args.output.resolve()
    source_root = args.replacement_source_root.resolve()
    replacement_binary = args.replacement_binary.resolve()
    case_runner = args.case_runner.resolve() if args.case_runner else None
    slskd_root = args.slskd_root.resolve() if args.slskd_root else None
    slskdn_root = args.slskdn_root.resolve() if args.slskdn_root else None
    slskd_binary = args.slskd_binary.resolve() if args.slskd_binary else None
    slskdn_binary = args.slskdn_binary.resolve() if args.slskdn_binary else None
    case_root = output.parent / f"{output.stem}.cases"
    if output.exists():
        raise SystemExit(f"refusing to overwrite existing lifecycle evidence: {output}")
    if case_root.exists() and any(case_root.iterdir()):
        raise SystemExit(f"refusing to overwrite existing lifecycle case artifacts: {case_root}")
    case_root.mkdir(parents=True, exist_ok=True)

    preflight_failures = validate_preflight(
        replacement_binary=replacement_binary,
        source_root=source_root,
        slskd_root=slskd_root,
        slskdn_root=slskdn_root,
        slskd_binary=slskd_binary,
        slskdn_binary=slskdn_binary,
        case_runner=case_runner,
    )
    cases: list[dict[str, Any]] = []
    for target, scenario in case_pairs():
        directory = case_root / target / scenario
        if preflight_failures:
            cases.append(
                failed_case(
                    target=target,
                    scenario=scenario,
                    case_directory=directory,
                    detail="; ".join(preflight_failures),
                )
            )
        else:
            assert case_runner is not None
            cases.append(
                run_case(
                    case_runner=case_runner,
                    target=target,
                    scenario=scenario,
                    case_directory=directory,
                    replacement_binary=replacement_binary,
                    source_root=source_root,
                    slskd_root=slskd_root,
                    slskdn_root=slskdn_root,
                    slskd_binary=slskd_binary,
                    slskdn_binary=slskdn_binary,
                    timeout_seconds=args.case_timeout_seconds,
                )
            )

    evidence = build_evidence(
        output=output,
        case_root=case_root,
        cases=cases,
        preflight_failures=preflight_failures,
        replacement_binary=replacement_binary,
        source_root=source_root,
        slskd_root=slskd_root,
        slskdn_root=slskdn_root,
        slskd_binary=slskd_binary,
        slskdn_binary=slskdn_binary,
        case_runner=case_runner,
    )
    write_json(output, evidence)
    passed = sum(case["status"] == "pass" for case in cases)
    print(f"universal lifecycle matrix: {passed}/{len(cases)} cases pass; output={output}")
    return 0 if evidence["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
