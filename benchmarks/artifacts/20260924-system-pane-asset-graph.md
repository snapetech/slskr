# System pane asset graph (2026-09-24)

The production Web build now defers System pane modules by section. Its
`System` entry chunk fell from 242,641 bytes (236.95 KiB) to 9,386 bytes
(9.17 KiB), a 96.13% reduction. Vite emits six grouped pane chunks for
Overview, Network, Security, Automation, Diagnostics, and Advanced, while
AdminPolicies, Integrations, and MediaCore remain independent dynamic chunks.

The production asset budget passes at 1,084.68 KiB initial JavaScript against
1,150 KiB and 593.07 KiB all-JavaScript gzip against 600 KiB. The emitted
chunk names and byte counts are retained in
[`20260924-system-pane-asset-graph.json`](20260924-system-pane-asset-graph.json).

Evidence was produced by `npm --prefix web run build` and
`npm --prefix web run test:bundle-budget`. This records local asset sizes and
dynamic import boundaries; it is not a browser-transfer-latency measurement.
