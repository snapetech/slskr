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
install_sha="$(slskr_sha256 packaging/aur/slskr.install)"

sed -i \
  -e "s/^pkgver=.*/pkgver=${pkgver}/" \
  -e "s/^sha256sums=.*/sha256sums=('${svc_sha}' '${sys_sha}' '${tmp_sha}' '${install_sha}')/" \
  -e "s/^sha256sums_x86_64=.*/sha256sums_x86_64=('${linux_x64_sha}')/" \
  -e "s/^sha256sums_aarch64=.*/sha256sums_aarch64=('${linux_arm64_sha}')/" \
  packaging/aur/PKGBUILD-bin

sed -i \
  -e "s/^pkgver=.*/pkgver=${pkgver}/" \
  -e "s/^sha256sums=.*/sha256sums=('SKIP' '${svc_sha}' '${sys_sha}' '${tmp_sha}' '${install_sha}')/" \
  packaging/aur/PKGBUILD

sed -i \
  -e "s/^Version:.*/Version:        ${pkgver}/" \
  -e "s|^Source0:.*|Source0:        ${linux_x64}|" \
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
PY

if grep -R "CHANGE_ME" \
  packaging/aur \
  packaging/homebrew \
  packaging/winget \
  packaging/rpm \
  packaging/debian; then
  echo "release metadata still contains CHANGE_ME placeholders" >&2
  exit 1
fi
