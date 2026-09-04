$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.38/slskr-v0.2.38-x86_64-pc-windows-msvc.zip"
$checksum = "3eeb8914f664070921b484ebb5318ddaf10bcd28e2e48126c5f639d70cc163af"

Install-ChocolateyZipPackage -PackageName 'slskr' -Url $url -UnzipLocation $toolsDir -Checksum $checksum -ChecksumType 'sha256'
