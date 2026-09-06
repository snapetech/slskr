$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.40/slskr-v0.2.40-x86_64-pc-windows-msvc.zip"
$checksum = "46bf8eae60d9e231ce959344848c1c32fd618e6af7703f554074e329d0853f3c"

Install-ChocolateyZipPackage -PackageName 'slskr' -Url $url -UnzipLocation $toolsDir -Checksum $checksum -ChecksumType 'sha256'
