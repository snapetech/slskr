#!/usr/bin/env bash
#
# Bug Council candidate scanner for slskR.
#
# Output is intentionally noisy: it is the durable discovery queue, not the
# pass/fail gate. The pass/fail gate is scripts/check-remediation-baseline.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

scan() {
  local title="$1"
  local pattern="$2"
  shift 2

  printf '\n## %s\n' "$title"
  rg -n --with-filename --pcre2 --hidden \
    --glob '!.git/**' \
    --glob '!.council/**' \
    --glob '!target/**' \
    --glob '!**/node_modules/**' \
    --glob '!**/dist/**' \
    --glob '!**/build/**' \
    "$pattern" "$@" || true
}

printf '# Council candidate scan\n'
printf '# Generated: %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"

# === Universal patterns ====================================================

scan "Security-sensitive material candidates" \
  'PRIVATE KEY|gh[pousr]_[A-Za-z0-9_]{36,}|xox[baprs]-[A-Za-z0-9-]{20,}|AKIA[0-9A-Z]{16}|(?i)(api[_-]?key|access[_-]?token|client[_-]?secret)' \
  .

# === slskR Rust attack and reliability surfaces ============================

scan "Protocol count/length and allocation candidates" \
  'read_u(16|32|64)_le\(\)\? as usize|Vec::with_capacity\(|vec!\[[^;]+;[^]]+\]|read_(bytes|chunk)\([^)]*\)' \
  crates

scan "Filesystem mutation and path-resolution candidates" \
  'canonicalize\(|create_dir_all\(|OpenOptions::new\(|remove_(file|dir|dir_all)\(|rename\(|set_permissions\(' \
  crates

scan "Outbound network and process-launch candidates" \
  'reqwest::Client|Client::builder\(|Command::new\(|\.resolve\(|to_socket_addrs\(' \
  crates

scan "Async lifecycle and unbounded-channel candidates" \
  'tokio::spawn|spawn_blocking|mpsc::unbounded|broadcast::channel|timeout\(|interval\(' \
  crates

scan "Production panic candidates" \
  '\.(unwrap|expect)\(' \
  crates/*/src

# === Browser and SDK attack surfaces =======================================

scan "DOM/HTML/code injection candidates" \
  'innerHTML\s*=|dangerouslySetInnerHTML|document\.write\(|eval\(|new Function\(' \
  web dashboard client-ts

scan "Auth, browser storage, and opener candidates" \
  'localStorage|sessionStorage|window\.open|target=.{0,1}_blank|Authorization|X-API-Key' \
  web dashboard client-ts

# === Gate bypass candidates =================================================

scan "Suppressed CI and script failures" \
  'continue-on-error:|allow_failure:|\|\|[[:space:]]+true|set[[:space:]]+\+e' \
  .github .gitlab-ci.yml scripts

printf '\n# End of candidate scan. Every hit must be ledgered as Fixed, Existing guard, False positive, or Out of scope before a council sweep is closed.\n'
