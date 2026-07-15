#!/usr/bin/env bash
#
# Bug Council active bughunt runner for slskR.
#
# This runner is a discovery queue, not a pass/fail gate. Keep it wired into
# run-bug-council-all-phases.sh so every council cycle refreshes suspicious
# shapes that sit outside the closed sweep registers.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

out_dir="${COUNCIL_OUT_DIR:-.council}"
mkdir -p "$out_dir"
report="$out_dir/active-bughunt.md"

write_section() {
  local title="$1"
  local pattern="$2"
  shift 2

  {
    printf '\n## %s\n' "$title"
    rg -n --with-filename --pcre2 --hidden \
      --glob '!.git/**' \
      --glob '!.council/**' \
      --glob '!target/**' \
      --glob '!**/node_modules/**' \
      --glob '!**/dist/**' \
      --glob '!**/build/**' \
      --glob '!**/tests/**' \
      --glob '!**/*.test.*' \
      --glob '!**/*.spec.*' \
      "$pattern" "$@" || true
  } >>"$report"
}

cat >"$report" <<'EOF'
# Active Council Bughunt Candidate Report

This report is not a pass/fail proof. It is a fresh queue of suspicious shapes
that sit outside, or at the edge of, the current closed sweep gates. A green
all-phases council run means registered gates passed; it does not mean these
candidate lines are bugs or that no bugs exist.

Classification rule: any accepted row must be ledgered, fixed with behavior
coverage, sibling-swept, and promoted into a durable gate before closure.
EOF

write_section \
  "Protocol-controlled allocations and lengths" \
  'read_u(16|32|64)_le\(\)\? as usize|Vec::with_capacity\(|vec!\[[^;]+;[^]]+\]|read_(bytes|chunk)\([^)]*\)' \
  crates

write_section \
  "Proxy, redirect, SSRF, and outbound trust boundaries" \
  'forwarded|x_forwarded_for|redirect\(|Client::builder\(|reqwest::Client|to_socket_addrs\(|\.resolve\(' \
  crates

write_section \
  "Filesystem and persistent-state boundaries" \
  'canonicalize\(|create_dir_all\(|OpenOptions::new\(|remove_(file|dir|dir_all)\(|rename\(|set_permissions\(' \
  crates

write_section \
  "Async task and channel lifecycle boundaries" \
  'tokio::spawn|spawn_blocking|mpsc::unbounded|broadcast::channel|timeout\(|interval\(' \
  crates

write_section \
  "Browser injection, token storage, and opener boundaries" \
  'innerHTML\s*=|dangerouslySetInnerHTML|document\.write\(|eval\(|new Function\(|localStorage|sessionStorage|window\.open|target=.{0,1}_blank' \
  web dashboard client-ts

write_section \
  "Suppressed CI and script failures" \
  'continue-on-error:|allow_failure:|\|\|[[:space:]]+true|set[[:space:]]+\+e' \
  .github .gitlab-ci.yml scripts

printf 'Active council bughunt candidates saved to %s.\n' "$report"
printf 'Verdict boundary: this report is a discovery queue, not proof of no bugs.\n'
