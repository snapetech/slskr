#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ledger="$repo_root/docs/parity/slskdn-slsknet-runtime-parity.md"
since="2026-05-13"
status=0

check_committed_registry() {
  local section slskdn_prefixes runtime_prefixes
  section="$(sed -n '/^### Frozen source-commit registry$/,/^## Upstream Delta Buckets$/p' "$ledger")"
  if [[ -z "$section" ]]; then
    printf 'upstream parity classification failed: missing frozen source-commit registry\n' >&2
    status=1
    return
  fi

  slskdn_prefixes="$(
    printf '%s\n' "$section" |
      sed -n '/^- `needs-proof` behavior default/,/^- `slskNet.Runtime` source registry:/p' |
      sed '$d' |
      rg -o '`[0-9a-f]{8,9}`' |
      tr -d '`'
  )"
  runtime_prefixes="$(
    printf '%s\n' "$section" |
      sed -n '/^- `slskNet.Runtime` source registry:/p' |
      rg -o '`[0-9a-f]{8,9}`' |
      tr -d '`'
  )"

  check_prefix_set \
    "slskdN" \
    164 \
    "6aaafb3ecd577eed22c195cdb508d77fec0de5d70cf8add46a588213bd1c00ed" \
    "$slskdn_prefixes"
  check_prefix_set \
    "slskNet.Runtime" \
    6 \
    "0d6017964b7b9b22c98482dd0856b667c45be2dee325bcb85aee67f6d02f06df" \
    "$runtime_prefixes"
}

check_prefix_set() {
  local label="$1"
  local expected="$2"
  local expected_digest="$3"
  local prefixes="$4"
  local total unique actual_digest
  total="$(printf '%s\n' "$prefixes" | sed '/^$/d' | wc -l)"
  unique="$(printf '%s\n' "$prefixes" | sed '/^$/d' | sort -u | wc -l)"
  actual_digest="$(
    printf '%s\n' "$prefixes" |
      sed '/^$/d' |
      sort |
      sha256sum |
      cut -d ' ' -f 1
  )"
  if [[ "$total" -ne "$expected" ]]; then
    printf 'upstream parity classification failed: %s registry has %d prefixes, expected %d\n' \
      "$label" "$total" "$expected" >&2
    status=1
  fi
  if [[ "$unique" -ne "$total" ]]; then
    printf 'upstream parity classification failed: %s registry contains duplicate prefixes\n' \
      "$label" >&2
    status=1
  fi
  if [[ "$actual_digest" != "$expected_digest" ]]; then
    printf 'upstream parity classification failed: %s registry identity digest changed\n' \
      "$label" >&2
    status=1
  fi
}

check_repo() {
  local label="$1"
  local repo="$2"
  local snapshot="$3"
  local total=0
  local explicit=0
  local path_classified=0

  if [[ ! -d "$repo/.git" ]]; then
    printf '%s live cross-check skipped: repository absent at %s\n' "$label" "$repo"
    return
  fi
  if ! git -C "$repo" cat-file -e "${snapshot}^{commit}" 2>/dev/null; then
    printf 'upstream parity classification failed: missing %s snapshot %s\n' "$label" "$snapshot" >&2
    status=1
    return
  fi
  if ! git -C "$repo" merge-base --is-ancestor "$snapshot" HEAD; then
    printf 'upstream parity classification failed: %s snapshot %s is not an ancestor of HEAD\n' "$label" "$snapshot" >&2
    status=1
  fi

  while IFS= read -r commit; do
    [[ -n "$commit" ]] || continue
    total=$((total + 1))
    local requires_explicit=0
    while IFS= read -r path; do
      [[ -n "$path" ]] || continue
      case "$path" in
        src/*|config/*)
          requires_explicit=1
          ;;
        docs/*|memory-bank/*|tests/*|test/*|examples/*|.github/*|packaging/*|Formula/*|scripts/*|tools/*|vendor/*|publish/*|bin/*|analyzers/*|.council/*|.cursor/*|.kilo/*|*.md|.gitlab-ci.yml|flake.nix|Dockerfile|coverage-baseline.json|task_validation_*|.gitignore|.dockerignore)
          ;;
        *)
          requires_explicit=1
          ;;
      esac
    done < <(git -C "$repo" diff-tree --no-commit-id --name-only -r "$commit")

    if (( requires_explicit )); then
      local prefix="${commit:0:8}"
      if rg -q --fixed-strings -- "$prefix" "$ledger"; then
        explicit=$((explicit + 1))
      else
        printf 'upstream parity classification failed: unclassified %s commit %s %s\n' \
          "$label" "$prefix" "$(git -C "$repo" show -s --format=%s "$commit")" >&2
        status=1
      fi
    else
      path_classified=$((path_classified + 1))
    fi
  done < <(git -C "$repo" rev-list --since="$since" --no-merges "$snapshot")

  printf '%s classification: %d commits (%d explicit, %d path-classified)\n' \
    "$label" "$total" "$explicit" "$path_classified"
}

check_committed_registry

check_repo \
  "slskdN" \
  "${SLSKR_SLSKDN_REPO:-$repo_root/../slskdn}" \
  "c5145e3a86b303408ec8da3fd1e32f5fe2525ff6"
check_repo \
  "slskNet.Runtime" \
  "${SLSKR_SLSKNET_RUNTIME_REPO:-$repo_root/../slskNet.Runtime}" \
  "af73ff3f84fda7ba890bb5aea3adf712e5400cf6"

if (( status != 0 )); then
  exit "$status"
fi

printf 'upstream parity classification check passed\n'
