# Release Channels

slskR publishes downstream packages from `.github/workflows/release-publish.yml`
after a GitHub Release is published. The workflow reuses the same account layout
as the slskdN release system, with slskR-specific package and project names.

| Channel | Target | Credentials |
| --- | --- | --- |
| GitHub Container Registry | `ghcr.io/snapetech/slskr` | Built-in `GITHUB_TOKEN` |
| Docker Hub | `snapetech/slskr` | `DOCKERHUB_USERNAME`, `DOCKERHUB_TOKEN` |
| AUR source package | `slskr` | `AUR_SSH_KEY` |
| AUR binary package | `slskr-bin` | `AUR_SSH_KEY` |
| COPR | `slskdn/slskr` | Preferred: `COPR_KERBEROS_PRINCIPAL`, `COPR_KERBEROS_KEYTAB_B64`; fallback: `COPR_LOGIN`, `COPR_TOKEN` |
| Launchpad PPA | `~keefshape/ubuntu/slskr` | `GPG_PRIVATE_KEY`, optional `LAUNCHPAD_SFTP_KEY`, optional `LAUNCHPAD_SFTP_USER` |
| Homebrew tap | `snapetech/homebrew-slskr` | `TAP_GITHUB_TOKEN` |
| Winget | `snapetech.slskr` | `WINGETCREATE_GITHUB_TOKEN` |

The release workflow skips credentialed channels whose secrets are not available,
except GHCR, which publishes with the repository token.

## COPR Authentication

COPR publishing prefers Fedora Kerberos/GSSAPI because Copr API tokens expire.
Store a keytab-backed principal in OpenBao, then let `github:sync-secrets`
propagate it to GitHub Actions:

```bash
scripts/store-copr-kerberos-openbao.sh \
  --principal '<principal>@FEDORAPROJECT.ORG' \
  --keytab /path/to/copr.keytab
```

Keep `COPR_LOGIN` and `COPR_TOKEN` configured until the Kerberos path has
completed a release-publish run. If Kerberos secrets are absent, the workflow
continues to use the token fallback.

## Dynamic Hashes

Do not hand-edit release checksums in package metadata. The release workflow
downloads the release assets for the selected tag and runs:

```bash
packaging/scripts/update-release-metadata.sh <release-tag> <asset-dir>
```

That script recalculates package metadata from the actual release assets and
updates AUR, Winget, Homebrew, RPM, and Debian metadata before each downstream
publish job runs.
