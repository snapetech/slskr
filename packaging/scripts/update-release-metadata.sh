#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"
. packaging/scripts/release-assets.sh

tag="${1:?usage: update-release-metadata.sh <release-tag> <asset-dir>}"
asset_dir="${2:?usage: update-release-metadata.sh <release-tag> <asset-dir>}"
release_version="$(slskr_release_version_from_tag "$tag")"
pkgver="${release_version#v}"

asset_path() {
  printf '%s/%s\n' "$asset_dir" "$1"
}

sha_for() {
  slskr_sha256 "$(asset_path "$1")"
}

linux_x64="slskr-${release_version}-x86_64-unknown-linux-gnu.tar.gz"
linux_arm64="slskr-${release_version}-aarch64-unknown-linux-gnu.tar.gz"
linux_musl_x64="slskr-${release_version}-x86_64-unknown-linux-musl.tar.gz"
linux_musl_arm64="slskr-${release_version}-aarch64-unknown-linux-musl.tar.gz"
mac_x64="slskr-${release_version}-x86_64-apple-darwin.tar.gz"
mac_arm64="slskr-${release_version}-aarch64-apple-darwin.tar.gz"
win_x64="slskr-${release_version}-x86_64-pc-windows-msvc.zip"

for file in "$linux_x64" "$linux_arm64" "$linux_musl_x64" "$linux_musl_arm64" "$mac_x64" "$mac_arm64" "$win_x64"; do
  test -f "$(asset_path "$file")" || { echo "missing release asset: $file" >&2; exit 1; }
done

linux_x64_sha="$(sha_for "$linux_x64")"
linux_arm64_sha="$(sha_for "$linux_arm64")"
mac_x64_sha="$(sha_for "$mac_x64")"
mac_arm64_sha="$(sha_for "$mac_arm64")"
win_x64_sha="$(sha_for "$win_x64")"

svc_sha="$(slskr_sha256 packaging/aur/slskr.service)"
sys_sha="$(slskr_sha256 packaging/aur/slskr.sysusers)"
tmp_sha="$(slskr_sha256 packaging/aur/slskr.tmpfiles)"

sed -i \
  -e "s/^pkgver=.*/pkgver=${pkgver}/" \
  -e "s/^sha256sums=.*/sha256sums=('${svc_sha}' '${sys_sha}' '${tmp_sha}')/" \
  -e "s/^sha256sums_x86_64=.*/sha256sums_x86_64=('${linux_x64_sha}')/" \
  -e "s/^sha256sums_aarch64=.*/sha256sums_aarch64=('${linux_arm64_sha}')/" \
  packaging/aur/PKGBUILD-bin

sed -i \
  -e "s/^pkgver=.*/pkgver=${pkgver}/" \
  -e "s/^sha256sums=.*/sha256sums=('SKIP' '${svc_sha}' '${sys_sha}' '${tmp_sha}')/" \
  packaging/aur/PKGBUILD

sed -i \
  -e "s/^Version:.*/Version:        ${pkgver}/" \
  -e "s|^Source0:.*|Source0:        ${linux_x64}|" \
  -e "s|^Source1:.*|Source1:        ${linux_arm64}|" \
  packaging/rpm/slskr.spec

cat > packaging/debian/changelog <<EOF
slskr (${pkgver}-1) unstable; urgency=medium

  * Release ${tag}.

 -- slskr maintainers <slskr@proton.me>  $(date -R)
EOF

python3 - "$tag" "$release_version" "$pkgver" "$linux_x64_sha" "$linux_arm64_sha" "$mac_x64_sha" "$mac_arm64_sha" "$win_x64_sha" <<'PY'
import pathlib
import re
import sys

tag, rel, pkgver, linux_x64, linux_arm64, mac_x64, mac_arm64, win_x64 = sys.argv[1:]

def write(path, text):
    pathlib.Path(path).write_text(text, encoding="utf-8")

write("packaging/winget/snapetech.slskr.yaml", f"""# yaml-language-server: $schema=https://aka.ms/winget-manifest.version.1.6.0.schema.json
PackageIdentifier: snapetech.slskr
PackageVersion: {pkgver}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.6.0
""")

write("packaging/winget/snapetech.slskr.locale.en-US.yaml", f"""# yaml-language-server: $schema=https://aka.ms/winget-manifest.defaultLocale.1.6.0.schema.json
PackageIdentifier: snapetech.slskr
PackageVersion: {pkgver}
PackageLocale: en-US
Publisher: snapetech
PublisherUrl: https://github.com/snapetech
PublisherSupportUrl: https://github.com/snapetech/slskr/issues
PackageName: slskr
PackageUrl: https://github.com/snapetech/slskr
License: AGPL-3.0-only
LicenseUrl: https://github.com/snapetech/slskr/blob/main/LICENSE
ShortDescription: Rust Soulseek daemon with bundled Web UI
Description: slskr is a Rust Soulseek daemon with an HTTP API, transfers, search, observability, and a bundled Web UI.
Moniker: slskr
Tags:
  - soulseek
  - slsk
  - daemon
  - webui
  - rust
ReleaseNotesUrl: https://github.com/snapetech/slskr/releases/tag/{tag}
ManifestType: defaultLocale
ManifestVersion: 1.6.0
""")

write("packaging/winget/snapetech.slskr.installer.yaml", f"""# yaml-language-server: $schema=https://aka.ms/winget-manifest.installer.1.6.0.schema.json
PackageIdentifier: snapetech.slskr
PackageVersion: {pkgver}
InstallerType: zip
NestedInstallerType: portable
NestedInstallerFiles:
  - RelativeFilePath: slskr-{rel}-x86_64-pc-windows-msvc\\slskr.exe
    PortableCommandAlias: slskr
Installers:
  - Architecture: x64
    InstallerUrl: https://github.com/snapetech/slskr/releases/download/{tag}/slskr-{rel}-x86_64-pc-windows-msvc.zip
    InstallerSha256: {win_x64.upper()}
ManifestType: installer
ManifestVersion: 1.6.0
""")

write("packaging/homebrew/Formula/slskr.rb", f'''class Slskr < Formula
  desc "Rust Soulseek daemon with bundled Web UI"
  homepage "https://github.com/snapetech/slskr"
  license "AGPL-3.0-only"
  version "{pkgver}"

  on_macos do
    on_arm do
      url "https://github.com/snapetech/slskr/releases/download/{tag}/slskr-{rel}-aarch64-apple-darwin.tar.gz"
      sha256 "{mac_arm64}"
    end
    on_intel do
      url "https://github.com/snapetech/slskr/releases/download/{tag}/slskr-{rel}-x86_64-apple-darwin.tar.gz"
      sha256 "{mac_x64}"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/snapetech/slskr/releases/download/{tag}/slskr-{rel}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "{linux_arm64}"
    else
      url "https://github.com/snapetech/slskr/releases/download/{tag}/slskr-{rel}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "{linux_x64}"
    end
  end

  def install
    libexec.install Dir["*"]
    bin.install libexec/"slskr"
  end

  test do
    assert_match "slskr", shell_output("#{{bin}}/slskr version")
  end
end
''')

write("packaging/snap/snapcraft.yaml", f'''name: slskr
base: core22
version: '{pkgver}'
summary: Rust Soulseek daemon with bundled Web UI
description: |
  slskR is a Rust Soulseek client and daemon with a local web interface.
grade: stable
confinement: strict
architectures:
  - build-on: [amd64]
    build-for: [amd64]
  - build-on: [amd64, arm64]
    build-for: [arm64]

apps:
  slskr:
    command: slskr
    daemon: simple
    plugs: [network, network-bind, home, removable-media]
    environment:
      SLSKR_CONFIG: $SNAP_USER_COMMON/config.toml

parts:
  slskr:
    plugin: dump
    source:
      - on amd64: https://github.com/snapetech/slskr/releases/download/{tag}/slskr-{rel}-x86_64-unknown-linux-gnu.tar.gz
      - on arm64: https://github.com/snapetech/slskr/releases/download/{tag}/slskr-{rel}-aarch64-unknown-linux-gnu.tar.gz
    source-type: file
    override-pull: |
      craftctl default
      case "${{CRAFT_ARCH_BUILD_FOR:-${{SNAPCRAFT_ARCH_BUILD_FOR:-}}}}" in
        amd64)
          archive="slskr-{rel}-x86_64-unknown-linux-gnu.tar.gz"
          expected="{linux_x64}"
          ;;
        arm64)
          archive="slskr-{rel}-aarch64-unknown-linux-gnu.tar.gz"
          expected="{linux_arm64}"
          ;;
        *)
          echo "unsupported Snap architecture: ${{CRAFT_ARCH_BUILD_FOR:-${{SNAPCRAFT_ARCH_BUILD_FOR:-}}}}" >&2
          exit 1
          ;;
      esac
      archive_path="$CRAFT_PART_SRC/$archive"
      test -f "$archive_path"
      printf '%s  %s\\n' "$expected" "$archive_path" | sha256sum --check --status
      tar -xzf "$archive_path" -C "$CRAFT_PART_SRC" --strip-components=1
      rm -f "$archive_path"
''')

write("packaging/flatpak/io.github.slskd.slskr.yml", f'''app-id: io.github.slskd.slskr
runtime: org.freedesktop.Platform
runtime-version: '23.08'
sdk: org.freedesktop.Sdk
command: slskr-wrapper

finish-args:
  - --share=network
  - --filesystem=xdg-download
  - --filesystem=xdg-music
  - --filesystem=~/.config/slskr:create
  - --filesystem=~/.local/share/slskr:create

modules:
  - name: slskr
    buildsystem: simple
    build-commands:
      - mkdir -p /app/bin /app/lib/slskr
      - cp -r . /app/lib/slskr/
      - |
        cat > /app/bin/slskr-wrapper <<'EOF'
        #!/usr/bin/env bash
        set -e
        CONFIG_DIR="${{XDG_CONFIG_HOME:-$HOME}}/slskr"
        mkdir -p "$CONFIG_DIR"
        if [ ! -f "$CONFIG_DIR/config.toml" ]; then
          printf '%s\\n' '[web]' 'port = 5030' > "$CONFIG_DIR/config.toml"
        fi
        exec /app/lib/slskr/slskr serve --config "$CONFIG_DIR/config.toml"
        EOF
      - chmod +x /app/bin/slskr-wrapper
    sources:
      - type: archive
        url: https://github.com/snapetech/slskr/releases/download/{tag}/slskr-{rel}-x86_64-unknown-linux-gnu.tar.gz
        sha256: {linux_x64}
        only-arches: [x86_64]
      - type: archive
        url: https://github.com/snapetech/slskr/releases/download/{tag}/slskr-{rel}-aarch64-unknown-linux-gnu.tar.gz
        sha256: {linux_arm64}
        only-arches: [aarch64]
''')
PY

sed -i \
  -e "s|<version>[^<]*</version>|<version>${pkgver}</version>|" \
  packaging/chocolatey/slskr.nuspec
sed -i \
  -e "s|^\$url = .*|\$url = \"https://github.com/snapetech/slskr/releases/download/${tag}/${win_x64}\"|" \
  -e "s|^\$checksum = .*|\$checksum = \"${win_x64_sha}\"|" \
  packaging/chocolatey/tools/chocolateyinstall.ps1

for chart in \
  packaging/helm/slskr/Chart.yaml \
  packaging/truenas-scale/charts/slskr/Chart.yaml; do
  sed -i "s|^version: .*|version: ${pkgver}|" "$chart"
  sed -i "s|^appVersion: .*|appVersion: \"${pkgver}\"|" "$chart"
done
for values in \
  packaging/helm/slskr/values.yaml \
  packaging/truenas-scale/charts/slskr/values.yaml; do
  sed -i "s|^  tag: .*|  tag: \"${pkgver}\"|" "$values"
done

sed -i "s|^  <Repository>.*</Repository>|  <Repository>ghcr.io/snapetech/slskr:${pkgver}</Repository>|" \
  packaging/unraid/slskr.xml
sed -i "s|^version=\".*\"|version=\"${pkgver}\"|" \
  packaging/synology-spk/INFO
sed -i "s|^VERSION=\"\${SLSKR_VERSION:-.*}\"|VERSION=\"\${SLSKR_VERSION:-${pkgver}}\"|" \
  packaging/synology-spk/build-spk.sh

sed -i \
  -e "s|^        version = .*|        version = \"${pkgver}\";|" \
  -e "/\"x86_64-linux\" = {/,/^[[:space:]]*};/ { s|^            url = .*|            url = \"https://github.com/snapetech/slskr/releases/download/${tag}/${linux_x64}\";|; s|^            sha256 = .*|            sha256 = \"${linux_x64_sha}\";|; }" \
  -e "/\"aarch64-linux\" = {/,/^[[:space:]]*};/ { s|^            url = .*|            url = \"https://github.com/snapetech/slskr/releases/download/${tag}/${linux_arm64}\";|; s|^            sha256 = .*|            sha256 = \"${linux_arm64_sha}\";|; }" \
  -e "/\"x86_64-darwin\" = {/,/^[[:space:]]*};/ { s|^            url = .*|            url = \"https://github.com/snapetech/slskr/releases/download/${tag}/${mac_x64}\";|; s|^            sha256 = .*|            sha256 = \"${mac_x64_sha}\";|; }" \
  -e "/\"aarch64-darwin\" = {/,/^[[:space:]]*};/ { s|^            url = .*|            url = \"https://github.com/snapetech/slskr/releases/download/${tag}/${mac_arm64}\";|; s|^            sha256 = .*|            sha256 = \"${mac_arm64_sha}\";|; }" \
  flake.nix

if grep -R "CHANGE_ME" \
  packaging/aur \
  packaging/homebrew \
  packaging/winget \
  packaging/rpm \
  packaging/debian; then
  echo "release metadata still contains CHANGE_ME placeholders" >&2
  exit 1
fi
