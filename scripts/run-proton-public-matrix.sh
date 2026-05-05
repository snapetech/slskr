#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pool_file="${SLSKR_PROTON_CREDENTIAL_POOL_FILE:-$repo_root/.secrets/proton-credential-pool.env}"
if [[ -f "$pool_file" ]]; then
    # shellcheck disable=SC1090
    source "$pool_file"
fi
listener_credential_file="${SLSKR_LISTENER_CREDENTIAL_FILE:-$repo_root/.secrets/live-listener-account.env}"
probe_credential_file="${SLSKR_PROBE_CREDENTIAL_FILE:-$repo_root/.secrets/live-probe-account.env}"
output_file="${1:-$repo_root/target/live-soak/proton-public-matrix-$(date +%Y%m%d-%H%M%S).tsv}"

default_labels="${SLSKR_PROTON_CONFIG_LABELS:-il741 au162 usca32 uk577}"
listener_labels=(${SLSKR_MATRIX_LISTENERS:-${SLSKR_PROTON_LISTENER_LABELS:-$default_labels}})
probe_labels=(${SLSKR_MATRIX_PROBES:-${SLSKR_PROTON_PROBE_LABELS:-$default_labels}})

declare -A configs=(
    [il741]="$repo_root/.secrets/proton-slskr-1.conf"
    [au162]="$repo_root/.secrets/proton-slskr-2.conf"
    [usca32]="$repo_root/.secrets/proton-slskr-3.conf"
    [uk577]="$repo_root/.secrets/proton-slskr-4.conf"
)
for label in $default_labels; do
    var_name="SLSKR_PROTON_CONFIG_${label}"
    if [[ -n "${!var_name:-}" ]]; then
        configured_path="${!var_name}"
        if [[ "$configured_path" != /* ]]; then
            configured_path="$repo_root/$configured_path"
        fi
        configs[$label]="$configured_path"
    fi
done

mkdir -p "$(dirname "$output_file")"

require_file() {
    local path="$1"
    if [[ ! -f "$path" ]]; then
        echo "missing required file: $path" >&2
        exit 1
    fi
}

for label in "${listener_labels[@]}" "${probe_labels[@]}"; do
    if [[ -z "${configs[$label]:-}" ]]; then
        echo "unknown Proton config label: $label" >&2
        exit 2
    fi
    require_file "${configs[$label]}"
done
require_file "$listener_credential_file"
require_file "$probe_credential_file"

listener_username="$(
    set -a
    # shellcheck disable=SC1090
    source "$listener_credential_file"
    set +a
    printf '%s' "${SLSKR_LISTENER_USERNAME:-${SLSK_USERNAME:-}}"
)"
if [[ -z "$listener_username" ]]; then
    echo "listener credential file does not define SLSKR_LISTENER_USERNAME or SLSK_USERNAME" >&2
    exit 1
fi

probe_username="$(
    set -a
    # shellcheck disable=SC1090
    source "$probe_credential_file"
    set +a
    printf '%s' "${SLSK_USERNAME:-${SLSKR_PROBE_USER:-}}"
)"
probe_password="$(
    set -a
    # shellcheck disable=SC1090
    source "$probe_credential_file"
    set +a
    printf '%s' "${SLSK_PASSWORD:-${SLSKR_PROBE_PASSWORD:-}}"
)"
if [[ -z "$probe_username" || -z "$probe_password" ]]; then
    echo "probe credential file does not define SLSK_USERNAME/SLSK_PASSWORD or SLSKR_PROBE_USER/SLSKR_PROBE_PASSWORD" >&2
    exit 1
fi

server_address="${SLSK_SERVER:-}"
if [[ -z "$server_address" ]]; then
    server_ip="$(getent ahostsv4 server.slsknet.org | awk 'NR == 1 { print $1 }')"
    if [[ -z "$server_ip" ]]; then
        echo "failed to resolve server.slsknet.org on host" >&2
        exit 1
    fi
    server_address="$server_ip:2242"
fi

printf 'timestamp\tlistener\tprobe\tcheck\tstatus\tdetail\n' >"$output_file"

record() {
    local listener="$1"
    local probe="$2"
    local check="$3"
    local status="$4"
    local detail="$5"
    detail="${detail//$'\t'/ }"
    detail="${detail//$'\n'/ | }"
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$(date -Is)" "$listener" "$probe" "$check" "$status" "$detail" | tee -a "$output_file"
}

run_probe() {
    local listener="$1"
    local probe="$2"
    local check="$3"
    shift 3
    local command=("$@")
    local namespace="m${probe}"
    local output
    local status

    set +e
    output="$(
        timeout "${SLSKR_MATRIX_COMMAND_TIMEOUT_SECONDS:-${SLSKR_MATRIX_CASE_TIMEOUT_SECONDS:-45}}" \
            "$repo_root/scripts/run-in-proton-wg-netns.sh" "$namespace" "${configs[$probe]}" \
            env \
                SLSK_USERNAME="$probe_username" \
                SLSK_PASSWORD="$probe_password" \
                SLSK_SERVER="$server_address" \
                SLSK_PEER_USERNAME="$listener_username" \
                SLSK_PLAIN_PEER_USERNAME="$listener_username" \
                SLSK_OBFUSCATED_PEER_USERNAME="$listener_username" \
                SLSK_PEER_ADDRESS_PROBE_ATTEMPTS=1 \
                SLSK_PEER_ADDRESS_PROBE_TIMEOUT_SECONDS=15 \
                SLSK_PEER_ADDRESS_SHOW_IP="${SLSKR_MATRIX_SHOW_PEER_IP:-0}" \
                SLSK_PLAIN_PROBE_TIMEOUT_SECONDS=15 \
                SLSK_OBFUSCATED_PROBE_TIMEOUT_SECONDS=15 \
                SLSK_PLAIN_PEER_INIT_TOKEN="${SLSKR_MATRIX_PLAIN_PEER_INIT_TOKEN:-0}" \
                SLSK_OBFUSCATED_PEER_INIT_TOKEN="${SLSKR_MATRIX_OBFUSCATED_PEER_INIT_TOKEN:-0}" \
                cargo run -q -p slskr -- "${command[@]}" 2>&1
    )"
    status=$?
    set -e

    if [[ "$status" -eq 0 ]]; then
        record "$listener" "$probe" "$check" "ok" "$output"
    else
        record "$listener" "$probe" "$check" "fail($status)" "$output"
    fi
}

run_indirect_probe() {
    local listener="$1"
    local probe="$2"
    local namespace="m${probe}"
    local output
    local status

    set +e
    output="$(
        timeout "${SLSKR_MATRIX_INDIRECT_TIMEOUT_SECONDS:-70}" \
            "$repo_root/scripts/run-in-proton-wg-netns.sh" "$namespace" "${configs[$probe]}" \
            env \
                SLSK_USERNAME="$probe_username" \
                SLSK_PASSWORD="$probe_password" \
                SLSK_SERVER="$server_address" \
                SLSK_PEER_USERNAME="$listener_username" \
                SLSK_INDIRECT_PEER_USERNAME="$listener_username" \
                SLSK_INDIRECT_LISTENER_BIND="${SLSK_INDIRECT_LISTENER_BIND:-0.0.0.0:2236}" \
                SLSK_INDIRECT_PROBE_TIMEOUT_SECONDS=25 \
                SLSK_INDIRECT_SEND_PEER_ADDRESS="${SLSKR_MATRIX_INDIRECT_SEND_PEER_ADDRESS:-0}" \
                bash -lc '
                    set -euo pipefail
                    local_port="${SLSK_INDIRECT_LISTENER_BIND##*:}"
                    mapping="$(natpmpc -g "${PROTON_NATPMP_GATEWAY:-10.2.0.1}" -a 0 "$local_port" tcp 60)"
                    printf "%s\n" "$mapping" >&2
                    public_port="$(awk "/Mapped public port/ { for (i = 1; i <= NF; i++) if (\$i == \"port\") { print \$(i + 1); exit } }" <<<"$mapping")"
                    if [[ -z "$public_port" ]]; then
                        echo "failed to claim indirect NAT-PMP public port" >&2
                        exit 1
                    fi
                    renew() {
                        while true; do
                            natpmpc -g "${PROTON_NATPMP_GATEWAY:-10.2.0.1}" -a "$public_port" "$local_port" tcp 60 >/dev/null 2>&1 || true
                            sleep 45
                        done
                    }
                    renew &
                    renew_pid=$!
                    trap "kill \"$renew_pid\" 2>/dev/null || true" EXIT
                    SLSK_INDIRECT_ADVERTISED_PORT="$public_port" cargo run -q -p slskr -- probe indirect-peer
                ' 2>&1
    )"
    status=$?
    set -e

    if [[ "$status" -eq 0 ]]; then
        record "$listener" "$probe" "indirect" "ok" "$output"
    else
        record "$listener" "$probe" "indirect" "fail($status)" "$output"
    fi
}

run_negative_indirect_probe() {
    local listener="$1"
    local probe="$2"

    SLSKR_MATRIX_COMMAND_TIMEOUT_SECONDS="${SLSKR_MATRIX_NEGATIVE_TIMEOUT_SECONDS:-45}" \
        run_probe "$listener" "$probe" "negative-indirect" probe negative-indirect
}

wait_for_metadata() {
    local listener="$1"
    local probe="$2"
    local deadline=$((SECONDS + ${SLSKR_MATRIX_METADATA_WAIT_SECONDS:-90}))
    local status

    while (( SECONDS < deadline )); do
        run_probe "$listener" "$probe" "metadata-wait" probe peer-address
        status="$(tail -n 1 "$output_file" | cut -f5)"
        if [[ "$status" == "ok" ]]; then
            return 0
        fi
        sleep 5
    done
    return 1
}

echo "writing matrix results to $output_file"

for listener in "${listener_labels[@]}"; do
    echo "starting listener endpoint: $listener"
    SLSKR_SOAK_CREDENTIAL_FILE="$listener_credential_file" \
    SLSKR_PROTON_ADVERTISE_REGULAR_LOCAL="${SLSKR_MATRIX_ADVERTISE_REGULAR_LOCAL:-1}" \
        "$repo_root/scripts/start-proton-listener-soak.sh" "${configs[$listener]}" "$listener" >/dev/null

    metadata_probe=""
    for probe in "${probe_labels[@]}"; do
        if [[ "$probe" != "$listener" ]]; then
            metadata_probe="$probe"
            break
        fi
    done
    if [[ -z "$metadata_probe" ]]; then
        echo "no usable probe endpoint for listener $listener" >&2
        exit 1
    fi

    wait_for_metadata "$listener" "$metadata_probe" || true

    for probe in "${probe_labels[@]}"; do
        if [[ "$probe" == "$listener" ]]; then
            continue
        fi
        echo "probing listener=$listener from probe=$probe"
        run_probe "$listener" "$probe" "metadata" probe peer-address
        run_probe "$listener" "$probe" "plain-direct" probe plain-peer
        run_probe "$listener" "$probe" "obfuscated-direct" probe obfuscated-peer
        run_probe "$listener" "$probe" "distributed-direct" probe distributed-peer
        run_probe "$listener" "$probe" "file-transfer-direct" probe file-transfer-peer
        run_indirect_probe "$listener" "$probe"
        run_probe "$listener" "$probe" "metadata-relogin" probe metadata-relogin
        run_negative_indirect_probe "$listener" "$probe"
    done
done

echo "matrix complete: $output_file"
