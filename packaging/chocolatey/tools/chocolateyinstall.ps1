$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.15/slskr-v0.2.15-x86_64-pc-windows-msvc.zip"
$checksum = "0000000000000000000000000000000000000000000000000000000000000000"

Install-ChocolateyZipPackage -PackageName 'slskr' -Url $url -UnzipLocation $toolsDir -Checksum $checksum -ChecksumType 'sha256'
