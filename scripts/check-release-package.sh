#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# slskr depends on unpublished internal workspace crates. Package the workspace
# together; `cargo package -p slskr` asks Cargo to resolve those crates from
# crates.io and is not a valid release gate for this repository.
cargo package --workspace --allow-dirty --no-verify
