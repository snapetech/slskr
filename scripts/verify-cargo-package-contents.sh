#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "${1:-}" != "--skip-package" ]]; then
  cargo package -p slskr-protocol -p slskr-client -p slskr --no-verify
fi

python3 - <<'PY'
import os
import pathlib
import shutil
import subprocess
import tarfile
import tempfile

root = pathlib.Path.cwd()
package_dir = root / "target" / "package"
packages = [
    ("slskr-protocol", root / "crates" / "slskr-protocol"),
    ("slskr-client", root / "crates" / "slskr-client"),
    ("slskr", root / "crates" / "slskr"),
]


def git_files(crate_dir):
    output = subprocess.check_output(
        ["git", "ls-files", "--", str(crate_dir.relative_to(root))],
        cwd=root,
        text=True,
    )
    return [pathlib.Path(line) for line in output.splitlines() if line]


def safe_member_name(name):
    path = pathlib.PurePosixPath(name)
    return bool(name) and not path.is_absolute() and ".." not in path.parts


with tempfile.TemporaryDirectory(prefix="slskr-package-verify-") as tmp:
    tmp_path = pathlib.Path(tmp)
    workspace_root = tmp_path / "workspace"
    workspace_crates = workspace_root / "crates"
    workspace_crates.mkdir(parents=True)

    for name, crate_dir in packages:
        archive = package_dir / f"{name}-0.0.0.crate"
        if not archive.exists():
            raise SystemExit(f"package verification failed: missing archive {archive}")

        expected_root = f"{name}-0.0.0"
        destination = workspace_crates / name
        destination.mkdir()

        with tarfile.open(archive, "r:gz") as tar:
            members = tar.getmembers()
            names = {member.name for member in members}
            for member in members:
                if not safe_member_name(member.name):
                    raise SystemExit(f"package verification failed: unsafe archive path {member.name!r}")
                parts = pathlib.PurePosixPath(member.name).parts
                if not parts or parts[0] != expected_root:
                    raise SystemExit(
                        f"package verification failed: {archive.name} contains unexpected root {member.name!r}"
                    )

            for required in [
                f"{expected_root}/Cargo.toml",
                f"{expected_root}/Cargo.toml.orig",
                f"{expected_root}/Cargo.lock",
                f"{expected_root}/.cargo_vcs_info.json",
            ]:
                if required not in names:
                    raise SystemExit(f"package verification failed: {archive.name} missing {required}")

            for source_file in git_files(crate_dir):
                relative = (root / source_file).relative_to(crate_dir)
                if relative == pathlib.Path("Cargo.toml"):
                    expected = f"{expected_root}/Cargo.toml.orig"
                else:
                    expected = f"{expected_root}/{relative.as_posix()}"
                if expected not in names:
                    raise SystemExit(
                        f"package verification failed: {archive.name} missing tracked file {relative}"
                    )

            for member in members:
                if not member.isfile():
                    continue
                relative = pathlib.PurePosixPath(member.name).relative_to(expected_root)
                target = destination / pathlib.Path(*relative.parts)
                target.parent.mkdir(parents=True, exist_ok=True)
                extracted = tar.extractfile(member)
                if extracted is None:
                    raise SystemExit(f"package verification failed: cannot read {member.name}")
                target.write_bytes(extracted.read())

        shutil.copyfile(destination / "Cargo.toml.orig", destination / "Cargo.toml")

    (workspace_root / "Cargo.toml").write_text(
        """[workspace]
members = [
    "crates/slskr",
    "crates/slskr-client",
    "crates/slskr-protocol",
]
resolver = "2"

[workspace.package]
edition = "2021"
license = "AGPL-3.0-only"
homepage = "https://github.com/snapetech/slskr"
repository = "https://github.com/snapetech/slskr"
rust-version = "1.76"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = "warn"
""",
        encoding="utf-8",
    )

    env = os.environ.copy()
    env.setdefault("SLSKR_BUILD_WEB", "")
    env.pop("SLSKR_BUILD_WEB", None)
    subprocess.check_call(
        ["cargo", "check", "--workspace", "--manifest-path", str(workspace_root / "Cargo.toml")],
        cwd=workspace_root,
        env=env,
    )

print("cargo package content verification passed")
PY
