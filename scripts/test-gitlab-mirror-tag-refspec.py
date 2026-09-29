#!/usr/bin/env python3
"""Exercise the GitLab tag-mirror helper against an isolated bare repository."""

import os
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
MIRROR_HELPER = ROOT / "scripts/gitlab-mirror-ref.sh"
TAG = "release-v0.0.0-rf-mirror-probe"


def git(*arguments: str, cwd: Path | None = None) -> str:
    result = subprocess.run(
        ["git", *arguments],
        check=True,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return result.stdout.strip()


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="slskr-gitlab-mirror-") as temp:
        root = Path(temp)
        source = root / "source"
        bare_remote = root / "mirror.git"
        git("init", "--bare", str(bare_remote))
        git("init", "--initial-branch=main", str(source))
        git("config", "user.name", "slskR GitLab mirror test", cwd=source)
        git("config", "user.email", "slskr-gitlab-mirror-test@example.invalid", cwd=source)
        (source / "probe.txt").write_text("synthetic tag mirror probe\n")
        git("add", "probe.txt", cwd=source)
        git("commit", "-m", "test synthetic tag mirror", cwd=source)
        git("tag", "-a", TAG, "-m", "synthetic tag refspec test", cwd=source)

        environment = os.environ.copy()
        environment.update(
            {
                "CI_COMMIT_REF_NAME": "main",
                "CI_COMMIT_TAG": TAG,
                "GITHUB_MIRROR_URL": str(bare_remote),
            }
        )
        subprocess.run(
            ["sh", str(MIRROR_HELPER)],
            check=True,
            cwd=source,
            env=environment,
        )

        source_tag = git("rev-parse", f"refs/tags/{TAG}", cwd=source)
        mirrored_tag = git(
            "--git-dir", str(bare_remote), "rev-parse", f"refs/tags/{TAG}"
        )
        if source_tag != mirrored_tag:
            raise SystemExit(
                f"tag mirror changed the ref object: source={source_tag}, mirror={mirrored_tag}"
            )

    print("GitLab tag-mirror refspec passed against an isolated bare repository.")


if __name__ == "__main__":
    main()
