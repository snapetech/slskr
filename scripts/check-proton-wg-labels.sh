#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pool_file="${SLSKR_PROTON_CREDENTIAL_POOL_FILE:-$repo_root/.secrets/proton-credential-pool.env}"
labels=(${SLSKR_PROTON_HEALTH_LABELS:-${SLSKR_PROTON_CONFIG_LABELS:-p1 p2 p3 p4 p5 p6 p7 p8}})
timeout_seconds="${SLSKR_PROTON_HEALTH_TIMEOUT_SECONDS:-35}"

if [[ -f "$pool_file" ]]; then
  # shellcheck disable=SC1090
  source "$pool_file"
fi

resolve_config() {
  local label="$1"
  local var_name="SLSKR_PROTON_CONFIG_${label}"
  local path="${!var_name:-}"
  if [[ -z "$path" ]]; then
    return 1
  fi
  if [[ "$path" != /* ]]; then
    path="$repo_root/$path"
  fi
  [[ -f "$path" ]] || return 1
  printf '%s' "$path"
}

printf 'timestamp\tlabel\tstatus\tdetail\n'
for label in "${labels[@]}"; do
  if ! config="$(resolve_config "$label")"; then
    printf '%s\t%s\tmissing\tconfig not found\n' "$(date -Is)" "$label"
    continue
  fi

  namespace="health${label}"
  stdout_file="$(mktemp)"
  stderr_file="$(mktemp)"
  set +e
  timeout "$timeout_seconds" "$repo_root/scripts/run-in-proton-wg-netns.sh" "$namespace" "$config" \
    bash -lc 'timeout 8 bash -c "</dev/tcp/1.1.1.1/53" || timeout 8 bash -c "</dev/tcp/9.9.9.9/53"' \
    >"$stdout_file" 2>"$stderr_file" &
  pid=$!
  sleep 8
  handshake="unavailable"
  if sudo ip netns exec "$namespace" true 2>/dev/null; then
    handshake="$(sudo ip netns exec "$namespace" wg show wg0 latest-handshakes 2>/dev/null | awk '{ print $2 }' | head -1)"
  fi
  wait "$pid"
  status=$?
  set -e

  if [[ "$status" -eq 0 && "${handshake:-0}" != "0" && "${handshake:-}" != "unavailable" && -n "${handshake:-}" ]]; then
    printf '%s\t%s\tok\thandshake=%s\n' "$(date -Is)" "$label" "$handshake"
  elif [[ "$status" -eq 0 ]]; then
    printf '%s\t%s\tok\tconnectivity=ok handshake=%s\n' "$(date -Is)" "$label" "${handshake:-unavailable}"
  else
    detail="$( { cat "$stdout_file"; tail -n 8 "$stderr_file"; } | tr '\n\t' '  ' | sed -E 's/[[:space:]]+/ /g; s/[A-Za-z0-9+\/]{30,}={0,2}/<redacted>/g; s/^ //; s/ $//' )"
    printf '%s\t%s\tfail\thandshake=%s detail=%s\n' "$(date -Is)" "$label" "${handshake:-0}" "$detail"
  fi
  rm -f "$stdout_file" "$stderr_file"
done
