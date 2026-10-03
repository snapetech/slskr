#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 || ! -f "$1" || ! -r "$1" ]]; then
  printf 'cross-client result check failed: expected a readable result log\n' >&2
  exit 2
fi

if awk -F '\t' '
  NR == 1 {
    header_seen = 1
    if ($0 != "timestamp\tscope\tcheck\tstatus\tdetail") {
      print "cross-client result check failed: invalid result-log header" > "/dev/stderr"
      invalid = 1
    }
    next
  }
  NF != 5 {
    printf "cross-client result check failed: malformed row %d\n", NR > "/dev/stderr"
    invalid = 1
  }
  $4 == "fail" || $4 ~ /^fail\([0-9]+\)$/ {
    printf "required cross-client check failed: scope=%s check=%s detail=%s\n", $2, $3, $5 > "/dev/stderr"
    failed = 1
  }
  END {
    if (!header_seen) {
      print "cross-client result check failed: result log is empty" > "/dev/stderr"
      invalid = 1
    }
    if (invalid) exit 2
    if (failed) exit 1
    exit 0
  }
' "$1"; then
  exit 0
else
  status=$?
  exit "$status"
fi
