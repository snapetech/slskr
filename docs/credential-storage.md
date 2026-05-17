# Soulseek Credential Storage

`slskr` supports several Soulseek credential sources so desktop, service,
container, and test deployments can choose the least surprising secret handling
for their environment.

Credential source priority is:

1. Credentials supplied with a Web UI connect request.
2. Configured environment or TOML credentials.
3. Stored credentials loaded from `credential_store`.

The Web UI connect dialog can write only to stores that are writable at runtime:
`memory`, `os`, and `file`. The `systemd` store is intentionally read-only from
inside the daemon; configure it in the service unit.

## Store Modes

| Mode | Best for | Persistence | Notes |
| --- | --- | --- | --- |
| `os` | Desktop/user services | Platform keyring | Default recommendation for interactive installs. Uses the OS credential backend through the `keyring` crate. |
| `systemd` | Linux system services | systemd credential manager | Reads `$CREDENTIALS_DIRECTORY` files created by `LoadCredential=` or `LoadCredentialEncrypted=`. Runtime writes are rejected. |
| `memory` | One-off sessions and testing | Process lifetime only | Enter credentials in the Web UI after each daemon restart. No disk persistence. |
| `file` | Fallback when no keyring exists | Local JSON file | Plaintext file fallback. Keep it on a protected filesystem with restrictive ownership and mode. |
| env/config | Containers and external secret managers | External manager decides | Set `SLSK_USERNAME` and `SLSK_PASSWORD`, or `username`/`password` in TOML loaded from a protected location. |

`SLSKR_CREDENTIAL_STORE` overrides the TOML `credential_store` setting. Valid
values are `os`, `systemd`, `systemd-credentials`, `systemd-creds`, `memory`,
and `file`.

## Recommended Choices

Use `os` for a normal desktop or per-user install. It avoids plaintext config
files and lets the platform keyring own storage policy.

Use `systemd` for a host-level Linux daemon. It keeps the secret outside the
process config, works well with unit hardening, and can use encrypted credential
files.

Use env/config credentials for containers, Kubernetes, or another external
secret manager where the orchestrator already owns secret injection.

Use `memory` when the operator accepts entering credentials after each restart.
This is the least persistent option.

Use `file` only as a compatibility fallback. It is operationally simple, but
the file contains the Soulseek password in plaintext.

## systemd Credentials

Set this in config:

```toml
[network]
credential_store = "systemd"
```

Then provide either split credentials:

```ini
LoadCredentialEncrypted=slsk-username:/etc/credstore.encrypted/slskr-slsk-username.cred
LoadCredentialEncrypted=slsk-password:/etc/credstore.encrypted/slskr-slsk-password.cred
```

or one JSON credential:

```ini
LoadCredentialEncrypted=slskr-soulseek:/etc/credstore.encrypted/slskr-soulseek.cred
```

The JSON payload must be:

```json
{"username":"your-soulseek-username","password":"your-soulseek-password"}
```

At runtime, systemd exposes those files under `$CREDENTIALS_DIRECTORY`. `slskr`
reads `slsk-username` and `slsk-password`, or `slskr-soulseek`, and trims
trailing newlines. It never attempts to write these files.

Create encrypted credentials with systemd:

```sh
printf '%s' 'your-soulseek-username' | sudo systemd-creds encrypt - /etc/credstore.encrypted/slskr-slsk-username.cred
printf '%s' 'your-soulseek-password' | sudo systemd-creds encrypt - /etc/credstore.encrypted/slskr-slsk-password.cred
```

For user services, use `LoadCredential=` or `LoadCredentialEncrypted=` in the
user unit and ensure the configured files are readable to the user manager.

## Web UI Behavior

On first connect, the Web UI prompts for Soulseek username/password when no
usable credentials are configured or stored. The dialog offers only writable
store modes:

- `memory`: keep credentials only until daemon exit.
- `os`: save to the platform credential store.
- `file`: save to the configured restricted local credential file.

When `credential_store = "systemd"`, configure credentials in the unit and
restart the service. The Web UI can still start a connection, but it cannot save
new credentials into systemd.

## Security Notes

- `/api/config`, server state responses, logs, and telemetry must not return raw
  Soulseek passwords.
- Keep config files containing `username` or `password` in protected deployment
  paths with permissions scoped to the service user.
- Prefer `LoadCredentialEncrypted=` over unit-file environment variables for
  Linux system services.
- Treat `file` mode as plaintext-at-rest. Back it with filesystem permissions,
  disk encryption, and a dedicated service user when possible.
