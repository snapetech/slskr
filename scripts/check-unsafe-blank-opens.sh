#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python3 - <<'PY'
from pathlib import Path
import re
import sys

roots = [Path("web/src"), Path("dashboard/src")]
status = 0

for root in roots:
    for path in sorted(root.rglob("*")):
        if path.suffix not in {".js", ".jsx", ".ts", ".tsx"}:
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        for index, line in enumerate(lines):
            if 'target="_blank"' in line or "target='_blank'" in line:
                window = "\n".join(lines[max(0, index - 4): min(len(lines), index + 5)])
                if "rel=" not in window or "noopener" not in window or "noreferrer" not in window:
                    print(f"{path}:{index + 1}: _blank link missing rel=\"noopener noreferrer\"", file=sys.stderr)
                    status = 1
            if "window.open(" in line and "safeOpen.js" not in str(path):
                print(f"{path}:{index + 1}: use safeOpenBlank for _blank window.open calls", file=sys.stderr)
                status = 1

safe_open = Path("web/src/lib/safeOpen.js").read_text(encoding="utf-8")
if "noopener,noreferrer" not in safe_open or "opened.opener = null" not in safe_open:
    print("web/src/lib/safeOpen.js: safeOpenBlank must set noopener,noreferrer and clear opener", file=sys.stderr)
    status = 1

sys.exit(status)
PY

printf 'unsafe _blank open check passed\n'
