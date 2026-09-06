$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.39/slskr-v0.2.39-x86_64-pc-windows-msvc.zip"
$checksum = "743cddd3ed93e5eded7db1c4a8ee67309ef421575680362c90b3796e8f9150ec"

Install-ChocolateyZipPackage -PackageName 'slskr' -Url $url -UnzipLocation $toolsDir -Checksum $checksum -ChecksumType 'sha256'
