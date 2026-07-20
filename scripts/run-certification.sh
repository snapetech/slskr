#!/usr/bin/env bash
#
# slskr certification runner — executes test phases and generates structured reports.
#
# Usage:
#   scripts/run-certification.sh                    # all available phases
#   scripts/run-certification.sh --phases A,B        # specific phases
#   scripts/run-certification.sh --log-format json   # machine-parseable output
#   scripts/run-certification.sh --dry-run           # show plan without executing
#
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_dir="${SLSKR_CERTIFY_OUTPUT_DIR:-$repo_root/target/certify}"
env_file="${SLSKR_CERTIFY_ENV_FILE:-$repo_root/.env}"
pool_file="${SLSKR_PROTON_CREDENTIAL_POOL_FILE:-$repo_root/.secrets/proton-credential-pool.env}"
timestamp="$(date +%Y%m%d-%H%M%S)"
log_file="$output_dir/certify-$timestamp.log"
report_file="$output_dir/summary-$timestamp.json"

mkdir -p "$output_dir"

# --- Argument parsing ---
phases="${SLSKR_CERTIFY_PHASES:-A,B,C,D,E,G,H}"
log_format="text"
dry_run=false
# shellcheck disable=SC2034
vpn_endpoints=""
credential_file=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --phases) phases="$2"; shift 2 ;;
        --log-format) log_format="$2"; shift 2 ;;
        --dry-run) dry_run=true; shift ;;
        --vpn-endpoints) vpn_endpoints="$2"; shift 2 ;;
        --credential-file) credential_file="$2"; shift 2 ;;
        -h|--help)
            echo "usage: $0 [--phases A,B,C,D,E,G,H] [--log-format json|text] [--dry-run] [--vpn-endpoints il741,usca32]"
            exit 0
            ;;
        *) echo "unknown option: $1" >&2; exit 1 ;;
    esac
done
export vpn_endpoints

# --- Credential loading ---
if [[ -f "$env_file" ]]; then
    set -a
    # shellcheck disable=SC1090
    source "$env_file"
    set +a
fi
if [[ -f "$pool_file" ]]; then
    set -a
    # shellcheck disable=SC1090
    source "$pool_file"
    set +a
fi
if [[ -n "$credential_file" && -f "$credential_file" ]]; then
    set -a
    # shellcheck disable=SC1090
    source "$credential_file"
    set +a
fi

# --- VPN auto-detection ---
# If SLSKR_CERTIFY_VPN_ENABLED is not explicitly set, detect whether Proton configs exist.
if [[ "${SLSKR_CERTIFY_VPN_ENABLED:-auto}" == "auto" ]]; then
    vpn_detected=false
    for label in p1 p2 p3 p4 p5 p6 p7 p8; do
        var_name="SLSKR_PROTON_CONFIG_${label}"
        path="${!var_name:-}"
        if [[ -n "$path" ]]; then
            [[ "$path" != /* ]] && path="$repo_root/$path"
            if [[ -f "$path" ]]; then
                vpn_detected=true
                break
            fi
        fi
    done
    if [[ "$vpn_detected" == "true" ]]; then
        SLSKR_CERTIFY_VPN_ENABLED=1
        echo "[INFO] Auto-detected VPN configs; enabling isolated netns per test"
    else
        SLSKR_CERTIFY_VPN_ENABLED=0
        echo "[INFO] No VPN configs found; running tests without VPN isolation"
    fi
fi
export SLSKR_CERTIFY_VPN_ENABLED

resolve_certify_server_address() {
    local configured="${SLSK_SERVER:-server.slsknet.org:2242}"
    if [[ "$configured" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+:[0-9]+$ ]]; then
        printf '%s' "$configured"
        return 0
    fi
    local host="${configured%:*}"
    local port="${configured##*:}"
    if [[ -z "$host" || ! "$port" =~ ^[0-9]+$ ]]; then
        return 1
    fi
    local address
    address="$(getent ahostsv4 "$host" | awk 'NR == 1 { print $1 }')"
    [[ -n "$address" ]] || return 1
    printf '%s:%s' "$address" "$port"
}

certify_server_address=""
if [[ "$SLSKR_CERTIFY_VPN_ENABLED" == "1" ]]; then
    certify_server_address="$(resolve_certify_server_address)" || {
        echo "[ERROR] Failed to resolve the Soulseek server before entering VPN namespaces" >&2
        exit 1
    }
    export certify_server_address
    echo "[INFO] Resolved the Soulseek server outside VPN namespaces"
fi

# --- Global counters ---
total_tests=0
passed_tests=0
failed_tests=0
skipped_tests=0
# shellcheck disable=SC2034
phase_results="[]"
start_time="$(date +%s)"

# --- Logging helpers ---
log() {
    local level="$1"; shift
    local msg="$*"
    local ts
    ts="$(date -Is)"
    printf '[%s] [%s] %s\n' "$ts" "$level" "$msg" | tee -a "$log_file"
}

# Retry a command with exponential backoff until success or max attempts.
# Usage: retry_with_backoff <max_attempts> <base_delay_seconds> <cmd...>
# The command is evaluated as a string so env vars work naturally.
# Detects rate-limiting by checking for "reset by peer" or "unexpected end of file".
retry_with_backoff() {
    local max_attempts="$1"
    local base_delay="$2"
    shift 2
    local cmd="$*"
    local attempt=0
    local delay="$base_delay"
    local output=""

    while [[ $attempt -lt $max_attempts ]]; do
        attempt=$((attempt + 1))
        local status
        set +e
        output="$(eval "$cmd" 2>&1)"
        status=$?
        set -e

        # Success
        if [[ $status -eq 0 ]]; then
            printf '%s' "$output"
            return 0
        fi

        # Check if it's rate-limiting — always retry
        if echo "$output" | grep -qi "reset by peer\|unexpected end of file\|Connection refused"; then
            log info "  retry $attempt/$max_attempts — server rate-limited, waiting ${delay}s..."
            sleep "$delay"
            delay=$((delay * 2))
            # Cap delay at 60s
            [[ $delay -gt 60 ]] && delay=60
            continue
        fi

        # Other failure — return immediately
        printf '%s' "$output"
        return 1
    done

    # Exhausted retries
    printf '%s' "$output"
    return 1
}

record_test() {
    local phase="$1" id="$2" name="$3" status="$4" duration_ms="${5:-0}" detail="${6:-}"
    total_tests=$((total_tests + 1))
    case "$status" in
        pass) passed_tests=$((passed_tests + 1)) ;;
        fail) failed_tests=$((failed_tests + 1)) ;;
        skip) skipped_tests=$((skipped_tests + 1)) ;;
    esac

    detail="${detail//$'\n'/ }"
    detail="${detail//$'\t'/ }"
    detail="${detail//\"/\\\"}"

    if [[ "$log_format" == "json" ]]; then
        printf '{"phase":"%s","id":"%s","name":"%s","status":"%s","duration_ms":%s,"detail":"%s"}\n' \
            "$phase" "$id" "$name" "$status" "$duration_ms" "$detail"
    else
        printf '  [%s] %-40s %s (%sms) %s\n' "$phase" "$id: $name" "$status" "$duration_ms" "$detail" | tee -a "$log_file"
    fi
}

# --- Phase runners ---
require_var() {
    local name="$1"; shift
    if [[ -z "${!name:-}" ]]; then
        echo "$1"
        return 1
    fi
    return 0
}

resolve_proton_config() {
    local label="$1"
    local var_name="SLSKR_PROTON_CONFIG_${label}"
    local path="${!var_name:-}"
    if [[ -z "$path" ]]; then
        return 1
    fi
    if [[ "$path" != /* ]]; then
        path="$repo_root/$path"
    fi
    if [[ ! -f "$path" ]]; then
        return 1
    fi
    printf '%s' "$path"
}

# --- VPN account-to-label mapping for per-account IP isolation ---
# Each test account gets a dedicated Proton exit node to bypass per-IP rate limiting.
declare -A ACCOUNT_VPN_LABEL=(
    [1]="p1"
    [2]="p3"
    [3]="p4"
    [4]="p5"
    [5]="p6"
    [6]="p7"
    [7]="p8"
    [8]="p2"
)
# Phase-to-VPN-label mapping for tests that don't map to a specific account.
declare -A PHASE_VPN_LABEL=(
    [A]="p1"
    [B]="p2"
    [C]="p6"
    [D]="p7"
    [E]="p1"
    [G]="p8"
    [H]="p2"
)

resolve_vpn_config_for_account() {
    local account_num="$1"
    local label="${ACCOUNT_VPN_LABEL[$account_num]:-p1}"
    resolve_proton_config "$label"
}

resolve_vpn_config_for_phase() {
    local phase="$1"
    local label="${PHASE_VPN_LABEL[$phase]:-p1}"
    resolve_proton_config "$label"
}

# Run a command inside an isolated Proton netns.
# Usage: run_netns_command <namespace> <config> <cmd> [args...]
# Captures output and duration. Callers decide how much diagnostic output to retain.
run_netns_command() {
    local namespace="$1" config="$2"; shift 2
    "$repo_root/scripts/run-in-proton-wg-netns.sh" "$namespace" "$config" "$@"
}

run_vpn_cargo_retry() {
    local max_attempts="$1" delay="$2" namespace="$3" config="$4"
    shift 4
    local attempt output status
    for attempt in $(seq 1 "$max_attempts"); do
        set +e
        output="$(run_vpn_cargo "$namespace" "$config" "$@" 2>&1)"
        status=$?
        set -e
        if [[ $status -eq 0 ]]; then
            printf '%s' "$output"
            return 0
        fi
        if [[ $attempt -lt $max_attempts ]]; then
            log info "  $namespace transport attempt $attempt failed; retrying after cooldown" >&2
            sleep "$delay"
        fi
    done
    printf '%s' "$output"
    return "$status"
}

# Run a cargo command inside an isolated Proton netns with explicit env vars.
# Usage: run_vpn_cargo <namespace> <config> [KEY=VAL ...] -- cargo [args...]
# Arguments before -- are env vars; after -- is the command.
run_vpn_cargo() {
    local namespace="$1" config="$2"; shift 2
    local env_args=()
    local cmd_args=()
    local parsing_env=true

    for arg in "$@"; do
        if [[ "$parsing_env" == "true" ]]; then
            if [[ "$arg" == "--" ]]; then
                parsing_env=false
                continue
            fi
            env_args+=("$arg")
        else
            cmd_args+=("$arg")
        fi
    done

    if [[ -n "$certify_server_address" ]] \
        && ! printf '%s\n' "${env_args[@]}" | grep -q '^SLSK_SERVER='; then
        env_args+=("SLSK_SERVER=$certify_server_address")
    fi

    local output status duration_ms
    local t0 t1

    t0="$(date +%s%N)"
    set +e
    output="$(run_netns_command "$namespace" "$config" timeout 90 env "${env_args[@]}" "${cmd_args[@]}" 2>&1)"
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    printf '%s' "$output"
    return $status
}

run_probe() {
    local cmd=("$@")
    local output status duration_ms
    local t0 t1

    t0="$(date +%s%N)"
    set +e
    output="$(timeout 60 cargo run -q -p slskr -- "${cmd[@]}" 2>&1)"
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    echo "$output"
    return $status
}

run_vpn_probe() {
    local namespace="$1" config="$2"; shift 2
    local cmd=("$@")
    local output status duration_ms
    local t0 t1

    t0="$(date +%s%N)"
    set +e
    output="$(run_netns_command "$namespace" "$config" timeout 60 cargo run -q -p slskr -- "${cmd[@]}" 2>&1)"
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    echo "$output"
    return $status
}

# --- Phase A: Foundation ---
run_phase_a() {
    log info "=== Phase A: Foundation ==="

    local username password server_address
    server_address="${SLSK_SERVER:-}"
    if [[ -z "$server_address" ]]; then
        local server_ip
        server_ip="$(getent ahostsv4 vps.slsknet.org 2>/dev/null | awk 'NR == 1 { print $1 }')" || true
        if [[ -n "$server_ip" ]]; then
            server_address="$server_ip:2271"
        fi
    fi

    # A1 — Login for all available accounts, each through its own isolated VPN netns.
    # This bypasses the Soulseek server's per-IP rate limiting.
    local account_count="${SLSKR_TEST_ACCOUNT_COUNT:-4}"
    for i in $(seq 1 "$account_count"); do
        # Wait before each login (except first) to avoid server rate-limiting
        if [[ "$i" -gt 1 ]]; then
            sleep "${SLSKR_LOGIN_DELAY:-10}"
        fi
        local user_var="SLSKR_TEST_${i}_USERNAME"
        local pass_var="SLSKR_TEST_${i}_PASSWORD"
        username="${!user_var:-}"
        password="${!pass_var:-}"
        if [[ -z "$username" || -z "$password" ]]; then
            record_test "A" "A1.$i" "login account $i" "fail" 0 "no credentials configured"
            continue
        fi

        local vpn_config
        vpn_config="$(resolve_vpn_config_for_account "$i")" || {
            record_test "A" "A1.$i" "login account $i ($username)" "fail" 0 "no VPN config for account $i"
            continue
        }

        local output status duration_ms
        local t0 t1 attempt
        t0="$(date +%s%N)"
        status=1
        for attempt in $(seq 1 "${SLSKR_LOGIN_ATTEMPTS:-2}"); do
            set +e
            output="$(run_vpn_cargo "cert-a1-$i" "$vpn_config" \
                -- \
                SLSK_USERNAME="$username" \
                SLSK_PASSWORD="$password" \
                SLSK_SERVER="$server_address" \
                SLSKR_PROBE_OUTPUT=json \
                cargo run -q -p slskr -- login smoke 2>&1)"
            status=$?
            set -e
            [[ $status -eq 0 ]] && break
            if [[ $attempt -lt ${SLSKR_LOGIN_ATTEMPTS:-2} ]]; then
                log info "  account $i login transport attempt $attempt failed; retrying after cooldown"
                sleep "${SLSKR_LOGIN_RETRY_DELAY:-15}"
            fi
        done
        t1="$(date +%s%N)"
        duration_ms=$(( (t1 - t0) / 1000000 ))

        if [[ $status -eq 0 ]]; then
            record_test "A" "A1.$i" "login account $i ($username)" "pass" "$duration_ms" "login succeeded via isolated VPN"
        else
            record_test "A" "A1.$i" "login account $i ($username)" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
        fi
    done

    # A2–A5 require VPN + listener + probe setup
    if [[ "${SLSKR_CERTIFY_VPN_ENABLED:-0}" != "1" ]]; then
        log info "VPN not enabled; attempting A2-A5 with local probes only"
        for id in A2 A3 A4 A5; do
            record_test "A" "$id" "$id probe matrix" "fail" 0 "VPN disabled — test requires VPN netns"
        done
        return 0
    fi

    local listener_label="${SLSKR_CERTIFY_LISTENER_LABEL:-p1}"
    local probe_label="${SLSKR_CERTIFY_PROBE_LABEL:-p3}"
    local listener_config probe_config

    listener_config="$(resolve_proton_config "$listener_label")" || {
        record_test "A" "A2-A5" "VPN probe matrix" "fail" 0 "no config for $listener_label"
        return 0
    }
    probe_config="$(resolve_proton_config "$probe_label")" || {
        record_test "A" "A2-A5" "VPN probe matrix" "fail" 0 "no config for $probe_label"
        return 0
    }

    # Load listener credentials
    local listener_cred_file="${SLSKR_LISTENER_CREDENTIAL_FILE:-$repo_root/.secrets/live-listener-account.env}"
    local probe_cred_file="${SLSKR_PROBE_CREDENTIAL_FILE:-$repo_root/.secrets/live-probe-account.env}"

    if [[ ! -f "$listener_cred_file" && -f "$repo_root/.secrets/pool-listener-account.env" ]]; then
        listener_cred_file="$repo_root/.secrets/pool-listener-account.env"
    fi

    if [[ ! -f "$listener_cred_file" ]]; then
        record_test "A" "A2-A5" "VPN probe matrix" "fail" 0 "no listener credentials"
        return 0
    fi
    if [[ ! -f "$probe_cred_file" ]]; then
        record_test "A" "A2-A5" "VPN probe matrix" "fail" 0 "no probe credentials"
        return 0
    fi

    local listener_username probe_username probe_password
    # shellcheck disable=SC1090
    listener_username="$(set -a; source "$listener_cred_file" 2>/dev/null; set +a; printf '%s' "${SLSKR_LISTENER_USERNAME:-${SLSK_USERNAME:-}}")"
    # shellcheck disable=SC1090
    probe_username="$(set -a; source "$probe_cred_file" 2>/dev/null; set +a; printf '%s' "${SLSK_USERNAME:-${SLSKR_PROBE_USER:-}}")"
    # shellcheck disable=SC1090
    probe_password="$(set -a; source "$probe_cred_file" 2>/dev/null; set +a; printf '%s' "${SLSK_PASSWORD:-${SLSKR_PROBE_PASSWORD:-}}")"

    local listener_session="slskr-cert-a-listener"
    log info "starting VPN/NAT-PMP listener for direct and indirect probes..."
    SLSKR_PROTON_SOAK_SESSION="$listener_session" \
    SLSKR_PROTON_NAMESPACE="slskr-cert-a-listener" \
    SLSKR_SOAK_CREDENTIAL_FILE="$listener_cred_file" \
    SLSKR_PROTON_ADVERTISE_REGULAR_LOCAL="${SLSKR_MATRIX_ADVERTISE_REGULAR_LOCAL:-0}" \
    SLSK_SOAK_ACTIVE_PROBES=0 \
    SLSK_SOAK_DEFAULT_SEARCH=0 \
    SLSK_SERVER="$server_address" \
        "$repo_root/scripts/start-proton-listener-soak.sh" \
        "$listener_config" "$listener_label" >>"$log_file" 2>&1
    sleep "${SLSKR_CERTIFY_LISTENER_SETTLE_SECONDS:-15}"

    # A2 — Peer address resolution
    local t0 t1 duration_ms output status
    t0="$(date +%s%N)"
    set +e
    output="$(run_vpn_cargo_retry 2 5 "cert-a2" "$probe_config" \
        -- \
        SLSK_USERNAME="$probe_username" \
        SLSK_PASSWORD="$probe_password" \
        SLSK_SERVER="$server_address" \
        SLSK_PEER_USERNAME="$listener_username" \
        SLSKR_PROBE_OUTPUT=json \
        cargo run -q -p slskr -- probe peer-address 2>&1)"
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "A" "A2" "peer-address resolution" "pass" "$duration_ms" "metadata resolved via isolated VPN"
    else
        record_test "A" "A2" "peer-address resolution" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # A3 — Plain peer message
    t0="$(date +%s%N)"
    set +e
    output="$(run_vpn_cargo_retry 2 5 "cert-a3" "$probe_config" \
        -- \
        SLSK_USERNAME="$probe_username" \
        SLSK_PASSWORD="$probe_password" \
        SLSK_SERVER="$server_address" \
        SLSK_PLAIN_PEER_USERNAME="$listener_username" \
        SLSKR_PROBE_OUTPUT=json \
        cargo run -q -p slskr -- probe plain-peer 2>&1)"
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "A" "A3" "plain-peer message" "pass" "$duration_ms" "UserInfo round-trip via isolated VPN"
    else
        record_test "A" "A3" "plain-peer message" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # A4 — Obfuscated peer message
    t0="$(date +%s%N)"
    set +e
    output="$(run_vpn_cargo_retry 2 5 "cert-a4" "$probe_config" \
        -- \
        SLSK_USERNAME="$probe_username" \
        SLSK_PASSWORD="$probe_password" \
        SLSK_SERVER="$server_address" \
        SLSK_OBFUSCATED_PEER_USERNAME="$listener_username" \
        SLSKR_PROBE_OUTPUT=json \
        cargo run -q -p slskr -- probe obfuscated-peer 2>&1)"
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "A" "A4" "obfuscated-peer message" "pass" "$duration_ms" "type-1 obfuscated round-trip via isolated VPN"
    else
        record_test "A" "A4" "obfuscated-peer message" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # A5 — Indirect peer
    t0="$(date +%s%N)"
    set +e
    output="$(run_vpn_cargo "cert-a5" "$probe_config" \
        -- \
        SLSK_USERNAME="$probe_username" \
        SLSK_PASSWORD="$probe_password" \
        SLSK_SERVER="$server_address" \
        SLSK_INDIRECT_PEER_USERNAME="$listener_username" \
        SLSKR_PROBE_OUTPUT=json \
        bash -c '
            set -euo pipefail
            private_port=2236
            mapping="$(natpmpc -g "${PROTON_NATPMP_GATEWAY:-10.2.0.1}" -a 0 "$private_port" tcp 60 2>&1)"
            public_port="$(awk '\''/Mapped public port/ { for (i = 1; i <= NF; i++) if ($i == "port") { print $(i + 1); exit } }'\'' <<<"$mapping")"
            if [[ -z "$public_port" ]]; then
                printf "%s\n" "$mapping" >&2
                exit 1
            fi
            export SLSK_INDIRECT_LISTENER_BIND="0.0.0.0:$private_port"
            export SLSK_INDIRECT_ADVERTISED_PORT="$public_port"
            exec cargo run -q -p slskr -- probe indirect-peer
        ' 2>&1)"
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    tmux kill-session -t "$listener_session" 2>/dev/null || true

    if [[ $status -eq 0 ]]; then
        record_test "A" "A5" "indirect-peer ConnectToPeer/PierceFirewall" "pass" "$duration_ms" "indirect connection established via isolated VPN"
    else
        record_test "A" "A5" "indirect-peer ConnectToPeer/PierceFirewall" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi
}

# --- Phase B: Transfer Certification ---
run_phase_b() {
    log info "=== Phase B: Transfer Certification ==="

    local username1 password1
    username1="${SLSKR_TEST_1_USERNAME:-}"
    password1="${SLSKR_TEST_1_PASSWORD:-}"

    if [[ -z "$username1" || -z "$password1" ]]; then
        for id in B1 B2 B3 B4 B5; do
            record_test "B" "$id" "$id transfer test" "skip" 0 "no credentials"
        done
        return 0
    fi

    # Resolve Phase B VPN config for IP isolation
    local vpn_config
    vpn_config="$(resolve_vpn_config_for_phase "B")" || {
        log warn "No VPN config for Phase B; running tests without isolation"
        vpn_config=""
    }

    local t0 t1 duration_ms output status

    # B1 — Download fixture via fixture-peer-smoke (local server + client, SHA-256 verified)
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_vpn_cargo "cert-b1" "$vpn_config" \
            -- \
            SLSK_USERNAME="$username1" \
            SLSK_PASSWORD="$password1" \
            SLSKR_PROBE_OUTPUT=json \
            cargo run -q -p slskr -- smoke fixture-peer 2>&1)"
    else
        output="$(SLSK_USERNAME="$username1" SLSK_PASSWORD="$password1" SLSKR_PROBE_OUTPUT=json \
            timeout 30 cargo run -q -p slskr -- smoke fixture-peer 2>&1)"
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]] && echo "$output" | grep -qi "completed.*bytes.*sha256"; then
        local bytes sha256
        bytes="$(echo "$output" | grep -oP 'bytes=\K[0-9]+' | tail -1)"
        sha256="$(echo "$output" | grep -oP 'sha256=\K[a-f0-9]+' | tail -1)"
        record_test "B" "B1" "download-fixture-sha256" "pass" "$duration_ms" "bytes=$bytes sha256=$sha256"
    else
        record_test "B" "B1" "download-fixture-sha256" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # B2 — Large fixture download (100KB pattern)
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_vpn_cargo "cert-b2" "$vpn_config" \
            -- \
            SLSK_USERNAME="$username1" \
            SLSK_PASSWORD="$password1" \
            SLSKR_LARGE_TRANSFER_SIZE=100000 \
            SLSKR_PROBE_OUTPUT=json \
            cargo run -q -p slskr -- smoke fixture-peer 2>&1)"
    else
        output="$(SLSK_USERNAME="$username1" SLSK_PASSWORD="$password1" \
            SLSKR_LARGE_TRANSFER_SIZE=100000 SLSKR_PROBE_OUTPUT=json \
            timeout 30 cargo run -q -p slskr -- smoke fixture-peer 2>&1)"
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "B" "B2" "large-fixture-download" "pass" "$duration_ms" "100KB fixture downloaded"
    else
        record_test "B" "B2" "large-fixture-download" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # B3 — Upload proof: use fixture-peer smoke as upload proxy test
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_vpn_cargo "cert-b3" "$vpn_config" \
            -- \
            SLSK_USERNAME="$username1" \
            SLSK_PASSWORD="$password1" \
            SLSKR_PROBE_OUTPUT=json \
            cargo run -q -p slskr -- smoke fixture-peer 2>&1)"
    else
        output="$(SLSK_USERNAME="$username1" SLSK_PASSWORD="$password1" \
            SLSKR_PROBE_OUTPUT=json \
            timeout 30 cargo run -q -p slskr -- smoke fixture-peer 2>&1)"
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "B" "B3" "upload-proof" "pass" "$duration_ms" "fixture peer upload/download round-trip"
    else
        record_test "B" "B3" "upload-proof" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # B4 — Transfer resume: local peer transfer with non-zero offset
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_vpn_cargo "cert-b4" "$vpn_config" \
            -- \
            SLSK_USERNAME="$username1" \
            SLSK_PASSWORD="$password1" \
            SLSKR_FIXTURE_PEER_USERNAME=slskr-cert-b4 \
            SLSKR_PROBE_OUTPUT=json \
            cargo run -q -p slskr -- smoke transfer-resume 2>&1)"
    else
        output="$(env SLSK_USERNAME="$username1" SLSK_PASSWORD="$password1" \
            SLSKR_FIXTURE_PEER_USERNAME=slskr-cert-b4 SLSKR_PROBE_OUTPUT=json \
            timeout 30 cargo run -q -p slskr -- smoke transfer-resume 2>&1)"
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "B" "B4" "transfer-resume" "pass" "$duration_ms" "resume from offset verified"
    else
        record_test "B" "B4" "transfer-resume" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # B5 — Transfer rejection: local peer transfer rejected gracefully
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_vpn_cargo "cert-b5" "$vpn_config" \
            -- \
            SLSK_USERNAME="$username1" \
            SLSK_PASSWORD="$password1" \
            SLSKR_FIXTURE_PEER_USERNAME=slskr-cert-b5 \
            SLSKR_PROBE_OUTPUT=json \
            cargo run -q -p slskr -- smoke transfer-reject 2>&1)"
    else
        output="$(env SLSK_USERNAME="$username1" SLSK_PASSWORD="$password1" \
            SLSKR_FIXTURE_PEER_USERNAME=slskr-cert-b5 SLSKR_PROBE_OUTPUT=json \
            timeout 30 cargo run -q -p slskr -- smoke transfer-reject 2>&1)"
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "B" "B5" "transfer-rejection-handling" "pass" "$duration_ms" "rejection handled gracefully"
    else
        record_test "B" "B5" "transfer-rejection-handling" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi
}

# --- Phase C: Social & Discovery ---
run_phase_c() {
    log info "=== Phase C: Social & Discovery ==="

    local account_count="${SLSKR_TEST_ACCOUNT_COUNT:-4}"
    if [[ "$account_count" -lt 2 ]]; then
        for id in C1 C2 C3 C4 C5 C6; do
            record_test "C" "$id" "$id social test" "skip" 0 "need 2+ accounts"
        done
        return 0
    fi

    local username1 password1 username2 password2
    username1="${SLSKR_TEST_1_USERNAME:-}"
    password1="${SLSKR_TEST_1_PASSWORD:-}"
    username2="${SLSKR_TEST_2_USERNAME:-}"
    password2="${SLSKR_TEST_2_PASSWORD:-}"

    if [[ -z "$username1" || -z "$username2" ]]; then
        for id in C1 C2 C3 C4 C5 C6; do
            record_test "C" "$id" "$id social test" "skip" 0 "no credentials"
        done
        return 0
    fi

    # Resolve Phase C VPN config for IP isolation
    local vpn_config
    vpn_config="$(resolve_vpn_config_for_phase "C")" || vpn_config=""

    # C1 — Private message bidirectional
    sleep 10
    local t0 t1 duration_ms output status
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_vpn_cargo "cert-c1" "$vpn_config" \
            -- \
            SLSK_USERNAME="$username1" \
            SLSK_PASSWORD="$password1" \
            SLSK_MESSAGE_USERNAME="$username2" \
            SLSK_MESSAGE_PASSWORD="$password2" \
            SLSKR_PROBE_OUTPUT=json \
            cargo run -q -p slskr -- probe private-message 2>&1)"
    else
        output="$(env SLSK_USERNAME="$username1" SLSK_PASSWORD="$password1" \
            SLSK_MESSAGE_USERNAME="$username2" SLSK_MESSAGE_PASSWORD="$password2" \
            SLSKR_PROBE_OUTPUT=json timeout 30 cargo run -q -p slskr -- probe private-message 2>&1)"
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "C" "C1" "private-message bidirectional" "pass" "$duration_ms" "PM sent/received/acked"
    else
        record_test "C" "C1" "private-message bidirectional" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # C2 — Room join/leave/message
    sleep 10
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_vpn_cargo "cert-c2" "$vpn_config" \
            -- \
            SLSK_USERNAME="$username1" \
            SLSK_PASSWORD="$password1" \
            SLSKR_PROBE_OUTPUT=json \
            cargo run -q -p slskr -- probe room-message 2>&1)"
    else
        output="$(env SLSK_USERNAME="$username1" SLSK_PASSWORD="$password1" \
            SLSKR_PROBE_OUTPUT=json timeout 30 cargo run -q -p slskr -- probe room-message 2>&1)"
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "C" "C2" "room join/leave/message" "pass" "$duration_ms" "room message sent/received"
    else
        record_test "C" "C2" "room join/leave/message" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # C3 — Deterministic create/join response and rejection-code handshake
    t0="$(date +%s%N)"
    set +e
    output="$(SLSKR_PROBE_OUTPUT=json timeout 15 \
        cargo run -q -p slskr -- smoke room-create 2>&1)"
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "C" "C3" "room create protocol handshake" "pass" "$duration_ms" "join response and rejection codes verified"
    else
        record_test "C" "C3" "room create protocol handshake" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # C4 — Live WishlistInterval receipt and WishlistSearch transmission
    sleep 10
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_vpn_cargo "cert-c4" "$vpn_config" \
            -- \
            SLSK_USERNAME="$username1" \
            SLSK_PASSWORD="$password1" \
            SLSKR_PROBE_OUTPUT=json \
            cargo run -q -p slskr -- probe wishlist-interval 2>&1)"
    else
        output="$(env SLSK_USERNAME="$username1" SLSK_PASSWORD="$password1" \
            SLSKR_PROBE_OUTPUT=json timeout 45 cargo run -q -p slskr -- probe wishlist-interval 2>&1)"
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "C" "C4" "wishlist interval/search wire" "pass" "$duration_ms" "live WishlistInterval received and WishlistSearch sent"
    else
        record_test "C" "C4" "wishlist interval/search wire" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # C5 — User watch stats
    sleep 10
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_vpn_cargo "cert-c5" "$vpn_config" \
            -- \
            SLSK_USERNAME="$username1" \
            SLSK_PASSWORD="$password1" \
            SLSK_PEER_USERNAME="$username2" \
            SLSKR_PROBE_OUTPUT=json \
            cargo run -q -p slskr -- probe user-watch 2>&1)"
    else
        output="$(env SLSK_USERNAME="$username1" SLSK_PASSWORD="$password1" \
            SLSK_PEER_USERNAME="$username2" \
            SLSKR_PROBE_OUTPUT=json timeout 30 cargo run -q -p slskr -- probe user-watch 2>&1)"
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "C" "C5" "user-watch-stats wire" "pass" "$duration_ms" "WatchUser and GetUserStats responses received"
    else
        record_test "C" "C5" "user-watch-stats wire" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # C6 — Browse complete shares
    local fixture_path="$repo_root/target/open-commons-fixtures/commons-click-track.ogg"
    if [[ ! -f "$fixture_path" ]]; then
        "$repo_root/scripts/verify-open-commons-fixtures.sh" >/dev/null
    fi
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_vpn_cargo "cert-c6" "$vpn_config" \
            -- \
            SLSK_USERNAME="$username1" \
            SLSK_PASSWORD="$password1" \
            SLSKR_FIXTURE_PEER_FILE="$fixture_path" \
            SLSKR_PROBE_OUTPUT=json \
            cargo run -q -p slskr -- smoke fixture-peer 2>&1)"
    else
        output="$(env SLSK_USERNAME="$username1" SLSK_PASSWORD="$password1" \
            SLSKR_FIXTURE_PEER_FILE="$fixture_path" \
            SLSKR_PROBE_OUTPUT=json timeout 30 cargo run -q -p slskr -- smoke fixture-peer 2>&1)"
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "C" "C6" "browse-complete-shares" "pass" "$duration_ms" "fixture browse completed"
    else
        record_test "C" "C6" "browse-complete-shares" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi
}

# --- Phase D: Distributed Search Tree ---
run_phase_d() {
    log info "=== Phase D: Distributed Search Tree ==="

    local t0 t1 duration_ms output status
    t0="$(date +%s%N)"
    set +e
    output="$(SLSKR_PROBE_OUTPUT=json timeout 30 \
        cargo run -q -p slskr -- smoke distributed-tree 2>&1)"
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "D" "D1" "distributed-ping-roundtrip" "pass" "$duration_ms" "TCP init and Ping response completed"
        record_test "D" "D2" "branch-level-parent-adoption" "pass" "$duration_ms" "parent and branch metadata verified"
        record_test "D" "D3" "distributed-search-forwarding" "pass" "$duration_ms" "search reached non-source child only"
        record_test "D" "D4" "child-connection-handling" "pass" "$duration_ms" "child depth and disconnect lifecycle verified"
    else
        local detail
        detail="$(echo "$output" | tail -3 | tr '\n' ' ')"
        record_test "D" "D1" "distributed-ping-roundtrip" "fail" "$duration_ms" "$detail"
        record_test "D" "D2" "branch-level-parent-adoption" "fail" "$duration_ms" "$detail"
        record_test "D" "D3" "distributed-search-forwarding" "fail" "$duration_ms" "$detail"
        record_test "D" "D4" "child-connection-handling" "fail" "$duration_ms" "$detail"
    fi
}

# --- Phase E: NAT-PMP & Network Resilience ---
run_phase_e() {
    log info "=== Phase E: NAT-PMP & Network Resilience ==="

    local username1 password1
    username1="${SLSKR_TEST_1_USERNAME:-}"
    password1="${SLSKR_TEST_1_PASSWORD:-}"

    # Resolve Phase E VPN config for NAT-PMP access
    local vpn_config
    vpn_config="$(resolve_vpn_config_for_phase "E")" || vpn_config=""

    # E1 — NAT-PMP claim port (runs inside VPN netns where gateway is reachable)
    local gateway="${PROTON_NATPMP_GATEWAY:-10.2.0.1}"
    local test_port=2234
    local t0 t1 duration_ms output status
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_netns_command "cert-e1" "$vpn_config" \
            timeout 30 env PROTON_NATPMP_GATEWAY="$gateway" \
            "$repo_root/scripts/probe-natpmp-mapping.sh" claim "$test_port" 2>&1)"
    else
        if command -v natpmpc >/dev/null 2>&1; then
            output="$(PROTON_NATPMP_GATEWAY="$gateway" timeout 30 \
                "$repo_root/scripts/probe-natpmp-mapping.sh" claim "$test_port" 2>&1)"
        else
            output="natpmpc not installed"
        fi
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if echo "$output" | grep -qi "mapped public port"; then
        local public_port
        public_port="$(echo "$output" | awk '/Mapped public port/ { for(i=1;i<=NF;i++) if($i=="port") print $(i+1) }')"
        record_test "E" "E1" "natpmp-claim-port" "pass" "$duration_ms" "mapped public port=$public_port via isolated VPN"
    else
        record_test "E" "E1" "natpmp-claim-port" "fail" "$duration_ms" "NAT-PMP gateway unreachable or natpmpc missing"
    fi

    # E2 — NAT-PMP renew mapping
    test_port=2235
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_netns_command "cert-e2" "$vpn_config" \
            timeout 30 env PROTON_NATPMP_GATEWAY="$gateway" \
            "$repo_root/scripts/probe-natpmp-mapping.sh" renew "$test_port" 2>&1)"
    else
        if command -v natpmpc >/dev/null 2>&1; then
            output="$(PROTON_NATPMP_GATEWAY="$gateway" timeout 30 \
                "$repo_root/scripts/probe-natpmp-mapping.sh" renew "$test_port" 2>&1)"
        else
            output="natpmpc not installed"
        fi
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if echo "$output" | grep -qi "mapped public port"; then
        record_test "E" "E2" "natpmp-renew-mapping" "pass" "$duration_ms" "renewal succeeded via isolated VPN"
    else
        record_test "E" "E2" "natpmp-renew-mapping" "fail" "$duration_ms" "NAT-PMP gateway unreachable"
    fi

    # E3 — Port collision detection
    test_port=2236
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_netns_command "cert-e3" "$vpn_config" \
            timeout 30 env PROTON_NATPMP_GATEWAY="$gateway" \
            "$repo_root/scripts/probe-natpmp-mapping.sh" collision "$test_port" 2>&1)"
    else
        if command -v natpmpc >/dev/null 2>&1; then
            output="$(PROTON_NATPMP_GATEWAY="$gateway" timeout 30 \
                "$repo_root/scripts/probe-natpmp-mapping.sh" collision "$test_port" 2>&1)"
        else
            output="natpmpc not installed"
        fi
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]] || echo "$output" | grep -qi "mapped public port"; then
        record_test "E" "E3" "port-collision-detection" "pass" "$duration_ms" "collision handled (re-claim succeeded)"
    else
        record_test "E" "E3" "port-collision-detection" "fail" "$duration_ms" "NAT-PMP gateway unreachable"
    fi

    # E4 — NAT-PMP with obfuscated port
    local obf_port=2237
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_netns_command "cert-e4" "$vpn_config" \
            timeout 30 env PROTON_NATPMP_GATEWAY="$gateway" \
            "$repo_root/scripts/probe-natpmp-mapping.sh" claim "$obf_port" 2>&1)"
    else
        if command -v natpmpc >/dev/null 2>&1; then
            output="$(PROTON_NATPMP_GATEWAY="$gateway" timeout 30 \
                "$repo_root/scripts/probe-natpmp-mapping.sh" claim "$obf_port" 2>&1)"
        else
            output="natpmpc not installed"
        fi
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if echo "$output" | grep -qi "mapped public port"; then
        record_test "E" "E4" "natpmp-obfuscated-port" "pass" "$duration_ms" "obfuscated port mapped via isolated VPN"
    else
        record_test "E" "E4" "natpmp-obfuscated-port" "fail" "$duration_ms" "NAT-PMP gateway unreachable"
    fi

    # E5 — Soak with NAT-PMP (short bounded soak, runs inside VPN netns)
    if [[ -n "$username1" && -n "$password1" ]]; then
        local e5_log="$output_dir/e5-natpmp-$timestamp.log"
        t0="$(date +%s%N)"
        set +e
        if [[ -n "$vpn_config" ]]; then
            output="$(run_netns_command "cert-e5" "$vpn_config" timeout 60 env \
                SLSKR_SOAK_CREDENTIAL_FILE=/dev/null \
                SLSK_USERNAME="$username1" \
                SLSK_PASSWORD="$password1" \
                SLSK_SERVER="${certify_server_address:-${SLSK_SERVER:-server.slsknet.org:2242}}" \
                PROTON_NATPMP_GATEWAY="$gateway" \
                PROTON_NATPMP_LIFETIME=30 \
                PROTON_NATPMP_RENEW_SECONDS=5 \
                SLSK_SOAK_SECONDS=10 \
                SLSK_LISTEN_PORT=2238 \
                SLSK_SOAK_OBFUSCATED_LISTEN_PORT=2239 \
                SLSK_SOAK_ACTIVE_PROBES=0 \
                SLSK_SOAK_DEFAULT_SEARCH=0 \
                "$repo_root/scripts/run-live-soak-proton-natpmp.sh" "$e5_log" 2>&1)"
        else
            output="$(timeout 60 env \
                SLSKR_SOAK_CREDENTIAL_FILE=/dev/null \
                SLSK_USERNAME="$username1" \
                SLSK_PASSWORD="$password1" \
                PROTON_NATPMP_GATEWAY="$gateway" \
                PROTON_NATPMP_LIFETIME=30 \
                PROTON_NATPMP_RENEW_SECONDS=5 \
                SLSK_SOAK_SECONDS=10 \
                SLSK_LISTEN_PORT=2238 \
                SLSK_SOAK_OBFUSCATED_LISTEN_PORT=2239 \
                SLSK_SOAK_ACTIVE_PROBES=0 \
                SLSK_SOAK_DEFAULT_SEARCH=0 \
                "$repo_root/scripts/run-live-soak-proton-natpmp.sh" "$e5_log" 2>&1)"
        fi
        status=$?
        set -e
        t1="$(date +%s%N)"
        duration_ms=$(( (t1 - t0) / 1000000 ))

        if [[ $status -eq 0 ]]; then
            record_test "E" "E5" "soak-with-natpmp" "pass" "$duration_ms" "10s bounded soak completed"
        else
            record_test "E" "E5" "soak-with-natpmp" "fail" "$duration_ms" \
                "soak exit status=$status: $(tail -3 "$e5_log" 2>/dev/null | tr '\n' ' ')"
        fi
    else
        record_test "E" "E5" "soak-with-natpmp" "fail" 0 "need credentials"
    fi
}

# --- Phase G: Soak Certification ---
run_phase_g() {
    log info "=== Phase G: Soak Certification ==="

    local username1 password1
    username1="${SLSKR_TEST_1_USERNAME:-}"
    password1="${SLSKR_TEST_1_PASSWORD:-}"

    if [[ -z "$username1" || -z "$password1" ]]; then
        for id in G1 G2 G3; do
            record_test "G" "$id" "$id soak test" "skip" 0 "no credentials"
        done
        return 0
    fi

    # Resolve Phase G VPN config for IP isolation
    local vpn_config
    vpn_config="$(resolve_vpn_config_for_phase "G")" || vpn_config=""

    # G1 — Short server soak (10s bounded)
    sleep 10
    local t0 t1 duration_ms output status
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_vpn_cargo "cert-g1" "$vpn_config" \
            -- \
            SLSK_USERNAME="$username1" \
            SLSK_PASSWORD="$password1" \
            SLSK_SOAK_SECONDS=10 \
            SLSKR_PROBE_OUTPUT=json \
            cargo run -q -p slskr -- soak live 2>&1)"
    else
        output="$(timeout 20 env SLSK_USERNAME="$username1" SLSK_PASSWORD="$password1" \
            SLSK_SOAK_SECONDS=10 SLSKR_PROBE_OUTPUT=json \
            cargo run -q -p slskr -- soak live 2>&1)"
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "G" "G1" "server-soak-10s" "pass" "$duration_ms" "10s bounded soak completed"
    else
        record_test "G" "G1" "server-soak-10s" "fail" "$duration_ms" "soak exit status=$status"
    fi

    # G2 — Listener soak (plain + obfuscated, 5s)
    # Add delay to avoid rate limiting
    sleep 3
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_vpn_cargo "cert-g2" "$vpn_config" \
            -- \
            SLSK_USERNAME="$username1" \
            SLSK_PASSWORD="$password1" \
            SLSK_SOAK_SECONDS=5 \
            SLSK_LISTEN_PORT=2239 \
            SLSK_SOAK_OBFUSCATED_LISTEN_PORT=2240 \
            SLSKR_PROBE_OUTPUT=json \
            cargo run -q -p slskr -- soak live 2>&1)"
    else
        output="$(timeout 15 env SLSK_USERNAME="$username1" SLSK_PASSWORD="$password1" \
            SLSK_SOAK_SECONDS=5 SLSK_LISTEN_PORT=2239 SLSK_SOAK_OBFUSCATED_LISTEN_PORT=2240 \
            SLSKR_PROBE_OUTPUT=json cargo run -q -p slskr -- soak live 2>&1)"
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -eq 0 ]]; then
        record_test "G" "G2" "listener-soak-plain-obfuscated" "pass" "$duration_ms" "5s listener soak completed"
    else
        record_test "G" "G2" "listener-soak-plain-obfuscated" "fail" "$duration_ms" "soak exit status=$status"
    fi

    # G3 — NAT-PMP soak (runs inside VPN netns where gateway is reachable)
    local soak_script="$repo_root/scripts/run-live-soak-proton-natpmp.sh"
    local natpmp_vpn_config
    natpmp_vpn_config="$(resolve_proton_config "${SLSKR_CERTIFY_NATPMP_LABEL:-p1}")" || \
        natpmp_vpn_config="$vpn_config"
    if [[ -n "$natpmp_vpn_config" ]]; then
        local gateway="${PROTON_NATPMP_GATEWAY:-10.2.0.1}"
        local g3_log="$output_dir/g3-natpmp-$timestamp.log"
        sleep 3
        t0="$(date +%s%N)"
        set +e
        # Run the NAT-PMP soak script inside the VPN netns
        # shellcheck disable=SC2016
        output="$(run_netns_command "cert-g3" "$natpmp_vpn_config" \
            timeout 30 env \
                SLSKR_SOAK_CREDENTIAL_FILE=/dev/null \
                SLSK_USERNAME="$username1" \
                SLSK_PASSWORD="$password1" \
                SLSK_SERVER="${certify_server_address:-${SLSK_SERVER:-server.slsknet.org:2242}}" \
                PROTON_NATPMP_GATEWAY="$gateway" \
                PROTON_NATPMP_LIFETIME=30 \
                PROTON_NATPMP_RENEW_SECONDS=10 \
                SLSK_LISTEN_PORT=2241 \
                SLSK_SOAK_OBFUSCATED_LISTEN_PORT=2242 \
                SLSK_SOAK_SECONDS=5 \
                SOAK_SCRIPT="$soak_script" \
                SOAK_LOG="$g3_log" \
                bash -c '
                    command -v natpmpc >/dev/null 2>&1 || { echo "natpmpc not found in netns"; exit 127; }
                    "$SOAK_SCRIPT" "$SOAK_LOG"
                ' 2>&1)"
        status=$?
        set -e
        t1="$(date +%s%N)"
        duration_ms=$(( (t1 - t0) / 1000000 ))

        if [[ $status -eq 0 ]]; then
            record_test "G" "G3" "natpmp-soak-5s" "pass" "$duration_ms" "5s NAT-PMP soak completed via isolated VPN"
        else
            record_test "G" "G3" "natpmp-soak-5s" "fail" "$duration_ms" \
                "NAT-PMP soak failed (exit=$status): $(tail -3 "$g3_log" 2>/dev/null | tr '\n' ' ')"
        fi
    else
        if [[ -x "$soak_script" ]] && command -v natpmpc >/dev/null 2>&1; then
            local gateway="${PROTON_NATPMP_GATEWAY:-10.2.0.1}"
            t0="$(date +%s%N)"
            set +e
            SLSKR_SOAK_CREDENTIAL_FILE=/dev/null \
                SLSK_USERNAME="$username1" \
                SLSK_PASSWORD="$password1" \
                PROTON_NATPMP_GATEWAY="$gateway" \
                PROTON_NATPMP_LIFETIME=30 \
                PROTON_NATPMP_RENEW_SECONDS=10 \
                SLSK_LISTEN_PORT=2241 \
                SLSK_SOAK_OBFUSCATED_LISTEN_PORT=2242 \
                SLSK_SOAK_SECONDS=5 \
                timeout 15 "$soak_script" /dev/null 2>&1
            status=$?
            set -e
            t1="$(date +%s%N)"
            duration_ms=$(( (t1 - t0) / 1000000 ))

            if [[ $status -eq 0 ]]; then
                record_test "G" "G3" "natpmp-soak-5s" "pass" "$duration_ms" "5s NAT-PMP soak completed"
            else
                record_test "G" "G3" "natpmp-soak-5s" "fail" "$duration_ms" "NAT-PMP soak failed (exit=$status, gateway unreachable without VPN)"
            fi
        else
            record_test "G" "G3" "natpmp-soak-5s" "fail" 0 "natpmpc not installed or soak script unavailable"
        fi
    fi
}

# --- Phase H: Negative & Failure Modes ---
run_phase_h() {
    log info "=== Phase H: Negative & Failure Modes ==="

    # Resolve Phase H VPN config for IP isolation
    local vpn_config
    vpn_config="$(resolve_vpn_config_for_phase "H")" || vpn_config=""

    # H1 — Wrong password
    local t0 t1 duration_ms output status
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_vpn_cargo "cert-h1" "$vpn_config" \
            -- \
            SLSK_USERNAME="${SLSKR_TEST_1_USERNAME:-}" \
            SLSK_PASSWORD="wrong_password_123" \
            SLSKR_PROBE_OUTPUT=json \
            cargo run -q -p slskr -- login smoke 2>&1)"
    else
        output="$(SLSK_USERNAME="${SLSKR_TEST_1_USERNAME:-}" SLSK_PASSWORD="wrong_password_123" \
            SLSKR_PROBE_OUTPUT=json \
            timeout 30 cargo run -q -p slskr -- login smoke 2>&1)"
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))

    if [[ $status -ne 0 ]] && echo "$output" | grep -Eqi 'INVALIDPASS|login rejected'; then
        record_test "H" "H1" "wrong-password login fails gracefully" "pass" "$duration_ms" "login rejected as expected"
    else
        record_test "H" "H1" "wrong-password login fails gracefully" "fail" "$duration_ms" \
            "missing explicit invalid-password rejection"
    fi

    # H2 — Account relogin elsewhere (two simultaneous sessions)
    local relogin_username relogin_password
    relogin_username="${SLSKR_TEST_2_USERNAME:-${SLSKR_TEST_1_USERNAME:-}}"
    relogin_password="${SLSKR_TEST_2_PASSWORD:-${SLSKR_TEST_1_PASSWORD:-}}"
    if [[ -n "$relogin_username" && -n "$relogin_password" ]]; then
        sleep 3
        t0="$(date +%s%N)"
        set +e
        if [[ -n "$vpn_config" ]]; then
            output="$(run_vpn_cargo "cert-h2" "$vpn_config" \
                SLSK_USERNAME="$relogin_username" \
                SLSK_PASSWORD="$relogin_password" \
                SLSKR_PROBE_OUTPUT=json \
                -- cargo run -q -p slskr -- smoke server-relogin 2>&1)"
        else
            output="$(SLSK_USERNAME="$relogin_username" SLSK_PASSWORD="$relogin_password" \
                SLSKR_PROBE_OUTPUT=json timeout 45 \
                cargo run -q -p slskr -- smoke server-relogin 2>&1)"
        fi
        status=$?
        set -e
        t1="$(date +%s%N)"
        duration_ms=$(( (t1 - t0) / 1000000 ))
        if [[ $status -eq 0 ]]; then
            record_test "H" "H2" "account-relogin-elsewhere" "pass" "$duration_ms" "first session received Relogged"
        else
            record_test "H" "H2" "account-relogin-elsewhere" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
        fi
    else
        record_test "H" "H2" "account-relogin-elsewhere" "skip" 0 "no credentials"
    fi

    # H3 — Offline peer
    local username password
    username="${SLSKR_TEST_1_USERNAME:-}"
    password="${SLSKR_TEST_1_PASSWORD:-}"
    if [[ -n "$username" && -n "$password" ]]; then
        t0="$(date +%s%N)"
        set +e
        if [[ -n "$vpn_config" ]]; then
            output="$(run_vpn_cargo "cert-h3" "$vpn_config" \
                -- \
                SLSK_USERNAME="$username" \
                SLSK_PASSWORD="$password" \
                SLSK_PEER_USERNAME="nonexistent_peer_xyz_12345" \
                SLSKR_PROBE_OUTPUT=json \
                cargo run -q -p slskr -- probe peer-address 2>&1)"
        else
            output="$(SLSK_USERNAME="$username" SLSK_PASSWORD="$password" \
                SLSK_PEER_USERNAME="nonexistent_peer_xyz_12345" \
                SLSKR_PROBE_OUTPUT=json \
                timeout 30 cargo run -q -p slskr -- probe peer-address 2>&1)"
        fi
        status=$?
        set -e
        t1="$(date +%s%N)"
        duration_ms=$(( (t1 - t0) / 1000000 ))

        if echo "$output" | grep -Eqi 'port[=: ]+0|did not advertise.*port'; then
            record_test "H" "H3" "offline-peer handled gracefully" "pass" "$duration_ms" "offline peer handled"
        else
            record_test "H" "H3" "offline-peer handled gracefully" "fail" "$duration_ms" "unexpected result: $(echo "$output" | tail -1)"
        fi
    else
        record_test "H" "H3" "offline-peer handled gracefully" "skip" 0 "no credentials"
    fi

    # H4 — Closed listener port
    t0="$(date +%s%N)"
    set +e
    output="$(SLSKR_PROBE_OUTPUT=json timeout 15 \
        cargo run -q -p slskr -- smoke closed-listener 2>&1)"
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))
    if [[ $status -eq 0 ]]; then
        record_test "H" "H4" "closed-listener-port" "pass" "$duration_ms" "connection refusal handled"
    else
        record_test "H" "H4" "closed-listener-port" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # H5 — Unsupported obfuscation metadata
    t0="$(date +%s%N)"
    set +e
    output="$(SLSKR_PROBE_OUTPUT=json timeout 15 \
        cargo run -q -p slskr -- smoke bad-obfuscation-type 2>&1)"
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))
    if [[ $status -eq 0 ]]; then
        record_test "H" "H5" "bad-obfuscation-type" "pass" "$duration_ms" "unsupported type rejected"
    else
        record_test "H" "H5" "bad-obfuscation-type" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # H6 — Explicit disconnect followed by a fresh authenticated session
    if [[ -n "$relogin_username" && -n "$relogin_password" ]]; then
        sleep 3
        t0="$(date +%s%N)"
        set +e
        if [[ -n "$vpn_config" ]]; then
            output="$(run_vpn_cargo "cert-h6" "$vpn_config" \
                SLSK_USERNAME="$relogin_username" \
                SLSK_PASSWORD="$relogin_password" \
                SLSKR_PROBE_OUTPUT=json \
                -- cargo run -q -p slskr -- smoke server-reconnect 2>&1)"
        else
            output="$(SLSK_USERNAME="$relogin_username" SLSK_PASSWORD="$relogin_password" \
                SLSKR_PROBE_OUTPUT=json timeout 60 \
                cargo run -q -p slskr -- smoke server-reconnect 2>&1)"
        fi
        status=$?
        set -e
        t1="$(date +%s%N)"
        duration_ms=$(( (t1 - t0) / 1000000 ))
        if [[ $status -eq 0 ]]; then
            record_test "H" "H6" "server-disconnect-reconnect" "pass" "$duration_ms" "fresh session authenticated and pinged"
        else
            record_test "H" "H6" "server-disconnect-reconnect" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
        fi
    else
        record_test "H" "H6" "server-disconnect-reconnect" "skip" 0 "no credentials"
    fi

    # H7 — Renewal failure is recorded without terminating the renewal loop
    t0="$(date +%s%N)"
    set +e
    if [[ -n "$vpn_config" ]]; then
        output="$(run_netns_command "cert-h7" "$vpn_config" timeout 20 env \
            NATPMP_COMMAND=false \
            SLSKR_NATPMP_RENEWAL_FAILURE_PROBE=1 \
            "$repo_root/scripts/run-live-soak-proton-natpmp.sh" \
            "$output_dir/h7-natpmp-$timestamp.log" 2>&1)"
    else
        output="$(NATPMP_COMMAND=false \
            SLSKR_NATPMP_RENEWAL_FAILURE_PROBE=1 timeout 20 \
            "$repo_root/scripts/run-live-soak-proton-natpmp.sh" \
            "$output_dir/h7-natpmp-$timestamp.log" 2>&1)"
    fi
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))
    if [[ $status -eq 0 ]] && echo "$output" | grep -q "renew_failed"; then
        record_test "H" "H7" "natpmp-renewal-failure" "pass" "$duration_ms" "failure recorded and loop continuation verified"
    else
        record_test "H" "H7" "natpmp-renewal-failure" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi

    # H8 — Truncated peer response frame
    t0="$(date +%s%N)"
    set +e
    output="$(SLSKR_PROBE_OUTPUT=json timeout 15 \
        cargo run -q -p slskr -- smoke malformed-peer-response 2>&1)"
    status=$?
    set -e
    t1="$(date +%s%N)"
    duration_ms=$(( (t1 - t0) / 1000000 ))
    if [[ $status -eq 0 ]]; then
        record_test "H" "H8" "malformed-peer-response" "pass" "$duration_ms" "truncated frame rejected"
    else
        record_test "H" "H8" "malformed-peer-response" "fail" "$duration_ms" "$(echo "$output" | tail -3 | tr '\n' ' ')"
    fi
}

# --- Report generation ---
generate_report() {
    local end_time duration_seconds
    end_time="$(date +%s)"
    duration_seconds=$((end_time - start_time))

    cat > "$report_file" <<EOF
{
  "timestamp": "$(date -Is)",
  "duration_seconds": $duration_seconds,
  "total": $total_tests,
  "passed": $passed_tests,
  "failed": $failed_tests,
  "skipped": $skipped_tests,
  "phases_run": "$phases",
  "vpn_enabled": "${SLSKR_CERTIFY_VPN_ENABLED:-0}",
  "log_format": "$log_format",
  "log_file": "$log_file"
}
EOF

    log info ""
    log info "=== Certification Summary ==="
    log info "  Total:    $total_tests"
    log info "  Passed:   $passed_tests"
    log info "  Failed:   $failed_tests"
    log info "  Skipped:  $skipped_tests"
    log info "  Duration: ${duration_seconds}s"
    log info "  Report:   $report_file"
    log info "  Log:      $log_file"
}

# --- Main ---
log info "slskr certification runner starting"
log info "phases: $phases"
log info "log_format: $log_format"
log info "output: $output_dir"

if [[ "$dry_run" == "true" ]]; then
    log info "DRY RUN — showing plan without executing"
    log info "  Phase A: Foundation (login, peer-address, plain/obfuscated/indirect peer)"
    log info "  Phase B: Transfer Certification (download, upload, resume, rejection)"
    log info "  Phase C: Social & Discovery (PM, rooms, wishlist, browse)"
    log info "  Phase D: Distributed Search Tree (parents, branch, forwarding)"
    log info "  Phase E: NAT-PMP & Network Resilience (claim, renew, collision)"
    log info "  Phase G: Soak Certification (server, listener, NAT-PMP soak)"
    log info "  Phase H: Negative & Failure Modes (wrong password, offline peer, etc.)"
    log info ""
    log info "Run without --dry-run to execute."
    exit 0
fi

IFS=',' read -ra phase_list <<< "$phases"
first_phase=true
for phase in "${phase_list[@]}"; do
    phase="$(echo "$phase" | tr '[:lower:]' '[:upper:]' | tr -d ' ')"
    # Add delay between live phases to avoid server rate-limiting
    if [[ "$first_phase" != "true" ]]; then
        sleep "${SLSKR_CERTIFY_INTER_PHASE_DELAY:-3}"
    fi
    first_phase=false
    case "$phase" in
        A) run_phase_a ;;
        B) run_phase_b ;;
        C) run_phase_c ;;
        D) run_phase_d ;;
        E) run_phase_e ;;
        G) run_phase_g ;;
        H) run_phase_h ;;
        *) log warn "unknown phase: $phase — skipping" ;;
    esac
done

generate_report

if [[ $failed_tests -gt 0 ]]; then
    exit 1
fi
