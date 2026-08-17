#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# slskr depends on unpublished internal workspace crates. Package the runtime
# crates together; running only the `Cargo package -p slskr` command asks Cargo to resolve those
# crates from crates.io and is not a valid release gate for this repository.
# slskr-web is a WASM migration target with git-only UI dependencies and is
# covered by the wasm build gate rather than the Cargo crate package gate.
scripts/with-build-guard.sh cargo package -p slskr-protocol -p slskr-client -p slskr --no-verify
scripts/verify-cargo-package-contents.sh --skip-package
