#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

if ! rg -n '^\| BUG-016 .* \| Verified \|$' "$ledger" >/dev/null; then
  printf 'secret scanning check failed: BUG-016 must stay verified in council ledger\n' >&2
  status=1
fi

for ignored in .env web/.env.local .secrets; do
  if ! git check-ignore -q "$ignored"; then
    printf 'secret scanning check failed: expected ignored local secret path is not ignored: %s\n' "$ignored" >&2
    status=1
  fi
done

python3 - <<'PY'
import math
import pathlib
import re
import subprocess
import sys

root = pathlib.Path.cwd()
tracked = subprocess.check_output(["git", "ls-files"], cwd=root, text=True).splitlines()

skip_suffixes = {
    ".png", ".jpg", ".jpeg", ".gif", ".webp", ".woff", ".woff2", ".ttf", ".eot",
    ".zip", ".gz", ".crate", ".lock",
}
config_like_suffixes = {
    ".env", ".json", ".toml", ".yaml", ".yml", ".ini", ".conf", ".config", ".properties",
    ".md", ".txt", ".sh",
}
placeholder_terms = {
    "placeholder", "example", "optional", "changeme", "change-me", "your-", "test-token",
    "secret_generated_value", "spotify-app-client-secret", "live-", "dummy", "redacted",
    "fixture", "differential",
}
secret_key = re.compile(r'(?i)\b(api[_-]?key|token|secret|password|passwd|private[_-]?key|client[_-]?secret)\b')
assignment = re.compile(r'''(?ix)
    (?P<key>[A-Z0-9_.-]*(?:api[_-]?key|token|secret|password|passwd|private[_-]?key|client[_-]?secret)[A-Z0-9_.-]*)
    \s*[:=]\s*
    (?P<quote>["']?)(?P<value>[^"'\s#,\]}]+)
''')
private_key = re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----")

def entropy(value: str) -> float:
    if not value:
        return 0.0
    total = len(value)
    return -sum((value.count(ch) / total) * math.log2(value.count(ch) / total) for ch in set(value))

def allowed(path: str, key: str, value: str) -> bool:
    haystack = f"{path} {key} {value}".lower()
    if any(term in haystack for term in placeholder_terms):
        return True
    if "storagekey" in key.lower():
        return True
    if value.startswith("$(") or value.startswith("${") or value.startswith("$"):
        return True
    if path.endswith("k8s/secrets.example.yaml"):
        return True
    if pathlib.Path(path).suffix.lower() == ".md" and len(value) < 64:
        return True
    return False

findings = []
for rel in tracked:
    path = root / rel
    if path.suffix.lower() in skip_suffixes or not path.is_file():
        continue
    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        continue
    if private_key.search(text) and "example" not in rel.lower():
        findings.append(f"{rel}: private key block")
    for line_no, line in enumerate(text.splitlines(), 1):
        if not secret_key.search(line):
            continue
        for match in assignment.finditer(line):
            key = match.group("key")
            value = match.group("value").strip()
            quote = match.group("quote")
            if not quote and path.suffix.lower() not in config_like_suffixes:
                continue
            if len(value) < 16 or allowed(rel, key, value):
                continue
            if entropy(value) >= 3.5 or re.search(r'[A-Za-z]', value) and re.search(r'[0-9]', value):
                findings.append(f"{rel}:{line_no}: possible committed secret in {key}")

if findings:
    print("secret scanning check failed:", file=sys.stderr)
    for finding in findings:
        print(f"  {finding}", file=sys.stderr)
    raise SystemExit(1)
PY

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'secret scanning check passed\n'
