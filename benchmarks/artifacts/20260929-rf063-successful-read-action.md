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
passes in 7.13 seconds. A current-source hosted Live Parity browser rerun is
pending; this record does not claim that rerun passed.
