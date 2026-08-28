# Rust Dependency Hygiene

Date: 2026-07-15

`cargo tree -d -p slskr` is the canonical local duplicate-family report for the release binary. Current duplicate roots are allowed only while they are transitive across upstream dependency families:

| Duplicate Root | Current Source | Status |
| --- | --- | --- |
| `base64` | `reqwest`/`sqlx` remain on `0.22`, while direct slskr and slskr-client usage is on `0.23`. | Tracked transitive duplicate. |
| `cpufeatures` | `sha1` uses `0.2`, while `sha2 0.11` uses `0.3`. | Tracked transitive duplicate. |
| `crypto-common` | `digest 0.10` uses `0.1`, while `digest 0.11` uses `0.2`. | Tracked transitive duplicate. |
| `getrandom` | `ring` uses `0.2`, `rand`/`tungstenite` use `0.3`, and `uuid` uses `0.4`. | Tracked transitive duplicate. |
| `hashbrown` | `sqlx`/`hashlink` use `0.15`, while `indexmap`/`toml_edit` use `0.17`. | Tracked transitive duplicate. |
| `memchr` | The release dependency graph resolves one `2.8` package through both host/proc-macro and target runtime paths. | Reviewed same-version graph duplicate. |
| `rand` | Direct workspace randomness uses `0.10`, while `tungstenite` and dev-only `proptest` remain on `0.9`. | Tracked transitive duplicate until upstreams converge. |
| `rand_core` | Direct `rand 0.10` uses `rand_core 0.10`, while `tungstenite`/`proptest` keep `rand_core 0.9`. | Tracked transitive duplicate until upstreams converge. |
| `regex-automata` | The release dependency graph resolves one `0.4` package through `regex`, `fancy-regex`, `matchers`, and host/proc-macro paths. | Reviewed same-version graph duplicate. |
| `syn` | Most proc-macro dependencies use `2.0`, while the current serde/thiserror family also resolves `3.0`. | Tracked transitive duplicate. |
| `webpki-roots` | `hyper-rustls`/reqwest use `1.0`, while tokio-tungstenite retains `0.26`. | Tracked transitive duplicate. |

Policy:

- New duplicate roots must be reviewed before release.
- Direct dependencies should not be added at older major versions when a workspace-compatible newer family is already present.
- `scripts/check-rust-dependency-hygiene.sh` fails if `cargo tree -d -p slskr` reports duplicate roots outside the table above.
