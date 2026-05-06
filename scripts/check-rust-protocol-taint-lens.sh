#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

tmp_bad="$(mktemp)"
tmp_good="$(mktemp)"
tmp_prod="$(mktemp)"
trap 'rm -f "$tmp_bad" "$tmp_good" "$tmp_prod"' EXIT

scan_files() {
  local output="$1"
  shift

  perl -0ne '
    sub scan_function {
      my ($file, $fn) = @_;
      my @findings = ();
      while ($fn =~ /let\s+(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)[^=;]*=\s*[^;]*(?:read_[ui](?:16|32|64)_le|read_[ui](?:16|32|64)\s*::\s*<\s*LittleEndian\s*>)[^;]*(?:as\s+usize)?\s*;/g) {
        my $var = $1;
        my $quoted = quotemeta($var);
        my $sink =
          qr/(?:Vec::with_capacity\s*\(\s*$quoted\s*\)|String::with_capacity\s*\(\s*$quoted\s*\)|HashMap::with_capacity\s*\(\s*$quoted\s*\)|HashSet::with_capacity\s*\(\s*$quoted\s*\)|\.resize\s*\(\s*$quoted\s*,|vec!\s*\[[^\]]*;\s*$quoted\s*\]|for\s+[^{};]+?\s+in\s+0\s*\.\.\s*$quoted\b|read_bytes\s*\(\s*$quoted\s*\)|read_exact_len\s*\(\s*$quoted\s*\))/s;
        next unless $fn =~ $sink;

        my $validator =
          qr/(?:DEFAULT_MAX|MAX_|max_|maximum|checked_|saturating_|try_from|ensure!\s*\([^;]*$quoted|if\s+$quoted\s*[<>]=?\s*[A-Za-z_][A-Za-z0-9_]*MAX|if\s+$quoted\s*>\s*[A-Za-z_][A-Za-z0-9_]*)/s;
        next if $fn =~ $validator;

        my ($name) = $fn =~ /(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)/;
        $name ||= "<unknown>";
        push @findings, "$file:$name: protocol-derived `$var` flows into allocation/read/loop sink without an obvious bound";
      }
      return @findings;
    }

    my $file = $ARGV;
    my $content = $_;
    while ($content =~ /((?:pub\s+)?(?:async\s+)?fn\s+[A-Za-z_][A-Za-z0-9_]*[\s\S]*?)(?=\n(?:pub\s+)?(?:async\s+)?fn\s+[A-Za-z_][A-Za-z0-9_]*|\z)/g) {
      print join("\n", scan_function($file, $1)), "\n";
    }
  ' "$@" | sed '/^$/d' >"$output"
}

scan_files "$tmp_bad" docs/dev/council-calibration/rust-protocol-taint-bad.rs
if [[ ! -s "$tmp_bad" ]]; then
  printf 'rust protocol taint lens failed: calibration bad fixture did not fire\n' >&2
  exit 1
fi

scan_files "$tmp_good" docs/dev/council-calibration/rust-protocol-taint-good.rs
if [[ -s "$tmp_good" ]]; then
  printf 'rust protocol taint lens failed: calibration good fixture produced findings\n' >&2
  sed 's/^/  /' "$tmp_good" >&2
  exit 1
fi

mapfile -d '' files < <(
  find crates -type f -name '*.rs' \
    ! -path '*/target/*' \
    ! -path '*/tests/fixtures/*' \
    -print0 | sort -z
)

scan_files "$tmp_prod" "${files[@]}"
if [[ -s "$tmp_prod" ]]; then
  printf 'rust protocol taint lens found unadjudicated production candidates:\n' >&2
  sed 's/^/  /' "$tmp_prod" >&2
  printf '\nAdd each row to docs/dev/council-scan-inventory.md or fix it before closing the council cycle.\n' >&2
  exit 1
fi

printf 'rust protocol taint lens passed\n'
