#!/usr/bin/env bash
set -euo pipefail

# Public matrices start long-lived listeners and probe helpers. Keep direct
# invocation inside the same hard process-memory ceiling as certification.
runner_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if ! "$runner_repo_root/scripts/process-memory-guard-active.sh"; then
    exec "$runner_repo_root/scripts/with-process-memory-guard.sh" "${BASH_SOURCE[0]}" "$@"
fi

# The listener soak is launched asynchronously and performs its own guarded
# Cargo build while the first probe also builds/runs a probe command. Keep the
# repository-wide Rust serialization, but wait for the bounded hand-off rather
# than letting the listener fail before it claims its public ports.
export SLSKR_BUILD_LOCK_WAIT_SECONDS="${SLSKR_BUILD_LOCK_WAIT_SECONDS:-180}"
# Keep the asynchronous listener build and the probe builds on one Cargo
# fingerprint. Without this, the listener's warning-suppressed build forces
# every first probe to compile the full binary again inside its case timeout.
if [[ -n "${RUSTFLAGS:-}" ]]; then
    export RUSTFLAGS="${RUSTFLAGS} -Awarnings"
else
    export RUSTFLAGS="-Awarnings"
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pool_file="${SLSKR_PROTON_CREDENTIAL_POOL_FILE:-$repo_root/.secrets/proton-credential-pool.env}"
if [[ -f "$pool_file" ]]; then
    # shellcheck disable=SC1090
    source "$pool_file"
fi
listener_credential_file="${SLSKR_LISTENER_CREDENTIAL_FILE:-$repo_root/.secrets/live-listener-account.env}"
probe_credential_file="${SLSKR_PROBE_CREDENTIAL_FILE:-$repo_root/.secrets/live-probe-account.env}"
output_file="${1:-$repo_root/target/live-soak/proton-public-matrix-$(date +%Y%m%d-%H%M%S).tsv}"
matrix_account_mode="${SLSKR_MATRIX_ACCOUNT_MODE:-fixed}"
account_env_file="${SLSKR_LIVE_ENV_FILE:-$repo_root/.env}"
account_extra_env_file="${SLSKR_LIVE_EXTRA_ENV_FILE:-$repo_root/.secrets/generated-soulseek-accounts.env}"
inter_probe_delay_seconds="${SLSKR_MATRIX_INTER_PROBE_DELAY_SECONDS:-0}"

if [[ ! "$inter_probe_delay_seconds" =~ ^[0-9]+$ ]]; then
    echo "SLSKR_MATRIX_INTER_PROBE_DELAY_SECONDS must be a non-negative integer" >&2
    exit 2
fi

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
probe_password=""
if [[ "$matrix_account_mode" == "fixed" ]]; then
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
elif [[ "$matrix_account_mode" != "label" ]]; then
    echo "SLSKR_MATRIX_ACCOUNT_MODE must be fixed or label" >&2
    exit 2
fi

account_index_for_label() {
    local label="$1"
    if [[ "$label" =~ ^p([1-9][0-9]*)$ ]]; then
        printf '%s' "${BASH_REMATCH[1]}"
        return 0
    fi
    echo "account-label mode requires pN endpoint labels; got: $label" >&2
    return 1
}

account_field() {
    local index="$1"
    local field="$2"
    local variable="SLSKR_TEST_${index}_${field}"
    if [[ ! -f "$account_env_file" ]]; then
        echo "missing live account file: $account_env_file" >&2
        return 1
    fi
    (
        set -a
        # shellcheck disable=SC1090
        source "$account_env_file"
        if [[ -f "$account_extra_env_file" ]]; then
            # shellcheck disable=SC1090
            source "$account_extra_env_file"
        fi
        set +a
        printf '%s' "${!variable:-}"
    )
}

config_label_for_account() {
    local role="$1"
    local account_label="$2"
    local variable="SLSKR_PROTON_${role}_CONFIG_${account_label}"
    printf '%s' "${!variable:-$account_label}"
}

server_address="${SLSK_SERVER:-}"
if [[ -z "$server_address" ]]; then
    # The active public Soulseek endpoint used by the live account harness is
    # vps.slsknet.org:2271. The old server.slsknet.org:2242 default can accept
    # a listener while resetting the probe login, producing a false matrix
    # failure even when the credentials and VPN are valid.
    server_ip="$(getent ahostsv4 vps.slsknet.org | awk 'NR == 1 { print $1 }')"
    if [[ -z "$server_ip" ]]; then
        echo "failed to resolve vps.slsknet.org on host" >&2
        exit 1
    fi
    server_address="$server_ip:2271"
fi

slskr_binary="$repo_root/target/debug/slskr"
if [[ ! -x "$slskr_binary" ]]; then
    scripts/with-build-guard.sh cargo build -q -p slskr
fi
export SLSKR_MATRIX_BINARY="$slskr_binary"

printf 'timestamp\tlistener\tprobe\tcheck\tstatus\tdetail\n' >"$output_file"
matrix_failures=0

record() {
    local listener="$1"
    local probe="$2"
    local check="$3"
    local status="$4"
    local detail="$5"
    detail="${detail//$'\t'/ }"
    detail="${detail//$'\n'/ | }"
    # metadata-wait is an intentionally retryable observation; only its final
    # successful observation or the required matrix cases determine the exit
    # status. Every other recorded failure must make the acceptance run fail.
    if [[ "$status" != "ok" && "$check" != "metadata-wait" ]]; then
        matrix_failures=$((matrix_failures + 1))
    fi
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
    local config_label
    local probe_username_for_run="$probe_username"
    local probe_password_for_run="$probe_password"
    local output
    local status

    config_label="$(config_label_for_account PROBE "$probe")"
    if [[ -z "${configs[$config_label]:-}" ]]; then
        record "$listener" "$probe" "$check" "fail(1)" "unknown probe VPN profile: $config_label"
        return 0
    fi

    if [[ "$matrix_account_mode" == "label" ]]; then
        local probe_index
        probe_index="$(account_index_for_label "$probe")"
        probe_username_for_run="$(account_field "$probe_index" USERNAME)"
        probe_password_for_run="$(account_field "$probe_index" PASSWORD)"
        if [[ -z "$probe_username_for_run" || -z "$probe_password_for_run" ]]; then
            record "$listener" "$probe" "$check" "fail(1)" "missing credentials for account label $probe"
            return 0
        fi
    fi

    set +e
    output="$(
        SLSK_USERNAME="$probe_username_for_run" \
        SLSK_PASSWORD="$probe_password_for_run" \
        SLSK_SERVER="$server_address" \
        SLSK_PEER_USERNAME="$listener_username" \
        SLSK_PLAIN_PEER_USERNAME="$listener_username" \
        SLSK_OBFUSCATED_PEER_USERNAME="$listener_username" \
          timeout "${SLSKR_MATRIX_COMMAND_TIMEOUT_SECONDS:-${SLSKR_MATRIX_CASE_TIMEOUT_SECONDS:-45}}" \
            "$repo_root/scripts/run-in-proton-wg-netns.sh" "$namespace" "${configs[$config_label]}" \
            env \
                SLSK_PEER_ADDRESS_PROBE_ATTEMPTS=1 \
                SLSK_PEER_ADDRESS_PROBE_TIMEOUT_SECONDS=15 \
                SLSK_PEER_ADDRESS_SHOW_IP="${SLSKR_MATRIX_SHOW_PEER_IP:-0}" \
                SLSK_PLAIN_PROBE_TIMEOUT_SECONDS=15 \
                SLSK_OBFUSCATED_PROBE_TIMEOUT_SECONDS=15 \
                SLSK_PLAIN_PEER_INIT_TOKEN="${SLSKR_MATRIX_PLAIN_PEER_INIT_TOKEN:-0}" \
                SLSK_OBFUSCATED_PEER_INIT_TOKEN="${SLSKR_MATRIX_OBFUSCATED_PEER_INIT_TOKEN:-0}" \
                "$slskr_binary" "${command[@]}" 2>&1
    )"
    status=$?
    set -e

    output="vpn_profile=${config_label}; ${output}"

    # A server response with zero ports is a successful protocol frame but not
    # usable listener metadata. Treat it as a matrix failure so the public
    # matrix cannot report green while direct and obfuscated peer paths are
    # guaranteed to fail afterward.
    if [[ "$status" -eq 0 ]]; then
        case "$check" in
            metadata|metadata-wait)
                if ! grep -Eq 'peer address attempt=.* port=[1-9][0-9]* .*obfuscated_port=[1-9][0-9]*' <<<"$output"; then
                    status=1
                    output="${output}\ninvalid listener metadata: regular and obfuscated ports must both be nonzero"
                fi
                ;;
            metadata-relogin)
                if ! grep -Eq 'before_port=[1-9][0-9]* .*before_obfuscated_port=[1-9][0-9]* .*after_port=[1-9][0-9]* .*after_obfuscated_port=[1-9][0-9]*' <<<"$output"; then
                    status=1
                    output="${output}\ninvalid relogin metadata: regular and obfuscated ports must remain nonzero"
                fi
                ;;
        esac
    fi

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
    local config_label
    local probe_username_for_run="$probe_username"
    local probe_password_for_run="$probe_password"
    local output
    local status

    config_label="$(config_label_for_account PROBE "$probe")"
    if [[ -z "${configs[$config_label]:-}" ]]; then
        record "$listener" "$probe" "indirect" "fail(1)" "unknown probe VPN profile: $config_label"
        return 0
    fi

    if [[ "$matrix_account_mode" == "label" ]]; then
        local probe_index
        probe_index="$(account_index_for_label "$probe")"
        probe_username_for_run="$(account_field "$probe_index" USERNAME)"
        probe_password_for_run="$(account_field "$probe_index" PASSWORD)"
        if [[ -z "$probe_username_for_run" || -z "$probe_password_for_run" ]]; then
            record "$listener" "$probe" "indirect" "fail(1)" "missing credentials for account label $probe"
            return 0
        fi
    fi

    set +e
    output="$(
        SLSK_USERNAME="$probe_username_for_run" \
        SLSK_PASSWORD="$probe_password_for_run" \
        SLSK_SERVER="$server_address" \
        SLSK_PEER_USERNAME="$listener_username" \
        SLSK_INDIRECT_PEER_USERNAME="$listener_username" \
          timeout "${SLSKR_MATRIX_INDIRECT_TIMEOUT_SECONDS:-70}" \
            "$repo_root/scripts/run-in-proton-wg-netns.sh" "$namespace" "${configs[$config_label]}" \
            env \
                SLSK_INDIRECT_LISTENER_BIND="${SLSK_INDIRECT_LISTENER_BIND:-0.0.0.0:2236}" \
                SLSK_INDIRECT_PROBE_TIMEOUT_SECONDS=25 \
                SLSK_INDIRECT_TOKEN="${SLSKR_MATRIX_INDIRECT_TOKEN:-1370169345}" \
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
                    SLSK_INDIRECT_ADVERTISED_PORT="$public_port" "$SLSKR_MATRIX_BINARY" probe indirect-peer
                ' 2>&1
    )"
    status=$?
    set -e

    output="vpn_profile=${config_label}; ${output}"

    if [[ "$status" -eq 0 ]]; then
        record "$listener" "$probe" "indirect" "ok" "$output"
    else
        record "$listener" "$probe" "indirect" "fail($status)" "$output"
    fi
}

run_negative_indirect_probe() {
    local listener="$1"
    local probe="$2"
    local saved_listener_username="$listener_username"
    # The negative case must target an offline/nonexistent peer. Reusing the
    # live listener exercises the positive indirect path and cannot produce a
    # deterministic CantConnectToPeer response.
    listener_username="${SLSKR_MATRIX_NEGATIVE_PEER_USERNAME:-slskrMatrixOffline${listener}}"

    SLSKR_MATRIX_COMMAND_TIMEOUT_SECONDS="${SLSKR_MATRIX_NEGATIVE_TIMEOUT_SECONDS:-45}" \
        run_probe "$listener" "$probe" "negative-indirect" probe negative-indirect
    listener_username="$saved_listener_username"
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
    listener_config_label="$(config_label_for_account LISTENER "$listener")"
    if [[ -z "${configs[$listener_config_label]:-}" ]]; then
        echo "unknown listener VPN profile: $listener_config_label" >&2
        exit 1
    fi
    listener_account_index=""
    if [[ "$matrix_account_mode" == "label" ]]; then
        listener_account_index="$(account_index_for_label "$listener")"
        listener_username="$(account_field "$listener_account_index" USERNAME)"
        if [[ -z "$listener_username" || -z "$(account_field "$listener_account_index" PASSWORD)" ]]; then
            echo "missing credentials for account label $listener" >&2
            exit 1
        fi
    fi
    SLSKR_SOAK_CREDENTIAL_FILE="$listener_credential_file" \
    SLSKR_SOAK_ACCOUNT_INDEX="$listener_account_index" \
    SLSKR_LIVE_ENV_FILE="$account_env_file" \
    SLSKR_LIVE_EXTRA_ENV_FILE="$account_extra_env_file" \
    SLSKR_SOAK_SKIP_BUILD=1 \
    SLSKR_MATRIX_BINARY="$slskr_binary" \
    SLSK_SERVER="$server_address" \
    SLSKR_PROTON_ADVERTISE_REGULAR_LOCAL="${SLSKR_MATRIX_ADVERTISE_REGULAR_LOCAL:-1}" \
        "$repo_root/scripts/start-proton-listener-soak.sh" "${configs[$listener_config_label]}" "$listener" >/dev/null

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
        if (( inter_probe_delay_seconds > 0 )); then
            sleep "$inter_probe_delay_seconds"
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

if (( matrix_failures > 0 )); then
    echo "matrix failed: $matrix_failures required case(s) failed; evidence=$output_file" >&2
    exit 1
fi

echo "matrix complete: $output_file"
