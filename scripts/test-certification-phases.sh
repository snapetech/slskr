#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
validator="$repo_root/scripts/validate-certification-phases.sh"

"$validator" 'a, B, h'

for invalid in '' 'A,Q' 'A,,B' 'F'; do
  if "$validator" "$invalid" >/dev/null 2>&1; then
    printf 'certification phase regression failed: accepted invalid phase list %q\n' "$invalid" >&2
    exit 1
  else
    status=$?
    if [[ "$status" -ne 2 ]]; then
      printf 'certification phase regression failed: invalid phase list returned %s, expected 2\n' "$status" >&2
      exit 1
    fi
  fi
done

if ! grep -Fq 'run_vpn_cargo_retry 2 5 "cert-a5"' "$repo_root/scripts/run-certification.sh"; then
  printf 'certification phase regression failed: indirect peer probe has no bounded retry\n' >&2
  exit 1
fi

test_root="$(mktemp -d)"
trap 'python3 -c "import shutil, sys; shutil.rmtree(sys.argv[1])" "$test_root"' EXIT
mkdir -p "$test_root/empty" "$test_root/library-output"
: >"$test_root/empty/credentials.env"
SLSKR_CERTIFICATION_LIBRARY_ONLY=1
SLSKR_CERTIFY_ENV_FILE="$test_root/empty/credentials.env"
SLSKR_PROTON_CREDENTIAL_POOL_FILE="$test_root/empty/credentials.env"
SLSKR_CERTIFY_OUTPUT_DIR="$test_root/library-output"
SLSKR_CERTIFY_VPN_ENABLED=0
# shellcheck disable=SC1090
source "$repo_root/scripts/run-certification.sh"

regular_endpoint="$(peer_address_metadata 'peer address attempt=1 ip=192.0.2.1 port=2234 obfuscation_type=0 obfuscated_port=0')"
obfuscated_endpoint="$(peer_address_metadata 'peer address attempt=1 port=0 obfuscation_type=1 obfuscated_port=2235')"
unreachable_endpoint="$(peer_address_metadata 'peer address attempt=1 port=0 obfuscation_type=0 obfuscated_port=0')"
if ! peer_address_has_usable_endpoint "$regular_endpoint" \
  || ! peer_address_has_usable_endpoint "$obfuscated_endpoint" \
  || peer_address_has_usable_endpoint "$unreachable_endpoint" \
  || peer_address_has_usable_endpoint 'malformed'; then
  printf 'certification phase regression failed: peer endpoint validity classification is incorrect\n' >&2
  exit 1
fi

mkdir -p "$test_root/bin" "$test_root/commons" "$test_root/output-ready"
printf 'certification fixture\n' >"$test_root/commons/commons-click-track.ogg"
fixture_size="$(wc -c <"$test_root/commons/commons-click-track.ogg" | tr -d ' ')"
fixture_sha="$(sha256sum "$test_root/commons/commons-click-track.ogg" | awk '{print $1}')"
printf '# id\tfilename\ttype\tsize\tsha\tlicense\tlicense_url\tsource_url\tdownload_url\tattribution\n' \
  >"$test_root/manifest.tsv"
printf 'test-fixture\tcommons-click-track.ogg\taudio/ogg\t%s\t%s\tCC0\thttps://example.invalid/license\thttps://example.invalid/source\thttps://example.invalid/download\tfixture author\n' \
  "$fixture_size" "$fixture_sha" >>"$test_root/manifest.tsv"
printf 'SLSKR_TEST_1_USERNAME=certification-test\nSLSKR_TEST_1_PASSWORD=placeholder\n' \
  >"$test_root/certification.env"
: >"$test_root/credential-pool.env"
cat >"$test_root/bin/cargo" <<'MOCK_CARGO'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\t%s\n' "${SLSKR_FIXTURE_PEER_FILE:-}" "$*" >>"$CERTIFICATION_MOCK_CARGO_LOG"
if [[ "$*" == *"smoke fixture-peer"* ]]; then
  printf 'fixture peer smoke completed; bytes=%s; sha256=%s\n' \
    "$MOCK_FIXTURE_SIZE" "$MOCK_FIXTURE_SHA"
else
  printf 'mock certification command completed\n'
fi
MOCK_CARGO
cat >"$test_root/bin/curl" <<'MOCK_CURL'
#!/usr/bin/env bash
printf 'fixture download intentionally failed in certification regression\n' >&2
exit 22
MOCK_CURL
chmod +x "$test_root/bin/cargo" "$test_root/bin/curl"

run_fixture_certification() {
  local case_name="$1"
  local command_log="$test_root/$case_name-cargo.log"
  local output_dir="$test_root/output-$case_name"
  local runner_output="$test_root/$case_name-output.log"
  mkdir -p "$output_dir"
  set +e
  PATH="$test_root/bin:$PATH" \
    SLSKR_CERTIFY_ENV_FILE="$test_root/certification.env" \
    SLSKR_PROTON_CREDENTIAL_POOL_FILE="$test_root/credential-pool.env" \
    SLSKR_CERTIFY_VPN_ENABLED=0 \
    SLSKR_CERTIFY_OUTPUT_DIR="$output_dir" \
    SLSKR_COMMONS_FIXTURE_DIR="$test_root/commons" \
    SLSKR_COMMONS_FIXTURE_MANIFEST="$test_root/manifest.tsv" \
    CERTIFICATION_MOCK_CARGO_LOG="$command_log" \
    MOCK_FIXTURE_SIZE="$fixture_size" \
    MOCK_FIXTURE_SHA="$fixture_sha" \
    "$repo_root/scripts/run-certification.sh" --phases B --log-format json \
    >"$runner_output" 2>&1
  local status=$?
  set -e
  printf '%s\n' "$status" >"$test_root/$case_name-status"
}

run_fixture_certification ready
python3 - "$test_root" "$fixture_size" "$fixture_sha" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
size, sha = sys.argv[2:]
report = next((root / "output-ready").glob("summary-*.json"))
summary = json.loads(report.read_text())
assert summary["failed"] == 0, summary
rows = [
    json.loads(line)
    for line in (root / "ready-output.log").read_text().splitlines()
    if line.startswith('{"phase":')
]
assert {row["id"]: row["status"] for row in rows} == {
    "B1": "pass", "B2": "pass", "B3": "pass", "B4": "pass", "B5": "pass"
}, rows
expected_path = str(root / "commons" / "commons-click-track.ogg")
fixture_calls = [
    line for line in (root / "ready-cargo.log").read_text().splitlines()
    if "smoke fixture-peer" in line
]
assert len(fixture_calls) == 3, fixture_calls
assert all(line.startswith(expected_path + "\t") for line in fixture_calls), fixture_calls
assert any(f"bytes={size}" in line and f"sha256={sha}" in line for line in (root / "ready-output.log").read_text().splitlines())
PY

printf 'corrupt fixture\n' >"$test_root/commons/commons-click-track.ogg"
run_fixture_certification missing
if [[ "$(cat "$test_root/missing-status")" -ne 1 ]]; then
  printf 'certification fixture regression failed: missing-fixture run did not report test failures\n' >&2
  exit 1
fi
python3 - "$test_root" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
report = next((root / "output-missing").glob("summary-*.json"))
summary = json.loads(report.read_text())
assert summary["failed"] == 3, summary
rows = [
    json.loads(line)
    for line in (root / "missing-output.log").read_text().splitlines()
    if line.startswith('{"phase":')
]
by_id = {row["id"]: row for row in rows}
for test_id in ("B1", "B2", "B3"):
    assert by_id[test_id]["status"] == "fail", by_id[test_id]
    assert "fixture preparation failed" in by_id[test_id]["detail"], by_id[test_id]
assert by_id["B4"]["status"] == "pass" and by_id["B5"]["status"] == "pass", rows
assert not any("smoke fixture-peer" in line for line in (root / "missing-cargo.log").read_text().splitlines())
PY

printf 'certification phase and fixture preparation regressions passed\n'
