#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

out_dir="${COUNCIL_OUT_DIR:-.council}"
mkdir -p "$out_dir"
scan_out="$out_dir/latest-candidate-counts.md"

printf '==> Fresh candidate inventory\n'
scripts/run-council-scan.sh | tee "$scan_out"

printf '\n==> Process gates\n'
scripts/check-council-active-backlog.sh
scripts/check-council-inventory-closure.sh
scripts/check-council-loop.sh
scripts/check-council-negative-space.sh

printf '\n==> Semantic/calibration lenses\n'
scripts/check-rust-protocol-taint-lens.sh

printf '\n==> Adversarial protocol corpus\n'
scripts/check-rust-protocol-adversarial-corpus.sh

printf '\n==> Pending bughunt phases\n'
if rg -n '^\| [0-9]+ \| .* \| Pending \|' docs/dev/bug-council-phases.md; then
  printf '\nCouncil bughunt is not complete: pending phases remain. Pick the first pending row above and burn it down.\n' >&2
  exit 2
fi

printf '\nAll slskR bug council phases passed. Candidate counts saved to %s.\n' "$scan_out"
printf 'Council verdict boundary: this is not proof of no bugs. It means the current calibrated lenses, active backlog, process gates, and adversarial corpus passed.\n'
