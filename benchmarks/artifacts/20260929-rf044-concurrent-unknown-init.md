# RF-044 concurrent unknown-init header bound (2026-09-29 UTC)

Internal-only security test evidence. No runtime behavior changed and no release
was published.

The shared-port listener test holds 64 loopback TCP peers open concurrently:
32 advertise a plain unknown init frame and 32 advertise an obfuscated unknown
init frame. Each header advertises a 16 MiB frame, but sends no body. All 64
connections return `FrameTooLarge` with the 8,205-byte `MAX_PEER_INIT_FRAME_LEN`
bound within the one-second per-connection deadline. The test completes in
0.02 seconds; it does not allocate or transmit 16 MiB bodies.

Validation:

- `scripts/check-rust-format.sh` passes.
- `cargo test --locked -p slskr-client --test listener shared_demux_concurrently_rejects_oversized_unknown_headers_without_body -- --exact` passes (1 test).
- The full current-source GitLab pipeline 148 and GitHub CI run 36566959004 both passed on commit `8265032a`; each runs the workspace client listener tests.

This closes the concurrent header-preflight stress gap. Unknown extension init
frames above 8,205 bytes remain intentionally unsupported before peer
authentication; their third-party interoperability is not claimed.
