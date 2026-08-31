[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$BaseInstalledTree,
    [Parameter(Mandatory)][string]$OutputRoot,
    [Parameter(Mandatory)][string]$CommitSha,
    [Parameter(Mandatory)][string]$ReleaseTool
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$BaseInstalledTree = [System.IO.Path]::GetFullPath($BaseInstalledTree)
$OutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)
$ReleaseTool = [System.IO.Path]::GetFullPath($ReleaseTool)
$BaseVersion = '0.3.0'
$FixtureVersion = '0.3.1'
$FixtureBuildId = "test-update-v$FixtureVersion-$CommitSha"

function Invoke-Checked {
    param([Parameter(Mandatory)][string]$FilePath, [Parameter(ValueFromRemainingArguments)][string[]]$Arguments)
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$FilePath failed with exit code $LASTEXITCODE" }
}

function Write-Utf8NoBom {
    param([string]$Path, [string]$Text)
    [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UTF8Encoding]::new($false))
}

function Write-Json {
    param([string]$Path, $Value)
    Write-Utf8NoBom $Path (($Value | ConvertTo-Json -Depth 12) + "`n")
}

function Get-Sha256([string]$Path) {
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function New-TestManifest {
    param(
        [string]$Path,
        [string]$ArtifactName,
        [uint64]$ArtifactSize,
        [string]$ArtifactSha,
        [string]$InstallerName,
        [uint64]$InstallerSize,
        [string]$InstallerSha,
        [uint32]$StorageMin = 1,
        [uint32]$StorageMax = 1
    )
    $manifest = [ordered]@{
        product = 'Takokit'
        version = $FixtureVersion
        channel = 'test'
        commit_sha = $CommitSha
        build_id = $FixtureBuildId
        build_timestamp = '2026-08-26T00:00:00Z'
        os = 'windows'
        architecture = 'x86_64'
        registry_schema_version = 1
        storage_schema = [ordered]@{
            current = 1
            minimum_readable = $StorageMin
            maximum_readable = $StorageMax
        }
        minimum_compatible_version = $BaseVersion
        signing_key_id = 'takokit-test-fixture-v1'
        test_fixture = $true
        artifacts = @(
            [ordered]@{
                role = 'update_bundle'
                name = $ArtifactName
                size = $ArtifactSize
                sha256 = $ArtifactSha
                url = $null
            },
            [ordered]@{
                role = 'installer'
                name = $InstallerName
                size = $InstallerSize
                sha256 = $InstallerSha
                url = $null
            }
        )
    }
    Write-Json $Path $manifest
}

if (-not (Test-Path -LiteralPath $BaseInstalledTree -PathType Container)) {
    throw "Base installed distribution is missing: $BaseInstalledTree"
}
if (-not (Test-Path -LiteralPath $ReleaseTool -PathType Leaf)) {
    throw "Release signing tool is missing: $ReleaseTool"
}
if (Test-Path -LiteralPath $OutputRoot) { Remove-Item -LiteralPath $OutputRoot -Recurse -Force }
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null

$TempBase = Join-Path ([System.IO.Path]::GetTempPath()) ("takokit-slice4-update-" + [Guid]::NewGuid().ToString('N'))
$Worktree = Join-Path $TempBase 'source'
$Target = Join-Path $TempBase 'target'
$FixtureTree = Join-Path $TempBase 'fixture-tree'
New-Item -ItemType Directory -Force -Path $TempBase | Out-Null

$OldTarget = $env:CARGO_TARGET_DIR
$OldBuildId = $env:TAKOKIT_BUILD_ID
try {
    Set-Location $RepoRoot
    Invoke-Checked git 'worktree' 'add' '--detach' $Worktree $CommitSha

    $CargoToml = Join-Path $Worktree 'Cargo.toml'
    $CargoText = Get-Content -LiteralPath $CargoToml -Raw
    $UpdatedCargoText = [regex]::Replace(
        $CargoText,
        '(?m)^version\s*=\s*"0\.3\.0"\s*$',
        'version = "0.3.1"',
        1
    )
    if ($UpdatedCargoText -eq $CargoText) {
        throw 'Could not rewrite isolated fixture workspace version from 0.3.0 to 0.3.1.'
    }
    Write-Utf8NoBom $CargoToml $UpdatedCargoText

    $env:CARGO_TARGET_DIR = $Target
    $env:TAKOKIT_BUILD_ID = $FixtureBuildId
    Push-Location $Worktree
    try {
        Invoke-Checked cargo 'build' '--release' '--bin' 'tako' '--bin' 'Takokit' '--bin' 'takokit-server' '--bin' 'takokit-updater'
    } finally {
        Pop-Location
    }

    Copy-Item -LiteralPath $BaseInstalledTree -Destination $FixtureTree -Recurse -Force
    Copy-Item -LiteralPath (Join-Path $Target 'release\tako.exe') -Destination (Join-Path $FixtureTree 'bin\tako.exe') -Force
    Copy-Item -LiteralPath (Join-Path $Target 'release\Takokit.exe') -Destination (Join-Path $FixtureTree 'bin\Takokit.exe') -Force
    Copy-Item -LiteralPath (Join-Path $Target 'release\takokit-server.exe') -Destination (Join-Path $FixtureTree 'bin\takokit-server.exe') -Force
    Copy-Item -LiteralPath (Join-Path $Target 'release\takokit-updater.exe') -Destination (Join-Path $FixtureTree 'bin\takokit-updater.exe') -Force

    $Distribution = [ordered]@{
        product = 'Takokit'
        version = $FixtureVersion
        mode = 'installed'
        install_root = $null
        update_manifest_url = $null
        default_channel = 'stable'
    }
    Write-Json (Join-Path $FixtureTree 'distribution.json') $Distribution
    $Metadata = [ordered]@{
        product = 'Takokit'
        version = $FixtureVersion
        commit_sha = $CommitSha
        build_id = $FixtureBuildId
        build_timestamp = '2026-08-26T00:00:00Z'
        distribution_mode = 'installed'
        test_fixture = $true
    }
    Write-Json (Join-Path $FixtureTree 'release-metadata.json') $Metadata

    $BundleName = "Takokit-v$FixtureVersion-windows-x86_64-update.zip"
    $Bundle = Join-Path $OutputRoot $BundleName
    Compress-Archive -Path (Join-Path $FixtureTree '*') -DestinationPath $Bundle -CompressionLevel Optimal -Force
    $BundleItem = Get-Item -LiteralPath $Bundle
    $BundleSha = Get-Sha256 $Bundle

    $IsccCandidates = @(
        (Get-Command ISCC.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -First 1),
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe')
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Leaf) }
    $Iscc = $IsccCandidates | Select-Object -First 1
    if (-not $Iscc) { throw 'Inno Setup compiler is required for the update fixture installer.' }
    Invoke-Checked $Iscc "/DMyAppVersion=$FixtureVersion" "/DSourceRoot=$FixtureTree" "/DOutputRoot=$OutputRoot" (Join-Path $Worktree 'packaging\windows\Takokit.iss')
    $InstallerName = "Takokit-v$FixtureVersion-windows-x86_64-installer.exe"
    $Installer = Join-Path $OutputRoot $InstallerName
    $InstallerItem = Get-Item -LiteralPath $Installer
    $InstallerSha = Get-Sha256 $Installer

    $Manifest = Join-Path $OutputRoot 'release-manifest.json'
    $Signature = Join-Path $OutputRoot 'release-manifest.sig'
    New-TestManifest -Path $Manifest -ArtifactName $BundleName -ArtifactSize $BundleItem.Length -ArtifactSha $BundleSha -InstallerName $InstallerName -InstallerSize $InstallerItem.Length -InstallerSha $InstallerSha
    Invoke-Checked $ReleaseTool 'sign' $Manifest $Signature '--test'
    Invoke-Checked $ReleaseTool 'verify' $Manifest $Signature '--allow-test'

    # Signed bad-hash manifest: metadata authenticates successfully, artifact verification must fail.
    $BadHashManifest = Join-Path $OutputRoot 'release-manifest-bad-hash.json'
    $BadHashSignature = Join-Path $OutputRoot 'release-manifest-bad-hash.sig'
    New-TestManifest -Path $BadHashManifest -ArtifactName $BundleName -ArtifactSize $BundleItem.Length -ArtifactSha ('0' * 64) -InstallerName $InstallerName -InstallerSize $InstallerItem.Length -InstallerSha $InstallerSha
    Invoke-Checked $ReleaseTool 'sign' $BadHashManifest $BadHashSignature '--test'

    # Signed incompatible storage schema manifest: signature is valid but compatibility must reject it.
    $IncompatibleManifest = Join-Path $OutputRoot 'release-manifest-incompatible.json'
    $IncompatibleSignature = Join-Path $OutputRoot 'release-manifest-incompatible.sig'
    New-TestManifest -Path $IncompatibleManifest -ArtifactName $BundleName -ArtifactSize $BundleItem.Length -ArtifactSha $BundleSha -InstallerName $InstallerName -InstallerSize $InstallerItem.Length -InstallerSha $InstallerSha -StorageMin 99 -StorageMax 99
    Invoke-Checked $ReleaseTool 'sign' $IncompatibleManifest $IncompatibleSignature '--test'

    # Invalid detached signature for the otherwise valid manifest.
    $ValidSignatureObject = Get-Content -LiteralPath $Signature -Raw | ConvertFrom-Json
    $ValidSignatureObject.signature = ('00' * 64)
    Write-Json (Join-Path $OutputRoot 'release-manifest-invalid-signature.sig') $ValidSignatureObject

    # Corrupt artifact with a signed manifest that claims the original expected digest/size.
    $CorruptName = "Takokit-v$FixtureVersion-windows-x86_64-update-corrupt.zip"
    $CorruptBundle = Join-Path $OutputRoot $CorruptName
    Copy-Item -LiteralPath $Bundle -Destination $CorruptBundle
    [System.IO.File]::AppendAllText($CorruptBundle, 'TAKOKIT_CORRUPTION_FIXTURE')
    $CorruptManifest = Join-Path $OutputRoot 'release-manifest-corrupt-artifact.json'
    $CorruptSignature = Join-Path $OutputRoot 'release-manifest-corrupt-artifact.sig'
    New-TestManifest -Path $CorruptManifest -ArtifactName $CorruptName -ArtifactSize $BundleItem.Length -ArtifactSha $BundleSha -InstallerName $InstallerName -InstallerSize $InstallerItem.Length -InstallerSha $InstallerSha
    Invoke-Checked $ReleaseTool 'sign' $CorruptManifest $CorruptSignature '--test'

    $Readme = @"
# Takokit Slice 4 non-production updater fixture

This directory is generated only for Windows updater acceptance. It is deliberately signed with the deterministic Takokit TEST key and must never be published as a production release.

Valid update:

    tako update check --manifest "$Manifest" --signature "$Signature" --allow-test
    tako update apply --manifest "$Manifest" --signature "$Signature" --allow-test

Negative fixtures:

- release-manifest-bad-hash.json/.sig: valid signature, wrong artifact hash
- release-manifest-incompatible.json/.sig: valid signature, incompatible storage schema
- release-manifest-invalid-signature.sig: invalid signature for the valid manifest
- release-manifest-corrupt-artifact.json/.sig + ${CorruptName}: corrupted archive bytes

The helper failpoint integration test covers interruption during replacement/rollback separately; no public 0.1.1 tag or GitHub Release is created.
"@
    Write-Utf8NoBom (Join-Path $OutputRoot 'README.md') $Readme

    $ChecksumPaths = Get-ChildItem -LiteralPath $OutputRoot -File | Where-Object { $_.Name -ne 'SHA256SUMS.txt' } | Sort-Object Name
    $Lines = foreach ($file in $ChecksumPaths) { "$(Get-Sha256 $file.FullName)  $($file.Name)" }
    Write-Utf8NoBom (Join-Path $OutputRoot 'SHA256SUMS.txt') (($Lines -join "`n") + "`n")
} finally {
    $env:CARGO_TARGET_DIR = $OldTarget
    $env:TAKOKIT_BUILD_ID = $OldBuildId
    Set-Location $RepoRoot
    if (Test-Path -LiteralPath $Worktree) {
        & git worktree remove --force $Worktree 2>$null | Out-Null
    }
    if (Test-Path -LiteralPath $TempBase) {
        Remove-Item -LiteralPath $TempBase -Recurse -Force -ErrorAction SilentlyContinue
    }
}
