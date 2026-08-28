#!/usr/bin/env python3
"""Bounded schema and execution test for the universal lifecycle runner."""

from __future__ import annotations

import json
import os
import stat
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
RUNNER = ROOT / "scripts" / "run-universal-lifecycle-matrix.py"


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="slskr-lifecycle-test-") as directory:
        root = Path(directory)
        source = root / "source"
        source.mkdir()
        (source / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
        (source / "Cargo.lock").write_text("", encoding="utf-8")
        (source / ".cargo").mkdir()
        (source / ".cargo" / "config.toml").write_text("", encoding="utf-8")
        for relative in ("crates/slskr/src", "crates/slskr-client/src", "crates/slskr-web/src"):
            (source / relative).mkdir(parents=True)
        (source / "crates/slskr/src" / "lib.rs").write_text(
            "pub fn run() {}\n", encoding="utf-8"
        )
        (source / "crates/slskr/src" / "main.rs").write_text(
            "fn main() {}\n", encoding="utf-8"
        )
        binary = root / "slskr"
        binary.write_bytes(b"test binary\n")
        binary.chmod(binary.stat().st_mode | stat.S_IXUSR)
        newest = max(path.stat().st_mtime_ns for path in source.rglob("*"))
        os.utime(binary, ns=(newest + 1, newest + 1))

        case_runner = root / "case-runner"
        case_runner.write_text(
            "#!/usr/bin/env python3\n"
            "import json, pathlib, sys\n"
            "target, scenario, directory = sys.argv[1:4]\n"
            "pathlib.Path(directory, 'independent-observation.json').write_text("
            "json.dumps({'target': target, 'scenario': scenario, 'status': 'pass'}) + '\\n')\n",
            encoding="utf-8",
        )
        case_runner.chmod(case_runner.stat().st_mode | stat.S_IXUSR)
        output = root / "lifecycle.json"
        command = [
            sys.executable,
            str(RUNNER),
            "--output",
            str(output),
            "--case-runner",
            str(case_runner),
            "--replacement-binary",
            str(binary),
            "--replacement-source-root",
            str(source),
            "--case-timeout-seconds",
            "10",
        ]
        completed = subprocess.run(command, cwd=ROOT, check=False, capture_output=True, text=True)
        if completed.returncode != 0:
            raise AssertionError(completed.stdout + completed.stderr)
        evidence = json.loads(output.read_text(encoding="utf-8"))
        assert evidence["status"] == "pass"
        assert len(evidence["cases"]) == 22
        assert all(case["status"] == "pass" for case in evidence["cases"])
        assert all(Path(path).is_file() for case in evidence["cases"] for path in case["evidenceArtifacts"])
        assert Path(evidence["evidenceArtifacts"][0]).is_file()

        stale = root / "stale"
        stale.write_bytes(b"stale\n")
        stale.chmod(stale.stat().st_mode | stat.S_IXUSR)
        os.utime(stale, ns=(1, 1))
        failed_output = root / "stale-lifecycle.json"
        failed = subprocess.run(
            [
                sys.executable,
                str(RUNNER),
                "--output",
                str(failed_output),
                "--case-runner",
                str(case_runner),
                "--replacement-binary",
                str(stale),
                "--replacement-source-root",
                str(source),
            ],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
        assert failed.returncode == 1
        stale_evidence = json.loads(failed_output.read_text(encoding="utf-8"))
        assert stale_evidence["status"] == "fail"
        assert all(case["status"] == "fail" for case in stale_evidence["cases"])
        assert any("stale-binary" in failure for failure in stale_evidence["preflightFailures"])

    print("universal lifecycle matrix test passed")


if __name__ == "__main__":
    main()
