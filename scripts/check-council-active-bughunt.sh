#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
report="${COUNCIL_OUT_DIR:-$repo_root/.council}/active-bughunt.md"
inventory="$repo_root/docs/dev/council-scan-inventory.md"
redteam_review="$repo_root/docs/dev/red-team-abuse-review.md"

if [[ ! -f "$report" ]]; then
  printf 'active bughunt check failed: report is missing: %s\n' "$report" >&2
  exit 1
fi

expected_date="$(date -u '+%Y-%m-%d')"
generated_at="$(sed -n 's/^# Generated: //p' "$report" | head -n 1)"
if [[ ! "$generated_at" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ || "${generated_at:0:10}" != "$expected_date" ]]; then
  printf 'active bughunt check failed: report is not fresh for %s\n' "$expected_date" >&2
  exit 1
fi

expected_digest="$(python3 "$repo_root/scripts/council-source-digest.py")"
report_digest="$(sed -n 's/^# Source digest: //p' "$report" | head -n 1)"
if [[ "$report_digest" != "$expected_digest" ]]; then
  printf 'active bughunt check failed: report source digest is stale\n' >&2
  printf '  expected: %s\n  found:    %s\n' "$expected_digest" "$report_digest" >&2
  exit 1
fi

python3 - "$report" "$inventory" "$redteam_review" <<'PY'
from pathlib import Path
import re
import sys

report_path = Path(sys.argv[1])
inventory_path = Path(sys.argv[2])
redteam_review_path = Path(sys.argv[3])
expected_sections = (
    "Protocol-controlled allocations and lengths",
    "Proxy, redirect, SSRF, and outbound trust boundaries",
    "Filesystem and persistent-state boundaries",
    "Async task and channel lifecycle boundaries",
    "Browser injection, token storage, and opener boundaries",
    "Go SDK transport, decoding, and filesystem boundaries",
    "Python SDK transport, decoding, and filesystem boundaries",
    "Suppressed CI and script failures",
)

sections = {}
current = None
for line in report_path.read_text(encoding="utf-8").splitlines():
    if line.startswith("## "):
        current = line[3:]
        if current in sections:
            raise SystemExit(f"active bughunt check failed: duplicate section {current}")
        sections[current] = 0
    elif current is not None and line and not line.startswith("#"):
        sections[current] += 1

if tuple(sections) != expected_sections:
    missing = sorted(set(expected_sections) - set(sections))
    unexpected = sorted(set(sections) - set(expected_sections))
    raise SystemExit(
        "active bughunt check failed: section list changed; "
        f"missing={missing}, unexpected={unexpected}"
    )

inventory_lines = inventory_path.read_text(encoding="utf-8").splitlines()
try:
    marker = inventory_lines.index("### Active bughunt section review")
except ValueError:
    raise SystemExit("active bughunt check failed: inventory review table is missing")

rows = {}
for line in inventory_lines[marker + 1 :]:
    if line.startswith("### "):
        break
    match = re.match(r"^\| `([^`]+)` \| ([0-9]+) \| ([^|]+) \|", line)
    if match:
        section, count, status = match.groups()
        rows[section] = (int(count), status.strip())

allowed_statuses = {"Open", "Guarded", "Existing guard", "Accepted", "Fixed", "False positive", "Out of scope"}
for section, count in sections.items():
    inventory_row = rows.get(section)
    if inventory_row is None:
        raise SystemExit(f"active bughunt check failed: inventory row missing for {section}")
    expected_count, status = inventory_row
    if expected_count != count:
        raise SystemExit(
            f"active bughunt check failed: {section} inventory count {expected_count} != fresh count {count}"
        )
    if status not in allowed_statuses:
        raise SystemExit(f"active bughunt check failed: invalid status {status!r} for {section}")
    print(f"PASS active bughunt inventory: {section} ({count}, {status})")

if re.search(r"^\| `Red-team abuse lens` \|", inventory_path.read_text(encoding="utf-8"), re.MULTILINE):
    raise SystemExit("active bughunt check failed: remove the unsupported aggregate row; it is not a generated section")

if not redteam_review_path.is_file():
    raise SystemExit("active bughunt check failed: classified red-team abuse review is missing")
redteam_text = redteam_review_path.read_text(encoding="utf-8")
table_header = "| Abuse class and hypothesis | Classification | Evidence reviewed | Confidence | Reopen when |"
if table_header not in redteam_text:
    raise SystemExit("active bughunt check failed: red-team classification table is missing")
review_rows = []
in_table = False
for line in redteam_text.splitlines():
    if line == table_header:
        in_table = True
        continue
    if in_table and line.startswith("## "):
        break
    if in_table and line.startswith("| ") and not line.startswith("| ---"):
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) == 5:
            review_rows.append(cells)
if len(review_rows) != 9:
    raise SystemExit(
        f"active bughunt check failed: expected 9 classified red-team abuse rows, found {len(review_rows)}"
    )
allowed_redteam_statuses = {"Existing Guard", "Fixed — BUG-097"}
for row in review_rows:
    if row[1] not in allowed_redteam_statuses:
        raise SystemExit(
            f"active bughunt check failed: red-team row is not classified: {row[0]} ({row[1]})"
        )
    if not row[2] or row[3] not in {"High", "Moderate", "Low", "Unknown"} or not row[4]:
        raise SystemExit(f"active bughunt check failed: red-team evidence is incomplete for {row[0]}")
print(f"PASS red-team abuse review: {len(review_rows)} classified threat paths")

for path in (
    inventory_path,
    inventory_path.parent / "bug-council-active-backlog.md",
    inventory_path.parent / "refactoring-efficiency-plan.md",
):
    text = path.read_text(encoding="utf-8")
    if re.search(r"(?<![0-9])1,?165(?![0-9])", text):
        raise SystemExit(f"active bughunt check failed: unsupported 1,165 aggregate remains in {path}")
PY

printf 'active bughunt report freshness and inventory check passed\n'
