#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  printf 'certification phase validation failed: select one or more phases\n' >&2
  exit 2
fi

phase_list="$(printf '%s' "$1" | tr '[:lower:]' '[:upper:]' | tr -d '[:space:]')"
if [[ -z "$phase_list" || "$phase_list" == ,* || "$phase_list" == *, || "$phase_list" == *,,* ]]; then
  printf 'certification phase validation failed: select one or more phases\n' >&2
  exit 2
fi

IFS=',' read -r -a phases <<<"$phase_list"
for raw_phase in "${phases[@]}"; do
  case "$raw_phase" in
    A|B|C|D|E|G|H) ;;
    *)
      printf 'certification phase validation failed: unknown phase %s (valid phases: A,B,C,D,E,G,H)\n' \
        "$raw_phase" >&2
      exit 2
      ;;
  esac
done
