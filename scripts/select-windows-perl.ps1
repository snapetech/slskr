$perlCandidates = @(
  Get-Command perl.exe -All -ErrorAction SilentlyContinue |
    ForEach-Object Source
  'C:\Strawberry\perl\bin\perl.exe'
) | Select-Object -Unique

$perl = $null
foreach ($candidate in $perlCandidates) {
  if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
    continue
  }

  & $candidate -MLocale::Maketext::Simple -e 1
  if ($LASTEXITCODE -eq 0) {
    $perl = $candidate
    break
  }
}

if (-not $perl) {
  throw 'No Perl installation with Locale::Maketext::Simple is available for vendored OpenSSL'
}

"OPENSSL_SRC_PERL=$perl" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
