$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.52/slskr-v0.2.52-x86_64-pc-windows-msvc.zip"
$checksum = "4bf61431449f79f19bd816ee71fd16b69b55f5f68e9db3fc891c54bb853634d5"

Install-ChocolateyZipPackage -PackageName 'slskr' -Url $url -UnzipLocation $toolsDir -Checksum $checksum -ChecksumType 'sha256'
