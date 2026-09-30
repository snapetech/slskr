$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.41/slskr-v0.2.41-x86_64-pc-windows-msvc.zip"
$checksum = "1a2b858149bb03016132ee3b58ac9131c63144e19aa394e092f40a59efb2729c"

Install-ChocolateyZipPackage -PackageName 'slskr' -Url $url -UnzipLocation $toolsDir -Checksum $checksum -ChecksumType 'sha256'
