#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

for id in BUG-005 BUG-006 BUG-007; do
  if ! rg -n "^\| ${id} \|" "$ledger" >/dev/null; then
    printf 'webhook outbound policy check failed: %s is missing from council ledger\n' "$id" >&2
    status=1
  fi
done

if ! rg -n 'validate_webhook_url_for_registration\(&url\)' crates/slskr/src/main.rs >/dev/null; then
  printf 'webhook outbound policy check failed: registration routes must validate webhook URLs before saving\n' >&2
  status=1
fi

for token in 'is_private' 'is_loopback' 'is_link_local' 'is_multicast' '2001:db8' 'SLSKR_WEBHOOK_ALLOW_CIDRS' 'SLSKR_WEBHOOK_DENY_CIDRS' 'localhost' '169.254.169.254'; do
  if ! rg -n "$token" crates/slskr/src/webhooks.rs >/dev/null; then
    printf 'webhook outbound policy check failed: expected webhook URL policy/test token missing: %s\n' "$token" >&2
    status=1
  fi
done

if ! rg -n 'MAX_WEBHOOK_DELIVERY_TASKS|webhook_deliveries' crates/slskr/src/main.rs >/dev/null; then
  printf 'webhook outbound policy check failed: bounded manual delivery pool is missing\n' >&2
  status=1
fi

if ! rg -n 'Uuid::new_v4\(\)' crates/slskr/src/webhooks.rs >/dev/null; then
  printf 'webhook outbound policy check failed: webhook and event IDs must use random UUIDs\n' >&2
  status=1
fi

if ! rg -n 'deliveries: Arc<Semaphore>' crates/slskr/src/webhooks.rs >/dev/null ||
   ! rg -n 'try_acquire_owned\(\)' crates/slskr/src/webhooks.rs >/dev/null; then
  printf 'webhook outbound policy check failed: normal dispatch must use the shared bounded delivery pool\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'webhook outbound policy check passed\n'
