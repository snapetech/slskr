[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $WingetCreatePath,

    [Parameter(Mandatory = $true)]
    [string] $SubmitPath,

    [Parameter(Mandatory = $true)]
    [string] $PackageVersion
)

$ErrorActionPreference = 'Stop'

$token = $env:WINGETCREATE_GITHUB_TOKEN
if ([string]::IsNullOrWhiteSpace($token)) {
    throw 'WINGETCREATE_GITHUB_TOKEN is required to submit the Winget manifest.'
}

if ($PackageVersion -notmatch '^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$') {
    throw "Invalid Winget package version: $PackageVersion"
}

if (-not (Test-Path -LiteralPath $WingetCreatePath -PathType Leaf)) {
    throw "WingetCreate executable not found: $WingetCreatePath"
}

if (-not (Test-Path -LiteralPath $SubmitPath -PathType Container)) {
    throw "Winget manifest directory not found: $SubmitPath"
}

$apiRoot = 'https://api.github.com'
$headers = @{
    Authorization = "Bearer $token"
    Accept = 'application/vnd.github+json'
    'X-GitHub-Api-Version' = '2022-11-28'
    'User-Agent' = 'slskr-release'
}

function Get-HttpStatusCode {
    param(
        [Parameter(Mandatory = $true)]
        [System.Management.Automation.ErrorRecord] $ErrorRecord
    )

    $response = $ErrorRecord.Exception.Response
    if ($null -ne $response) {
        try {
            return [int]$response.StatusCode
        }
        catch {
            return 0
        }
    }

    return 0
}

function Invoke-GitHubApi {
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet('GET', 'POST', 'PATCH')]
        [string] $Method,

        [Parameter(Mandatory = $true)]
        [string] $Uri,

        [Parameter(Mandatory = $false)]
        [AllowNull()]
        [object] $Body
    )

    $request = @{
        Method = $Method
        Uri = $Uri
        Headers = $headers
        ErrorAction = 'Stop'
    }
    if ($PSBoundParameters.ContainsKey('Body')) {
        $request.Body = $Body | ConvertTo-Json -Depth 20 -Compress
        $request.ContentType = 'application/json'
    }

    Invoke-RestMethod @request
}

function Escape-PathSegment {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Value
    )

    [Uri]::EscapeDataString($Value)
}

$user = Invoke-GitHubApi -Method GET -Uri "$apiRoot/user"
$owner = [string]$user.login
if ([string]::IsNullOrWhiteSpace($owner)) {
    throw 'GitHub token did not identify an account.'
}

$upstream = Invoke-GitHubApi -Method GET -Uri "$apiRoot/repos/microsoft/winget-pkgs"
$upstreamBranch = [string]$upstream.default_branch
$upstreamRef = Invoke-GitHubApi -Method GET -Uri "$apiRoot/repos/microsoft/winget-pkgs/git/ref/heads/$(Escape-PathSegment $upstreamBranch)"
$upstreamSha = [string]$upstreamRef.object.sha
if ([string]::IsNullOrWhiteSpace($upstreamSha)) {
    throw "Unable to resolve microsoft/winget-pkgs/$upstreamBranch."
}

$forkUri = "$apiRoot/repos/$owner/winget-pkgs"
$fork = $null
try {
    $fork = Invoke-GitHubApi -Method GET -Uri $forkUri
}
catch {
    if ((Get-HttpStatusCode $_) -ne 404) {
        throw
    }

    Write-Host "Creating $owner/winget-pkgs fork."
    $null = Invoke-GitHubApi -Method POST -Uri "$apiRoot/repos/microsoft/winget-pkgs/forks" -Body @{}
    for ($poll = 1; $poll -le 12; $poll++) {
        Start-Sleep -Seconds 5
        try {
            $fork = Invoke-GitHubApi -Method GET -Uri $forkUri
            break
        }
        catch {
            if ((Get-HttpStatusCode $_) -ne 404 -or $poll -eq 12) {
                throw
            }
        }
    }
}

if ($null -eq $fork) {
    throw "GitHub fork was not available after creation: $forkUri"
}

$forkBranch = [string]$fork.default_branch
if ([string]::IsNullOrWhiteSpace($forkBranch)) {
    throw "GitHub fork has no default branch: $forkUri"
}

$forkBranchPath = Escape-PathSegment $forkBranch
$forkRefUri = "$forkUri/git/ref/heads/$forkBranchPath"
$forkUpdateUri = "$forkUri/git/refs/heads/$forkBranchPath"
$syncUri = "$forkUri/merge-upstream"
try {
    $null = Invoke-GitHubApi -Method POST -Uri $syncUri -Body @{ branch = $forkBranch }
    Write-Host "Synchronized $owner/winget-pkgs/$forkBranch with microsoft/winget-pkgs/$upstreamBranch."
}
catch {
    $status = Get-HttpStatusCode $_
    Write-Warning "Winget fork synchronization failed (HTTP $status); preserving the old default branch and repairing it from upstream."

    $forkRef = Invoke-GitHubApi -Method GET -Uri $forkRefUri
    $forkSha = [string]$forkRef.object.sha
    if ([string]::IsNullOrWhiteSpace($forkSha)) {
        throw "Unable to resolve the current Winget fork default branch: $forkRefUri"
    }

    $runId = $env:GITHUB_RUN_ID
    if ([string]::IsNullOrWhiteSpace($runId)) {
        $runId = [DateTime]::UtcNow.ToString('yyyyMMddHHmmss')
    }
    $runAttempt = $env:GITHUB_RUN_ATTEMPT
    if ([string]::IsNullOrWhiteSpace($runAttempt)) {
        $runAttempt = '1'
    }
    $backupBranch = "slskr-release-backup-$PackageVersion-$runId-$runAttempt"
    $backupRefUri = "$forkUri/git/refs/heads/$(Escape-PathSegment $backupBranch)"

    try {
        $null = Invoke-GitHubApi -Method POST -Uri "$forkUri/git/refs" -Body @{
            ref = "refs/heads/$backupBranch"
            sha = $forkSha
        }
    }
    catch {
        if ((Get-HttpStatusCode $_) -ne 422) {
            throw
        }

        $existingBackup = Invoke-GitHubApi -Method GET -Uri $backupRefUri
        if ([string]$existingBackup.object.sha -ne $forkSha) {
            throw "Backup branch already exists at a different commit: $backupBranch"
        }
    }

    $null = Invoke-GitHubApi -Method PATCH -Uri $forkUpdateUri -Body @{
        sha = $upstreamSha
        force = $true
    }
    Write-Host "Reset $owner/winget-pkgs/$forkBranch to upstream $upstreamSha. Previous default-branch state is preserved as $backupBranch."
}

& $WingetCreatePath submit $SubmitPath -t $token
if ($LASTEXITCODE -ne 0) {
    throw "wingetcreate submit failed with exit code $LASTEXITCODE"
}
