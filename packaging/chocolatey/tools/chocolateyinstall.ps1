$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.42/slskr-v0.2.42-x86_64-pc-windows-msvc.zip"
$checksum = "9d254676ed3de8f5dd74e0add0553adcf143735316b6771ffc9e251087e1e340"

Install-ChocolateyZipPackage -PackageName 'slskr' -Url $url -UnzipLocation $toolsDir -Checksum $checksum -ChecksumType 'sha256'
