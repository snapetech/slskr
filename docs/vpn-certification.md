# VPN-Isolated Certification

This document describes the per-account VPN isolation architecture used by the
`slskr` certification runner to bypass the Soulseek server's per-IP rate
limiting.

## Problem

The Soulseek server aggressively rate-limits rapid logins from the same public
IP. When running certification tests with multiple accounts or repeating test
phases, connections fail with:

```
I/O error: Connection reset by peer (os error 104)
I/O error: unexpected end of file
```

Fixed delays between tests help but are insufficient — the server tracks
per-IP concurrent login attempts, not just request frequency.

## Solution: Per-Account VPN Isolation

Each test account is routed through its own isolated Proton WireGuard network
namespace. The Soulseek server sees a different source IP per account,
completely bypassing the per-IP rate limiter.

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Host System                          │
│                                                             │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌──────────┐ │
│  │ Account 1 │  │ Account 2 │  │ Account 3 │  │ Account 4│ │
│  │   (p1)    │  │   (p3)    │  │   (p4)    │  │   (p5)   │ │
│  │  netns    │  │  netns    │  │  netns    │  │  netns   │ │
│  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └────┬─────┘ │
│        │              │              │              │       │
│   ┌────┴────┐    ┌────┴────┐    ┌────┴────┐    ┌───┴─────┐  │
│   │ WG p1   │    │ WG p3   │    │ WG p4   │    │ WG p5   │  │
│   │169.x.x.x│    │79.x.x.x │    │79.x.x.x │    │89.x.x.x │  │
│   └────┬────┘    └────┬────┘    └────┬────┘    └────┬────┘  │
│        │              │              │              │       │
│   ┌────┴──────────────┴──────────────┴──────────────┴────┐  │
│   │                  Internet / Soulseek                  │  │
│   └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Components

| Component | File | Purpose |
| --- | --- | --- |
| **Certification runner** | `scripts/run-certification.sh` | Orchestrates test phases, spawns netns per test |
| **Netns runner** | `scripts/run-in-proton-wg-netns.sh` | Creates network namespace, sets up WireGuard, runs command |
| **Proton configs** | `.secrets/proton-slskr-{1..8}.conf` | 8 WireGuard configs for different Proton exit nodes |
| **Credential pool** | `.secrets/proton-credential-pool.env` | Maps labels (p1–p8) to config file paths |
| **Account env** | `.env` | Test account credentials (SLSKR_TEST_{1..N}_USERNAME/PASSWORD) |

### Account-to-VPN Mapping

The runner maps each test account to a dedicated Proton exit node:

| Account | VPN Label | Config File |
| --- | --- | --- |
| Test 1 | p1 | `proton-slskr-1.conf` |
| Test 2 | p3 | `proton-slskr-3.conf` |
| Test 3 | p4 | `proton-slskr-4.conf` |
| Test 4 | p5 | `proton-slskr-5.conf` |
| Test 5 | p6 | `proton-slskr-6.conf` |
| Test 6 | p7 | `proton-slskr-7.conf` |
| Test 7 | p8 | `proton-slskr-8.conf` |
| Test 8 | p2 | `proton-slskr-2.conf` |

Non-account tests (transfers, social, soak, etc.) use phase-specific VPN labels
to maintain IP isolation without reusing account IPs.

### How It Works

1. **Netns creation**: `run-in-proton-wg-netns.sh` creates a network namespace
   with a veth pair for host-to-namespace routing.
2. **WireGuard setup**: The Proton config is loaded inside the namespace with
   its own `wg0` interface.
3. **Split routing**: Default route goes through `wg0`; the Proton endpoint IP
   is routed via the host's default gateway to prevent routing loops.
4. **Command execution**: The `cargo run` command executes inside the namespace
   with test credentials as environment variables.
5. **Cleanup**: On exit, the namespace, routes, and iptables rules are removed.

## Setup

### Prerequisites

- **WireGuard tools**: `wg-quick` or `wireguard-tools`
- **Root/sudo**: Network namespace creation requires elevated privileges
- **Proton configs**: 8 WireGuard configs in `.secrets/`
- **Test accounts**: At least 2 Soulseek accounts in `.env`

### Credential Setup

```bash
# .env
SLSKR_TEST_1_USERNAME=user1
SLSKR_TEST_1_PASSWORD=pass1
SLSKR_TEST_2_USERNAME=user2
SLSKR_TEST_2_PASSWORD=pass2
SLSKR_TEST_3_USERNAME=user3
SLSKR_TEST_3_PASSWORD=pass3
SLSKR_TEST_4_USERNAME=user4
SLSKR_TEST_4_PASSWORD=pass4

SLSKR_TEST_ACCOUNT_COUNT=4
```

### Proton Config Setup

Place WireGuard configs in `.secrets/`:

```
.secrets/
├── proton-slskr-1.conf
├── proton-slskr-2.conf
├── ...
├── proton-slskr-8.conf
└── proton-credential-pool.env
```

The credential pool file maps labels to config paths:

```bash
# .secrets/proton-credential-pool.env
SLSKR_PROTON_CONFIG_LABELS="p1 p2 p3 p4 p5 p6 p7 p8"
SLSKR_PROTON_CONFIG_p1=".secrets/proton-slskr-1.conf"
SLSKR_PROTON_CONFIG_p2=".secrets/proton-slskr-2.conf"
# ... etc
```

## Running Certification

### Full Run

```bash
scripts/run-certification.sh
```

The runner auto-detects VPN configs and enables isolation automatically.

### Specific Phases

```bash
# Login + transfers only
scripts/run-certification.sh --phases A,B

# Negative tests only
scripts/run-certification.sh --phases H
```

### JSON Output

```bash
scripts/run-certification.sh --log-format json
```

### Dry Run

```bash
scripts/run-certification.sh --dry-run
```

### Configuration Variables

| Variable | Default | Purpose |
| --- | --- | --- |
| `SLSKR_CERTIFY_VPN_ENABLED` | `auto` | Set to `1` to force VPN, `0` to disable, or leave as `auto` for detection |
| `SLSKR_TEST_ACCOUNT_COUNT` | `4` | Number of test accounts to use |
| `SLSKR_LOGIN_DELAY` | `5` | Seconds between login attempts |
| `SLSKR_CERTIFY_INTER_PHASE_DELAY` | `10` | Seconds between test phases |
| `SLSKR_CERTIFY_OUTPUT_DIR` | `target/certify` | Output directory for logs and reports |

## Troubleshooting

### "failed to lookup address information"

DNS resolution failed inside the namespace. Check that the netns `resolv.conf`
is set correctly — the runner uses the gateway IP (`10.2.0.1` by default).

### "Connection reset by peer"

This is the rate-limiting symptom that VPN isolation is designed to fix.
Ensure:
1. VPN configs are present and valid
2. `SLSKR_CERTIFY_VPN_ENABLED` is `1` or `auto` with detected configs
3. Each account has a unique VPN label assigned

### "natpmpc not found in netns"

`natpmpc` must be available on the host. The netns inherits the host's PATH but
may not have all binaries. Install it:

```bash
# Debian/Ubuntu
sudo apt install libnatpmp5 natpmpc

# Arch
sudo pacman -S libnatpmp
```

### Slow WireGuard handshake

Proton servers may take 2–5 seconds to complete the initial handshake. The
netns runner includes a `SLSKR_NETNS_WG_SETTLE_SECONDS` (default 2s) to wait
before running commands. Increase if needed:

```bash
SLSKR_NETNS_WG_SETTLE_SECONDS=5 scripts/run-certification.sh --phases A
```

### Namespace cleanup failures

If the runner exits abnormally, namespaces may be left behind:

```bash
# List remaining namespaces
ip netns list

# Clean up
sudo ip netns del cert-a1-1 2>/dev/null || true
sudo ip netns del cert-a1-2 2>/dev/null || true
# ... etc
```

## Results

A successful full certification run produces:

```
=== Certification Summary ===
  Total:    39
  Passed:   39
  Failed:   0
  Skipped:  0
  Duration: 442s
  Report:   target/certify/summary-20260716-103416.json
  Log:      target/certify/certify-20260716-103416.log
```

Reports and logs are saved to `target/certify/` for each run.

## See Also

- [full-network-test-plan.md](./full-network-test-plan.md) — Full test plan and pass criteria
- [live-interop-test-matrix.md](./live-interop-test-matrix.md) — Live network verification matrix
- [scripts/run-certification.sh](../scripts/run-certification.sh) — Certification runner source
- [scripts/run-in-proton-wg-netns.sh](../scripts/run-in-proton-wg-netns.sh) — Netns runner source
