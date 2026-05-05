# Rust Dependency Hygiene

Date: 2026-05-05

`cargo tree -d -p slskr` is the canonical local duplicate-family report for the release binary. Current duplicate roots are allowed only while they are transitive across upstream dependency families:

| Duplicate Root | Current Source | Status |
| --- | --- | --- |
| `getrandom` | `rand`/`ring` use `0.2`, while `uuid` uses `0.4`. | Tracked transitive duplicate. |
| `hashbrown` | `sqlx`/`hashlink` use `0.15`, while `indexmap`/`toml_edit` use `0.17`. | Tracked transitive duplicate. |
| `thiserror` | `slskr-client`/`slskr-protocol`/`tungstenite` use `1.x`, while `sqlx` uses `2.x`. | Tracked transitive duplicate. |
| `thiserror-impl` | Proc-macro companion for the tracked `thiserror` duplicate family. | Tracked transitive duplicate. |

Policy:

- New duplicate roots must be reviewed before release.
- Direct dependencies should not be added at older major versions when a workspace-compatible newer family is already present.
- `scripts/check-rust-dependency-hygiene.sh` fails if `cargo tree -d -p slskr` reports duplicate roots outside the table above.
