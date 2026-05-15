# Rust Dependency Hygiene

Date: 2026-05-15

`cargo tree -d -p slskr` is the canonical local duplicate-family report for the release binary. Current duplicate roots are allowed only while they are transitive across upstream dependency families:

| Duplicate Root | Current Source | Status |
| --- | --- | --- |
| `block-buffer` | `digest 0.10` consumers (`ed25519-dalek`/`tungstenite`) use `0.10`, while direct `hmac`/`sha2 0.11` use `0.12`. | Tracked transitive duplicate. |
| `cpufeatures` | `curve25519-dalek`/`sha1`/`sha2 0.10` use `0.2`, while `sha2 0.11` uses `0.3`. | Tracked transitive duplicate. |
| `crypto-common` | `digest 0.10` uses `0.1`, while `digest 0.11` uses `0.2`. | Tracked transitive duplicate. |
| `digest` | `ed25519-dalek`/`tungstenite` remain on `0.10`, while direct `hmac`/`sha2` usage is on `0.11`. | Tracked transitive duplicate. |
| `getrandom` | `ring` uses `0.2`, `rand`/`tungstenite` use `0.3`, and `uuid` uses `0.4`. | Tracked transitive duplicate. |
| `hashbrown` | `sqlx`/`hashlink` use `0.15`, while `indexmap`/`toml_edit` use `0.17`. | Tracked transitive duplicate. |
| `sha2` | `ed25519-dalek` remains on `0.10`, while direct workspace hashing/signing uses `0.11`. | Tracked transitive duplicate. |

Policy:

- New duplicate roots must be reviewed before release.
- Direct dependencies should not be added at older major versions when a workspace-compatible newer family is already present.
- `scripts/check-rust-dependency-hygiene.sh` fails if `cargo tree -d -p slskr` reports duplicate roots outside the table above.
