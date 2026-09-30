# Local live interop matrix — 2026-09-27

Source commit: `f3b1412e2ec18d5a0f6dbc79b51370f59bc0c8aa`.

The current debug daemon was rebuilt with `cargo build -p slskr`, then
`scripts/run-live-interop-matrix.sh` exited successfully. Four account login
probes, one direct/obfuscated/indirect local-peer probe, and two private/room
message probes all reported `ok`.

The raw TSVs and runner log remain in the ignored local directory
`target/live-interop-rf-20260927/`. The TSV hashes identify this run without
publishing test-account names:

| File | SHA-256 |
| --- | --- |
| `slskr-login-smoke.tsv` | `27e990066a2cfcec1f4661ebac9b77722580bb6c6d441a36f40b78f4b2488e77` |
| `slskr-local-peer-smoke.tsv` | `ed3fe44c7d309e6537797a4c6e478bb188cd51fdd7c2c8e2bb2fd989e1773045` |
| `slskr-social-smoke.tsv` | `3365c7ee753f1b46f133617b62db5a308cad5c54b32b55d00450298cbe0ef5ee` |

This run did not exercise shutdown overlap, optional media, browser parity, or
the scheduled compatibility smoke. It is local evidence; hosted artifact links
remain open.
