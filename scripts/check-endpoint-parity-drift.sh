#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

scripts/diff-webui-endpoints.sh
printf 'endpoint parity drift check passed\n'
