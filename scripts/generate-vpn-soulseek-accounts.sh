#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pool_file="${SLSKR_PROTON_CREDENTIAL_POOL_FILE:-$repo_root/.secrets/proton-credential-pool.env}"
base_env_file="${SLSKR_LIVE_ENV_FILE:-$repo_root/.env}"
output_file="${SLSKR_GENERATED_ACCOUNTS_FILE:-$repo_root/.secrets/generated-soulseek-accounts.env}"
labels=(${SLSKR_ACCOUNT_GENERATOR_LABELS:-p5 p6 p7 p8})
start_index="${SLSKR_ACCOUNT_GENERATOR_START_INDEX:-5}"
prefix="${SLSKR_ACCOUNT_GENERATOR_PREFIX:-sr$(date +%m%d)}"
server_address="${SLSK_SERVER:-}"

if [[ ! -f "$pool_file" ]]; then
  echo "missing Proton pool file: $pool_file" >&2
  exit 1
fi

if [[ -f "$base_env_file" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$base_env_file"
  set +a
fi

# shellcheck disable=SC1090
source "$pool_file"

if [[ -z "$server_address" ]]; then
  server_ip="$(getent ahostsv4 server.slsknet.org | awk 'NR == 1 { print $1 }')"
  if [[ -z "$server_ip" ]]; then
    echo "failed to resolve server.slsknet.org on host" >&2
    exit 1
  fi
  server_address="$server_ip:2242"
fi

resolve_config() {
  local label="$1"
  local var_name="SLSKR_PROTON_CONFIG_${label}"
  local path="${!var_name:-}"
  if [[ -z "$path" ]]; then
    echo "unknown Proton config label: $label" >&2
    exit 2
  fi
  if [[ "$path" != /* ]]; then
    path="$repo_root/$path"
  fi
  if [[ ! -f "$path" ]]; then
    echo "missing Proton config for $label" >&2
    exit 1
  fi
  printf '%s' "$path"
}

quote_env() {
  printf '%q' "$1"
}

mkdir -p "$(dirname "$output_file")"
chmod 700 "$(dirname "$output_file")"
tmp="$(mktemp)"
chmod 600 "$tmp"

if [[ -f "$output_file" ]]; then
  grep -v -E '^(SLSKR_TEST_ACCOUNT_COUNT|SLSKR_TEST_[0-9]+_(USERNAME|PASSWORD))=' "$output_file" > "$tmp" || true
fi

index="$start_index"
created=0
for label in "${labels[@]}"; do
  config="$(resolve_config "$label")"
  username="${prefix}${label}$(printf '%04d' "$((RANDOM % 10000))")"
  password="$(openssl rand -base64 24 | tr -d '\n=+/ ' | cut -c1-24)"
  namespace="acct${label}"
  stdout_file="$(mktemp)"
  stderr_file="$(mktemp)"

  set +e
  timeout "${SLSKR_ACCOUNT_GENERATOR_TIMEOUT_SECONDS:-90}" \
    "$repo_root/scripts/run-in-proton-wg-netns.sh" "$namespace" "$config" \
    env SLSK_USERNAME="$username" SLSK_PASSWORD="$password" SLSK_SERVER="$server_address" \
    cargo run -q -p slskr -- login smoke >"$stdout_file" 2>"$stderr_file"
  status=$?
  set -e

  if [[ "$status" -eq 0 ]]; then
    {
      printf 'SLSKR_TEST_%s_USERNAME=%s\n' "$index" "$(quote_env "$username")"
      printf 'SLSKR_TEST_%s_PASSWORD=%s\n' "$index" "$(quote_env "$password")"
    } >> "$tmp"
    echo "created account index=$index via label=$label"
    index=$((index + 1))
    created=$((created + 1))
  else
    detail="$( { cat "$stdout_file"; tail -n 12 "$stderr_file"; } | tr '\n\t' '  ' | sed -E 's/[[:space:]]+/ /g; s/password=[^ ]+/password=<redacted>/Ig; s/SLSK_PASSWORD=[^ ]+/SLSK_PASSWORD=<redacted>/g' )"
    echo "failed account via label=$label: $detail" >&2
  fi

  rm -f "$stdout_file" "$stderr_file"
  sleep "${SLSKR_ACCOUNT_GENERATOR_COOLDOWN_SECONDS:-10}"
done

count=$((index - 1))
{
  printf 'SLSKR_TEST_ACCOUNT_COUNT=%s\n' "$count"
  cat "$tmp"
} > "$output_file"
chmod 600 "$output_file"
rm -f "$tmp"

echo "generated $created account(s); output=$output_file; highest_index=$count"
