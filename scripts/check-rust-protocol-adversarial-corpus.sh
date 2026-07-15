#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

test_file="crates/slskr-protocol/tests/adversarial.rs"

for anchor in SEEDS KNOWN_CORPUS adversarial_known_corpus_does_not_panic adversarial_multi_seed_random_corpus_does_not_panic; do
  if ! rg -n --fixed-strings -- "$anchor" "$test_file" >/dev/null; then
    printf 'rust protocol adversarial corpus check failed: missing %s in %s\n' "$anchor" "$test_file" >&2
    exit 1
  fi
done

for anchor in MAX_DECOMPRESSED_SEARCH_RESPONSE_BYTES decompress_zlib_with_limit compressed_search_response_is_bounded_before_decode; do
  if ! rg -n --fixed-strings -- "$anchor" crates/slskr-protocol/src/peer.rs >/dev/null; then
    printf 'rust protocol adversarial corpus check failed: missing %s in compressed peer-search handling\n' "$anchor" >&2
    exit 1
  fi
done

cargo test -p slskr-protocol adversarial_

printf 'rust protocol adversarial corpus passed\n'
