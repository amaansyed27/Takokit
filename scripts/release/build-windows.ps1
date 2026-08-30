[CmdletBinding()]
param(
    [string]$Version = "0.2.0",
    [string]$OutputRoot,
    [switch]$SkipBuild,
    [switch]$SkipInstaller,
    [switch]$IncludeTestUpdateFixture,
    [switch]$RequireProductionSigning,
    [switch]$AllowDirty
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
if (-not $OutputRoot) {
    $OutputRoot = Join-Path $RepoRoot 'dist\windows'
}
$OutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)
Set-Location $RepoRoot

if (Test-Path -LiteralPath (Join-Path $RepoRoot 'apps\desktop')) {
    throw 'Release gate: apps\desktop must not exist; Takokit GUI remains browser-served and the native application is only its resident controller.'
}

function Invoke-Checked {
    param([Parameter(Mandatory)][string]$FilePath, [Parameter(ValueFromRemainingArguments)][string[]]$Arguments)
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$FilePath failed with exit code $LASTEXITCODE"
    }
}

function Write-Utf8NoBom {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][string]$Text)
    $encoding = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($Path, $Text, $encoding)
}

function Write-Json {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)]$Value)
    $json = $Value | ConvertTo-Json -Depth 12
    Write-Utf8NoBom -Path $Path -Text ($json + "`n")
}

function Get-Sha256 {
    param([Parameter(Mandatory)][string]$Path)
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Get-ArtifactRecord {
    param([string]$Role, [string]$Path)
    $item = Get-Item -LiteralPath $Path
    return [ordered]@{
        role = $Role
        name = $item.Name
        size = [uint64]$item.Length
        sha256 = Get-Sha256 $item.FullName
        url = $null
    }
}

function Set-TreeTimestamp {
    param([string]$Root, [datetime]$Timestamp)
    Get-ChildItem -LiteralPath $Root -Recurse -Force | ForEach-Object {
        $_.LastWriteTimeUtc = $Timestamp
    }
    (Get-Item -LiteralPath $Root).LastWriteTimeUtc = $Timestamp
}

function Copy-DirectoryContents {
    param([string]$Source, [string]$Destination)
    New-Item -ItemType Directory -Force -Path $Destination | Out-Null
    Get-ChildItem -LiteralPath $Source -Force | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $Destination -Recurse -Force
    }
}

function Find-Iscc {
    $command = Get-Command 'ISCC.exe' -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    $candidates = @(
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 7\ISCC.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 7\ISCC.exe'),
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 7\ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe'),
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe')
    )
    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate)) { return $candidate }
    }
    throw 'Inno Setup compiler (ISCC.exe) was not found. Install pinned Inno Setup before building the installer.'
}

if ($Version -ne '0.2.0') {
    throw "The Windows distribution candidate version is locked to 0.2.0; got $Version"
}
Invoke-Checked python 'scripts/check_release_version.py'

$HasProductionSigningKey = -not [string]::IsNullOrWhiteSpace($env:TAKOKIT_RELEASE_SIGNING_KEY_HEX)
$HasProductionPublicKey = -not [string]::IsNullOrWhiteSpace($env:TAKOKIT_RELEASE_PUBLIC_KEY_HEX)
if ($RequireProductionSigning -and (-not $HasProductionSigningKey -or -not $HasProductionPublicKey)) {
    throw 'Production release assembly requires TAKOKIT_RELEASE_SIGNING_KEY_HEX and TAKOKIT_RELEASE_PUBLIC_KEY_HEX.'
}
if ($HasProductionSigningKey -ne $HasProductionPublicKey) {
    throw 'Production signing private and public key material must be configured together.'
}

$CommitSha = (git rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or -not $CommitSha) { throw 'Could not resolve git commit SHA.' }
$CommitTimeText = (git show -s --format=%cI $CommitSha).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Could not resolve git commit timestamp.' }
$CommitTime = [DateTimeOffset]::Parse($CommitTimeText).UtcDateTime
$DirtyLines = @(git status --porcelain --untracked-files=normal)
$IsDirty = $DirtyLines.Count -gt 0
if ($IsDirty -and -not $AllowDirty) {
    throw "Refusing to build release artifacts from a dirty source tree. Commit/stash changes or pass -AllowDirty for an explicitly marked developer artifact."
}

$BuildTimestamp = if ($env:SOURCE_DATE_EPOCH) {
    [DateTimeOffset]::FromUnixTimeSeconds([int64]$env:SOURCE_DATE_EPOCH).UtcDateTime.ToString('yyyy-MM-ddTHH:mm:ssZ')
} else {
    $CommitTime.ToString('yyyy-MM-ddTHH:mm:ssZ')
}
$BuildId = if ($env:TAKOKIT_BUILD_ID) { $env:TAKOKIT_BUILD_ID } else { "windows-v$Version-$CommitSha" }
$StableManifestUrl = if ($env:TAKOKIT_STABLE_UPDATE_MANIFEST_URL) {
    $env:TAKOKIT_STABLE_UPDATE_MANIFEST_URL
} else {
    'https://github.com/amaansyed27/Takokit/releases/latest/download/release-manifest.json'
}
$PreviewManifestUrl = if ($env:TAKOKIT_PREVIEW_UPDATE_MANIFEST_URL) {
    $env:TAKOKIT_PREVIEW_UPDATE_MANIFEST_URL
} else {
    'https://github.com/amaansyed27/Takokit/releases/download/preview/release-manifest.json'
}
$env:TAKOKIT_BUILD_ID = $BuildId
$env:SOURCE_DATE_EPOCH = [DateTimeOffset]$CommitTime | ForEach-Object { $_.ToUnixTimeSeconds().ToString() }

if (-not $SkipBuild) {
    Invoke-Checked cargo 'build' '--release' '--locked' '--bin' 'tako' '--bin' 'Takokit' '--bin' 'takokit-server' '--bin' 'takokit-updater'
    Invoke-Checked cargo 'build' '--release' '--locked' '-p' 'takokit-release' '--bin' 'takokit-release-tool'
    Push-Location (Join-Path $RepoRoot 'apps\gui')
    try {
        Invoke-Checked npm 'ci'
        Invoke-Checked npm 'run' 'build'
    } finally {
        Pop-Location
    }
}

$RequiredFiles = @(
    'target\release\tako.exe',
    'target\release\Takokit.exe',
    'target\release\takokit-server.exe',
    'target\release\takokit-updater.exe',
    'target\release\takokit-release-tool.exe',
    'apps\gui\dist\index.html',
    'registry\index.json',
    'LICENSE'
)
foreach ($relative in $RequiredFiles) {
    $path = Join-Path $RepoRoot $relative
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required release input is missing: $relative"
    }
}

if (Test-Path -LiteralPath $OutputRoot) { Remove-Item -LiteralPath $OutputRoot -Recurse -Force }
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
$Staging = Join-Path $OutputRoot '_staging'
$BaseTree = Join-Path $Staging 'base'
$InstalledTree = Join-Path $Staging 'installed'
$PortableTree = Join-Path $Staging 'portable'
New-Item -ItemType Directory -Force -Path (Join-Path $BaseTree 'bin') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $BaseTree 'resources\licenses') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $BaseTree 'resources\icons') | Out-Null

Copy-Item 'target\release\tako.exe' (Join-Path $BaseTree 'bin\tako.exe')
Copy-Item 'target\release\Takokit.exe' (Join-Path $BaseTree 'bin\Takokit.exe')
Copy-Item 'target\release\takokit-server.exe' (Join-Path $BaseTree 'bin\takokit-server.exe')
Copy-Item 'target\release\takokit-updater.exe' (Join-Path $BaseTree 'bin\takokit-updater.exe')
Copy-DirectoryContents (Join-Path $RepoRoot 'apps\gui\dist') (Join-Path $BaseTree 'resources\gui')
Copy-DirectoryContents (Join-Path $RepoRoot 'registry') (Join-Path $BaseTree 'resources\registry')
Copy-Item 'assets\favicon\favicon.ico' (Join-Path $BaseTree 'resources\icons\takokit.ico')
Copy-Item 'LICENSE' (Join-Path $BaseTree 'resources\licenses\LICENSE.txt')

$NoticesPath = Join-Path $BaseTree 'resources\licenses\THIRD_PARTY_NOTICES.md'
Invoke-Checked python 'scripts/release/generate_dependency_notices.py' '--output' $NoticesPath

$Provenance = [ordered]@{
    product = 'Takokit'
    version = $Version
    commit_sha = $CommitSha
    build_id = $BuildId
    build_timestamp = $BuildTimestamp
    source_date_epoch = [int64]$env:SOURCE_DATE_EPOCH
    source_tree_dirty = $IsDirty
    os = 'windows'
    architecture = 'x86_64'
    registry_schema_version = 1
    storage_schema_version = 1
    github_run_id = if ($env:GITHUB_RUN_ID) { $env:GITHUB_RUN_ID } else { $null }
    github_run_attempt = if ($env:GITHUB_RUN_ATTEMPT) { $env:GITHUB_RUN_ATTEMPT } else { $null }
}
Write-Json (Join-Path $BaseTree 'build-provenance.json') $Provenance

function Materialize-DistributionTree {
    param([string]$Destination, [ValidateSet('installed','portable')][string]$Mode)
    if (Test-Path -LiteralPath $Destination) { Remove-Item -LiteralPath $Destination -Recurse -Force }
    Copy-Item -LiteralPath $BaseTree -Destination $Destination -Recurse -Force
    $metadata = [ordered]@{
        product = 'Takokit'
        version = $Version
        mode = $Mode
        install_root = $null
        update_manifest_url = $StableManifestUrl
        update_manifest_urls = [ordered]@{
            stable = $StableManifestUrl
            preview = $PreviewManifestUrl
        }
        default_channel = 'stable'
    }
    Write-Json (Join-Path $Destination 'distribution.json') $metadata
    $releaseMetadata = [ordered]@{
        product = 'Takokit'
        version = $Version
        commit_sha = $CommitSha
        build_id = $BuildId
        build_timestamp = $BuildTimestamp
        distribution_mode = $Mode
        portable = ($Mode -eq 'portable')
    }
    Write-Json (Join-Path $Destination 'release-metadata.json') $releaseMetadata
    Set-TreeTimestamp -Root $Destination -Timestamp $CommitTime
}

Materialize-DistributionTree -Destination $InstalledTree -Mode 'installed'
Materialize-DistributionTree -Destination $PortableTree -Mode 'portable'

$PortableFolderName = "Takokit-v$Version-windows-x86_64"
$PortablePackageRoot = Join-Path $Staging 'portable-package'
$PortableFolder = Join-Path $PortablePackageRoot $PortableFolderName
New-Item -ItemType Directory -Force -Path $PortablePackageRoot | Out-Null
Copy-Item -LiteralPath $PortableTree -Destination $PortableFolder -Recurse -Force
Set-TreeTimestamp -Root $PortablePackageRoot -Timestamp $CommitTime
$PortableZip = Join-Path $OutputRoot "$PortableFolderName.zip"
Compress-Archive -Path (Join-Path $PortablePackageRoot '*') -DestinationPath $PortableZip -CompressionLevel Optimal -Force

$UpdateBundle = Join-Path $OutputRoot "Takokit-v$Version-windows-x86_64-update.zip"
Compress-Archive -Path (Join-Path $InstalledTree '*') -DestinationPath $UpdateBundle -CompressionLevel Optimal -Force

if (-not $SkipInstaller) {
    $Iscc = Find-Iscc
    Write-Host "Using Inno Setup compiler: $Iscc"
    Invoke-Checked $Iscc "/DSourceRoot=$InstalledTree" "/DOutputRoot=$OutputRoot" 'packaging\windows\Takokit.iss'
}

$Installer = Join-Path $OutputRoot "Takokit-v$Version-windows-x86_64-installer.exe"
if (-not $SkipInstaller -and -not (Test-Path -LiteralPath $Installer -PathType Leaf)) {
    throw "Installer compiler did not produce $Installer"
}
$AuthenticodeStatusPath = Join-Path $OutputRoot 'authenticode-status.json'
if (Test-Path -LiteralPath $Installer -PathType Leaf) {
    $Authenticode = Get-AuthenticodeSignature -LiteralPath $Installer
    Write-Json $AuthenticodeStatusPath ([ordered]@{
        artifact = [System.IO.Path]::GetFileName($Installer)
        status = [string]$Authenticode.Status
        status_message = [string]$Authenticode.StatusMessage
        signer_subject = if ($Authenticode.SignerCertificate) { $Authenticode.SignerCertificate.Subject } else { $null }
        signer_thumbprint = if ($Authenticode.SignerCertificate) { $Authenticode.SignerCertificate.Thumbprint } else { $null }
    })
}

$ArtifactRecords = @()
if (Test-Path -LiteralPath $Installer) { $ArtifactRecords += Get-ArtifactRecord 'installer' $Installer }
$ArtifactRecords += Get-ArtifactRecord 'portable' $PortableZip
$ArtifactRecords += Get-ArtifactRecord 'update_bundle' $UpdateBundle

$SigningKeyId = if ($HasProductionSigningKey) { 'takokit-release-v1' } else { 'takokit-test-fixture-v1' }
$Channel = if ($HasProductionSigningKey) { 'stable' } else { 'test' }
$TestFixture = -not $HasProductionSigningKey

$Manifest = [ordered]@{
    product = 'Takokit'
    version = $Version
    channel = $Channel
    commit_sha = $CommitSha
    build_id = $BuildId
    build_timestamp = $BuildTimestamp
    os = 'windows'
    architecture = 'x86_64'
    registry_schema_version = 1
    storage_schema = [ordered]@{
        current = 1
        minimum_readable = 1
        maximum_readable = 1
    }
    minimum_compatible_version = '0.1.0'
    signing_key_id = $SigningKeyId
    test_fixture = $TestFixture
    artifacts = $ArtifactRecords
}
$ManifestPath = Join-Path $OutputRoot 'release-manifest.json'
$SignaturePath = Join-Path $OutputRoot 'release-manifest.sig'
Write-Json $ManifestPath $Manifest

$ReleaseTool = Join-Path $RepoRoot 'target\release\takokit-release-tool.exe'
if ($HasProductionSigningKey) {
    Invoke-Checked $ReleaseTool 'sign' $ManifestPath $SignaturePath
    Invoke-Checked $ReleaseTool 'verify' $ManifestPath $SignaturePath
} else {
    Write-Warning 'No production application release signing key is configured. Producing an explicitly TEST-SIGNED pre-release manifest for manual Slice 4 validation.'
    Invoke-Checked $ReleaseTool 'sign' $ManifestPath $SignaturePath '--test'
    Invoke-Checked $ReleaseTool 'verify' $ManifestPath $SignaturePath '--allow-test'
}

$ReleaseNotesSource = Join-Path $RepoRoot 'docs\release\windows-v0.2.0-release-notes.md'
$ReleaseNotes = Join-Path $OutputRoot 'RELEASE_NOTES-v0.2.0.md'
if (-not (Test-Path -LiteralPath $ReleaseNotesSource)) {
    throw "Release notes source is missing: $ReleaseNotesSource"
}
Copy-Item -LiteralPath $ReleaseNotesSource -Destination $ReleaseNotes
$ProvenancePath = Join-Path $OutputRoot 'build-provenance.json'
Copy-Item -LiteralPath (Join-Path $BaseTree 'build-provenance.json') -Destination $ProvenancePath

$ChecksumFiles = @($Installer, $PortableZip, $UpdateBundle, $ManifestPath, $SignaturePath, $ReleaseNotes, $ProvenancePath, $AuthenticodeStatusPath) |
    Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Leaf) }
$ChecksumLines = foreach ($file in $ChecksumFiles) {
    "$(Get-Sha256 $file)  $([System.IO.Path]::GetFileName($file))"
}
Write-Utf8NoBom -Path (Join-Path $OutputRoot 'SHA256SUMS.txt') -Text (($ChecksumLines -join "`n") + "`n")

if ($IncludeTestUpdateFixture) {
    Invoke-Checked powershell '-NoProfile' '-ExecutionPolicy' 'Bypass' '-File' 'scripts\release\build-test-update-fixture.ps1' '-BaseInstalledTree' $InstalledTree '-OutputRoot' (Join-Path $OutputRoot 'test-update') '-CommitSha' $CommitSha '-ReleaseTool' $ReleaseTool
}

$Summary = [ordered]@{
    version = $Version
    commit_sha = $CommitSha
    build_id = $BuildId
    dirty = $IsDirty
    signing = if ($HasProductionSigningKey) { 'production' } else { 'test-only' }
    installer = if (Test-Path -LiteralPath $Installer) { $Installer } else { $null }
    portable = $PortableZip
    update_bundle = $UpdateBundle
    manifest = $ManifestPath
    signature = $SignaturePath
    checksums = (Join-Path $OutputRoot 'SHA256SUMS.txt')
    provenance = $ProvenancePath
    authenticode = if (Test-Path -LiteralPath $AuthenticodeStatusPath) { $AuthenticodeStatusPath } else { $null }
    test_update = if ($IncludeTestUpdateFixture) { (Join-Path $OutputRoot 'test-update') } else { $null }
}
Write-Json (Join-Path $OutputRoot 'build-summary.json') $Summary
Write-Host ($Summary | ConvertTo-Json -Depth 6)
