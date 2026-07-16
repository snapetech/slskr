#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ledger="$repo_root/docs/parity/slskdn-slsknet-runtime-parity.md"
since="2026-05-13T00:00:00-06:00"
status=0
slskdn_registry=""
runtime_registry=""
slskdn_delta_registry=""

check_committed_registry() {
  local section
  section="$(sed -n '/^### Frozen source-commit registry$/,/^## Upstream Delta Buckets$/p' "$ledger")"
  if [[ -z "$section" ]]; then
    printf 'upstream parity classification failed: missing frozen source-commit registry\n' >&2
    status=1
    return
  fi

  slskdn_registry="$(
    printf '%s\n' "$section" |
      sed -n '/^- `needs-proof` behavior default/,/^- `slskNet.Runtime` source registry:/p' |
      sed '$d' |
      rg -o '`[0-9a-f]{8,9}`' |
      tr -d '`'
  )"
  runtime_registry="$(
    printf '%s\n' "$section" |
      sed -n '/^- `slskNet.Runtime` source registry:/p' |
      rg -o '`[0-9a-f]{8,9}`' |
      tr -d '`'
  )"

  check_prefix_set \
    "slskdN" \
    217 \
    "b9c3d63a81021ba0241941d21a381a87a8b588af0e167379e78b1280fe57d59c" \
    "$slskdn_registry"
  check_prefix_set \
    "slskNet.Runtime" \
    6 \
    "0d6017964b7b9b22c98482dd0856b667c45be2dee325bcb85aee67f6d02f06df" \
    "$runtime_registry"
}

check_current_delta_registry() {
  local section
  section="$(sed -n '/^### Post-freeze current-head source-commit registry$/,/^## Upstream Delta Buckets$/p' "$ledger")"
  if [[ -z "$section" ]]; then
    printf 'upstream parity classification failed: missing current-head delta registry\n' >&2
    status=1
    return
  fi
  slskdn_delta_registry="$(printf '%s\n' "$section" | rg -o '`[0-9a-f]{8}`' | tr -d '`')"
  check_prefix_set \
    "slskdN current-head delta" \
    10 \
    "b68a78bfef5a542122ec8c8cabc767a1a7d24073966e2233cc20b1d0512a9ce0" \
    "$slskdn_delta_registry"
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

registry_contains_prefix() {
  local registry="$1"
  local prefix="$2"
  grep -Eq "^${prefix}([0-9a-f])?$" <<<"$registry"
}

check_repo() {
  local label="$1"
  local repo="$2"
  local snapshot="$3"
  local registry="$4"
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
      if registry_contains_prefix "$registry" "$prefix"; then
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

check_delta_repo() {
  local label="$1"
  local repo="$2"
  local base="$3"
  local snapshot="$4"
  local expected_total="$5"
  local expected_explicit="$6"
  local registry="$7"
  local total=0
  local explicit=0
  local path_classified=0

  if [[ ! -d "$repo/.git" ]]; then
    printf '%s current-head delta cross-check skipped: repository absent at %s\n' "$label" "$repo"
    return
  fi
  for commit in "$base" "$snapshot"; do
    if ! git -C "$repo" cat-file -e "${commit}^{commit}" 2>/dev/null; then
      printf 'upstream parity classification failed: missing %s delta commit %s\n' \
        "$label" "$commit" >&2
      status=1
      return
    fi
  done
  if ! git -C "$repo" merge-base --is-ancestor "$base" "$snapshot"; then
    printf 'upstream parity classification failed: %s delta base is not an ancestor of snapshot\n' \
      "$label" >&2
    status=1
    return
  fi
  if ! git -C "$repo" merge-base --is-ancestor "$snapshot" HEAD; then
    printf 'upstream parity classification failed: %s delta snapshot is not an ancestor of HEAD\n' \
      "$label" >&2
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
      if registry_contains_prefix "$registry" "$prefix"; then
        explicit=$((explicit + 1))
      else
        printf 'upstream parity classification failed: unclassified %s delta commit %s %s\n' \
          "$label" "$prefix" "$(git -C "$repo" show -s --format=%s "$commit")" >&2
        status=1
      fi
    else
      path_classified=$((path_classified + 1))
    fi
  done < <(git -C "$repo" rev-list --no-merges "$base..$snapshot")

  if [[ "$total" -ne "$expected_total" || "$explicit" -ne "$expected_explicit" ]]; then
    printf 'upstream parity classification failed: %s delta classified %d commits (%d explicit), expected %d (%d explicit)\n' \
      "$label" "$total" "$explicit" "$expected_total" "$expected_explicit" >&2
    status=1
  fi
  printf '%s current-head delta classification: %d commits (%d explicit, %d path-classified)\n' \
    "$label" "$total" "$explicit" "$path_classified"
}

check_committed_registry
check_current_delta_registry

check_repo \
  "slskdN" \
  "${SLSKR_SLSKDN_REPO:-$repo_root/../slskdn}" \
  "c2586f576d8443e0229bf53501989568e6cbd61e" \
  "$slskdn_registry"
check_repo \
  "slskNet.Runtime" \
  "${SLSKR_SLSKNET_RUNTIME_REPO:-$repo_root/../slskNet.Runtime}" \
  "af73ff3f84fda7ba890bb5aea3adf712e5400cf6" \
  "$runtime_registry"
check_delta_repo \
  "slskdN" \
  "${SLSKR_SLSKDN_REPO:-$repo_root/../slskdn}" \
  "c2586f576d8443e0229bf53501989568e6cbd61e" \
  "7527bfe9d5622b40e893d13766ce51aafacc1d38" \
  40 \
  10 \
  "$slskdn_delta_registry"

if (( status != 0 )); then
  exit "$status"
fi

printf 'upstream parity classification check passed\n'
