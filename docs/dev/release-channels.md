# Release Channels

slskR publishes downstream packages from `.github/workflows/release-publish.yml`
as a required reusable job after the tag release uploads its assets. GHCR
publication and the published-image shutdown smoke test must pass before the
release workflow completes. The workflow reuses the same account layout as the
slskdN release system, with slskR-specific package and project names.

| Channel | Target | Credentials |
| --- | --- | --- |
| GitHub Container Registry | `ghcr.io/snapetech/slskr` | Built-in `GITHUB_TOKEN` |
| Docker Hub | `snapetech/slskr` | `DOCKERHUB_USERNAME`, `DOCKERHUB_TOKEN` |
| AUR source package | `slskr` | `AUR_SSH_KEY` |
| AUR binary package | `slskr-bin` | `AUR_SSH_KEY` |
| COPR | `slskdn/slskr` (x86_64 and aarch64) | Preferred: `COPR_KERBEROS_PRINCIPAL`, `COPR_KERBEROS_KEYTAB_B64`; fallback: `COPR_LOGIN`, `COPR_TOKEN` |
| Launchpad PPA | `~keefshape/ubuntu/slskr` (amd64 and arm64) | `GPG_PRIVATE_KEY`, optional `LAUNCHPAD_SFTP_KEY`, optional `LAUNCHPAD_SFTP_USER` |
| Homebrew tap | `snapetech/homebrew-slskr` | `TAP_GITHUB_TOKEN` |
| Snap | `slskr` (amd64 and arm64) | Snap Store credentials managed outside this workflow |
| Flatpak | `io.github.slskd.slskr` (x86_64 and aarch64) | Flathub credentials managed outside this workflow |
| Helm | `packaging/helm/slskr` (multi-arch GHCR image) | none |
| TrueNAS SCALE | `packaging/truenas-scale/charts/slskr` (multi-arch GHCR image) | none |
| Unraid | `packaging/unraid/slskr.xml` (multi-arch GHCR image) | none |
| Synology SPK | `packaging/synology-spk` (x86_64 or aarch64 build) | none |
| Winget | `snapetech.slskr` (Windows x64) | `WINGETCREATE_GITHUB_TOKEN` |
| Chocolatey | `slskr` (Windows x64) | `CHOCO_API_KEY` |
| Nix flake | `slskr` | none |

The release workflow skips credentialed channels whose secrets are not available,
except GHCR, which publishes with the repository token. Chocolatey is a
separate manual-dispatch workflow and derives both its package version and
Windows archive checksum from the requested release tag.

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
updates AUR, Winget, Homebrew, RPM, Debian, Chocolatey, Snap, Flatpak, Helm,
TrueNAS, Unraid, Synology, and Nix release metadata before downstream
publishing or manual packaging runs.

The binary release matrix currently covers Linux GNU and musl on x86_64 and
aarch64, macOS on Intel and Apple Silicon, and Windows x64. Windows ARM64 is
not advertised or published until it has a native release runner and package
installer coverage.
