# RF-063 successful GET action refresh audit (2026-09-29 UTC)

Internal-only verification record. No release was published.

## Finding

GitHub Live Parity run
[`36561787367`](https://github.com/snapetech/slskr/actions/runs/36561787367)
used source `56e3cd74914c4bc2f58cdea81bd6aa5e1ad8e11f`. The credentialed public
interop job passed. The Rust UI route audit failed only because desktop
`/collections` made 22 requests against the 21-request budget; mobile made 21.

The desktop audit recorded 20 initial requests, then one additional
`GET /api/v0/collections` 725.1 ms after the successful `Open Collection` GET
action. The generic action handler refreshed the full route after every action,
including successful reads whose response was already displayed. This was a
single action-driven repeat above the 200 ms cadence floor, not a polling loop.

## Correction

Successful GET actions now display their response without a route-wide refresh.
Mutating actions and failed requests retain the refresh path. The `/collections`
budget remains 21 rather than being raised to include the redundant read.

`scripts/check-rust-format.sh` passes, and the direct locked release build
`cargo build --locked -p slskr-web --release --target wasm32-unknown-unknown`
passes in 7.13 seconds. The hosted Live Parity rerun was pending when this
correction was first recorded; see the revalidation below.

## Current-source revalidation

Live Parity run
[`36565318189`](https://github.com/snapetech/slskr/actions/runs/36565318189)
on `1283666858fd85e9733f79b4f1daa982a5d5f05a` passed both the Rust UI/API
audit and credentialed public interop jobs. The retained audit JSON records all
30 desktop/mobile route views with zero errors; desktop `/collections` made
21 requests against its 21-request budget. This confirms the corrected action
path no longer performs the extra route refresh.
