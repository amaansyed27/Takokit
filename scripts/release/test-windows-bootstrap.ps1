[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$OutputRoot
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'bootstrap-test-support.ps1')

$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$OutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)
$BootstrapScript = Join-Path $RepoRoot 'site\public\install.ps1'
$AppId = '{C5EC7671-2A42-43A6-9ED4-BC9FE091BC91}'
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\${AppId}_is1"
$record = Get-BootstrapInstallerRecord -OutputRoot $OutputRoot
$Version = [string]$record.Manifest.version
$InstallerName = [string]$record.Installer.name
$InstallerHash = (Get-FileHash -LiteralPath $record.InstallerPath -Algorithm SHA256).Hash.ToLowerInvariant()

Assert-BootstrapTest (Test-Path -LiteralPath $BootstrapScript -PathType Leaf) "Missing bootstrap script: $BootstrapScript"
Assert-BootstrapTest ($InstallerHash -eq ([string]$record.Installer.sha256).ToLowerInvariant()) 'Canonical installer hash does not match release-manifest.json.'
Assert-BootstrapTest (-not (Test-Path -LiteralPath $UninstallKey)) 'Bootstrap acceptance requires no Takokit installation from an earlier suite.'

$OriginalTemp = $env:TEMP
$OriginalTmp = $env:TMP
$OriginalUserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$TempRoot = Join-Path $env:RUNNER_TEMP ("Takokit bootstrap acceptance ü " + [Guid]::NewGuid().ToString('N'))
$FixtureRoot = Join-Path $TempRoot 'Fixture Server'
$UnicodeTemp = Join-Path $TempRoot 'Temporary Files ü'
$InstallRoot = Join-Path $TempRoot 'Installed Takokit ü'
$BadHashInstallRoot = Join-Path $TempRoot 'Must Not Install ü'
$DefaultHome = Join-Path $HOME '.takokit'
$HomeSentinel = Join-Path $DefaultHome ("bootstrap-preserve-" + [Guid]::NewGuid().ToString('N') + '.txt')
$WorkspaceRoot = Join-Path $TempRoot 'Workspace Preserve ü'
$WorkspaceSentinel = Join-Path $WorkspaceRoot '.tako\preserve.txt'
$Server = $null
$Log = [System.Collections.Generic.List[string]]::new()
$Report = [ordered]@{
    valid_windows_x86_64_release = $false
    valid_installer_sha256 = $false
    bad_hash_rejected = $false
    bad_hash_installer_not_executed = $false
    missing_metadata_rejected = $false
    malformed_metadata_rejected = $false
    failed_download_rejected = $false
    unsupported_architecture_rejected = $false
    installer_failure_propagated = $false
    temporary_files_cleaned = $false
    unicode_space_paths = $false
    installed_cli_validated = $false
    reinstall_succeeded = $false
    path_entry_deduplicated = $false
    takokit_home_preserved = $false
    workspace_preserved = $false
    uninstall_succeeded = $false
}

function Add-ResultLog {
    param([string]$Name, $Result)
    $Log.Add("=== $Name ===")
    if ($Result.StdOut) { $Log.Add($Result.StdOut.TrimEnd()) }
    if ($Result.StdErr) { $Log.Add($Result.StdErr.TrimEnd()) }
    $Log.Add("exit=$($Result.ExitCode)")
}

function New-Metadata {
    param(
        [string]$Name,
        [string]$ArtifactName = $InstallerName,
        [string]$Sha256 = $InstallerHash,
        [string]$ArtifactUrl,
        [string]$VersionValue = $Version,
        [string]$Channel = 'test',
        [bool]$TestFixture = $true,
        [string]$SigningKeyId = 'takokit-test-fixture-v1'
    )
    $metadata = [ordered]@{
        schema_version = 1
        product = 'Takokit'
        version = $VersionValue
        channel = $Channel
        platform = 'windows'
        architecture = 'x86_64'
        signing_key_id = $SigningKeyId
        test_fixture = $TestFixture
        installer = [ordered]@{
            name = $ArtifactName
            url = $ArtifactUrl
            sha256 = $Sha256
            size = 1
        }
    }
    $path = Join-Path $FixtureRoot $Name
    Write-TestJson -Path $path -Value $metadata
    return $path
}

function Invoke-Case {
    param(
        [string]$Name,
        [string]$MetadataPath,
        [string]$CaseInstallRoot,
        [string]$ArchitectureOverride,
        [switch]$ExpectFailure
    )
    $relative = [System.IO.Path]::GetRelativePath($FixtureRoot, $MetadataPath).Replace('\', '/')
    $url = "http://127.0.0.1:$($Server.Port)/$relative"
    $result = Invoke-BootstrapScriptProcess `
        -BootstrapScript $BootstrapScript `
        -MetadataUrl $url `
        -InstallDirectory $CaseInstallRoot `
        -TempDirectory $UnicodeTemp `
        -ArchitectureOverride $ArchitectureOverride `
        -ExpectFailure:$ExpectFailure
    Add-ResultLog -Name $Name -Result $result
    return $result
}

function Wait-ForBootstrapUninstall {
    param([string]$InstalledTako, [string]$InstalledBin, [int]$TimeoutSeconds = 30)
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        $pathCount = Get-UserPathEntryCount ([Environment]::GetEnvironmentVariable('Path', 'User')) $InstalledBin
        if ((-not (Test-Path -LiteralPath $InstalledTako)) -and $pathCount -eq 0 -and (-not (Test-Path -LiteralPath $UninstallKey))) {
            return
        }
        Start-Sleep -Milliseconds 250
    }
    throw 'Timed out waiting for bootstrap-installed Takokit to uninstall completely.'
}

try {
    New-Item -ItemType Directory -Force -Path $FixtureRoot, $UnicodeTemp, (Split-Path -Parent $WorkspaceSentinel), $DefaultHome | Out-Null
    Copy-Item -LiteralPath $record.InstallerPath -Destination (Join-Path $FixtureRoot $InstallerName) -Force
    Set-Content -LiteralPath $HomeSentinel -Value 'preserve' -NoNewline
    Set-Content -LiteralPath $WorkspaceSentinel -Value 'preserve' -NoNewline

    $failingName = 'Takokit-v9.9.9-windows-x86_64-installer.exe'
    $failingExe = Join-Path $FixtureRoot $failingName
    Add-Type -TypeDefinition 'public static class BootstrapFailure { public static int Main(string[] args) { return 42; } }' -OutputAssembly $failingExe -OutputType ConsoleApplication
    $failingHash = (Get-FileHash -LiteralPath $failingExe -Algorithm SHA256).Hash.ToLowerInvariant()

    $Server = Start-BootstrapFixtureServer -Root $FixtureRoot
    $baseUrl = "http://127.0.0.1:$($Server.Port)"
    $validMetadata = New-Metadata -Name 'valid.json' -ArtifactUrl "$baseUrl/$InstallerName"
    $badHashMetadata = New-Metadata -Name 'bad-hash.json' -ArtifactUrl "$baseUrl/$InstallerName" -Sha256 ('0' * 64)
    $failedDownloadMetadata = New-Metadata -Name 'failed-download.json' -ArtifactUrl "$baseUrl/missing-installer.exe"
    $failingInstallerMetadata = New-Metadata -Name 'installer-failure.json' -ArtifactName $failingName -ArtifactUrl "$baseUrl/$failingName" -Sha256 $failingHash -VersionValue '9.9.9'
    $invalidTrustMetadata = New-Metadata -Name 'invalid-trust.json' -ArtifactUrl "$baseUrl/$InstallerName" -Channel 'stable' -TestFixture $true
    Set-Content -LiteralPath (Join-Path $FixtureRoot 'malformed.json') -Value '{ this is not json' -NoNewline

    $badHash = Invoke-Case -Name 'bad hash' -MetadataPath $badHashMetadata -CaseInstallRoot $BadHashInstallRoot -ExpectFailure
    Assert-BootstrapTest (($badHash.StdOut + $badHash.StdErr) -match 'checksum mismatch') 'Bad hash did not produce checksum rejection.'
    $Report.bad_hash_rejected = $true
    Assert-BootstrapTest (-not (Test-Path -LiteralPath $BadHashInstallRoot)) 'Installer executed despite a bad hash.'
    Assert-BootstrapTest (-not (Test-Path -LiteralPath $UninstallKey)) 'Bad hash unexpectedly registered an installation.'
    $Report.bad_hash_installer_not_executed = $true

    $missingUrl = "$baseUrl/does-not-exist.json"
    $missing = Invoke-BootstrapScriptProcess -BootstrapScript $BootstrapScript -MetadataUrl $missingUrl -InstallDirectory $BadHashInstallRoot -TempDirectory $UnicodeTemp -ExpectFailure
    Add-ResultLog -Name 'missing metadata' -Result $missing
    $Report.missing_metadata_rejected = $true

    $malformed = Invoke-Case -Name 'malformed metadata' -MetadataPath (Join-Path $FixtureRoot 'malformed.json') -CaseInstallRoot $BadHashInstallRoot -ExpectFailure
    $Report.malformed_metadata_rejected = $true

    $downloadFailure = Invoke-Case -Name 'failed download' -MetadataPath $failedDownloadMetadata -CaseInstallRoot $BadHashInstallRoot -ExpectFailure
    Assert-BootstrapTest (($downloadFailure.StdOut + $downloadFailure.StdErr) -match 'download failed') 'Missing installer did not produce a download failure.'
    $Report.failed_download_rejected = $true

    $unsupported = Invoke-Case -Name 'unsupported architecture' -MetadataPath $validMetadata -CaseInstallRoot $BadHashInstallRoot -ArchitectureOverride 'Arm64' -ExpectFailure
    Assert-BootstrapTest (($unsupported.StdOut + $unsupported.StdErr) -match 'x86_64 is required') 'Unsupported architecture was not rejected.'
    $Report.unsupported_architecture_rejected = $true

    $invalidTrust = Invoke-Case -Name 'invalid trust' -MetadataPath $invalidTrustMetadata -CaseInstallRoot $BadHashInstallRoot -ExpectFailure
    Assert-BootstrapTest (($invalidTrust.StdOut + $invalidTrust.StdErr) -match 'invalid trust identity') 'Invalid test trust state was not rejected.'

    $installerFailure = Invoke-Case -Name 'installer failure' -MetadataPath $failingInstallerMetadata -CaseInstallRoot $BadHashInstallRoot -ExpectFailure
    Assert-BootstrapTest (($installerFailure.StdOut + $installerFailure.StdErr) -match 'exit code 42') 'Installer failure code was not propagated.'
    $Report.installer_failure_propagated = $true

    $first = Invoke-Case -Name 'valid install' -MetadataPath $validMetadata -CaseInstallRoot $InstallRoot
    Assert-BootstrapTest ($first.StdOut -match "Takokit installed successfully") 'Valid bootstrap did not report success.'
    $Report.valid_windows_x86_64_release = $true
    $Report.valid_installer_sha256 = $true
    $InstalledBin = Join-Path $InstallRoot 'bin'
    $InstalledTako = Join-Path $InstalledBin 'tako.exe'
    Assert-BootstrapTest (Test-Path -LiteralPath $InstalledTako -PathType Leaf) 'Bootstrap install did not produce bin\tako.exe.'
    $versionOutput = & $InstalledTako version 2>&1 | Out-String
    Assert-BootstrapTest ($LASTEXITCODE -eq 0 -and $versionOutput -match [regex]::Escape($Version)) 'Installed bootstrap CLI version check failed.'
    $Report.installed_cli_validated = $true
    Assert-BootstrapTest ((Get-UserPathEntryCount ([Environment]::GetEnvironmentVariable('Path', 'User')) $InstalledBin) -eq 1) 'Bootstrap install did not own exactly one PATH entry.'

    $second = Invoke-Case -Name 'reinstall' -MetadataPath $validMetadata -CaseInstallRoot $InstallRoot
    Assert-BootstrapTest ($second.StdOut -match "Takokit installed successfully") 'Second bootstrap execution failed.'
    $Report.reinstall_succeeded = $true
    Assert-BootstrapTest ((Get-UserPathEntryCount ([Environment]::GetEnvironmentVariable('Path', 'User')) $InstalledBin) -eq 1) 'Bootstrap reinstall duplicated the PATH entry.'
    $Report.path_entry_deduplicated = $true

    $leftovers = @(Get-ChildItem -LiteralPath $UnicodeTemp -Force -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'Takokit install *' })
    Assert-BootstrapTest ($leftovers.Count -eq 0) 'Bootstrap temporary download directories were not cleaned.'
    $Report.temporary_files_cleaned = $true
    $Report.unicode_space_paths = $true

    Invoke-BootstrapUninstall -InstallRoot $InstallRoot
    Wait-ForBootstrapUninstall -InstalledTako $InstalledTako -InstalledBin $InstalledBin
    $Report.uninstall_succeeded = $true
    Assert-BootstrapTest (Test-Path -LiteralPath $HomeSentinel -PathType Leaf) 'Uninstall removed the preserved .takokit sentinel.'
    $Report.takokit_home_preserved = $true
    Assert-BootstrapTest (Test-Path -LiteralPath $WorkspaceSentinel -PathType Leaf) 'Uninstall removed the preserved workspace .tako sentinel.'
    $Report.workspace_preserved = $true

    Copy-Item -LiteralPath $BootstrapScript -Destination (Join-Path $OutputRoot 'install.ps1') -Force
    $ManualRoot = Join-Path $OutputRoot 'bootstrap-test'
    New-Item -ItemType Directory -Force -Path $ManualRoot | Out-Null
    foreach ($name in @('bootstrap-test-support.ps1', 'invoke-windows-bootstrap-test.ps1')) {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $ManualRoot $name) -Force
    }
    Write-TestJson -Path (Join-Path $OutputRoot 'bootstrap-acceptance-report.json') -Value $Report
    [System.IO.File]::WriteAllLines((Join-Path $OutputRoot 'bootstrap-acceptance.log'), $Log, [System.Text.UTF8Encoding]::new($false))
    Write-Host ($Report | ConvertTo-Json -Depth 4)
} finally {
    Stop-BootstrapFixtureServer -Server $Server
    $env:TEMP = $OriginalTemp
    $env:TMP = $OriginalTmp
    if (Test-Path -LiteralPath $HomeSentinel) { Remove-Item -LiteralPath $HomeSentinel -Force -ErrorAction SilentlyContinue }
    if (Test-Path -LiteralPath $TempRoot) { Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue }
    if ($null -ne $OriginalUserPath -and (-not (Test-Path -LiteralPath $UninstallKey))) {
        [Environment]::SetEnvironmentVariable('Path', $OriginalUserPath, 'User')
    }
}
