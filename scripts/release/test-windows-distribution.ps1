[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$OutputRoot
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$OutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)
$Version = '0.3.0'
$FixtureVersion = '0.3.1'

function Invoke-Checked {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [Parameter(ValueFromRemainingArguments)][string[]]$Arguments
    )
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$FilePath failed with exit code $LASTEXITCODE"
    }
}

function Invoke-InnoInstaller {
    param(
        [Parameter(Mandatory)][string]$Installer,
        [Parameter(Mandatory)][string]$InstallRoot,
        [Parameter(Mandatory)][string]$LogPath
    )

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $Installer
    $startInfo.UseShellExecute = $false
    foreach ($argument in @(
        '/VERYSILENT',
        '/SUPPRESSMSGBOXES',
        '/NORESTART',
        '/CURRENTUSER',
        "/DIR=$InstallRoot",
        "/LOG=$LogPath"
    )) {
        $startInfo.ArgumentList.Add($argument)
    }

    $process = [System.Diagnostics.Process]::Start($startInfo)
    if ($null -eq $process) {
        throw "Failed to start installer: $Installer"
    }
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) {
        throw "$Installer failed with exit code $($process.ExitCode). See Inno log: $LogPath"
    }
}

function Invoke-InnoUninstaller {
    param(
        [Parameter(Mandatory)][string]$Uninstaller,
        [Parameter(Mandatory)][string]$LogPath
    )

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $Uninstaller
    $startInfo.UseShellExecute = $false
    foreach ($argument in @(
        '/VERYSILENT',
        '/SUPPRESSMSGBOXES',
        '/NORESTART',
        "/LOG=$LogPath"
    )) {
        $startInfo.ArgumentList.Add($argument)
    }

    $process = [System.Diagnostics.Process]::Start($startInfo)
    if ($null -eq $process) {
        throw "Failed to start uninstaller: $Uninstaller"
    }
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) {
        throw "$Uninstaller failed with exit code $($process.ExitCode). See Inno log: $LogPath"
    }
}

function Invoke-ExpectFailure {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [Parameter(ValueFromRemainingArguments)][string[]]$Arguments
    )
    $output = & $FilePath @Arguments 2>&1 | Out-String
    if ($LASTEXITCODE -eq 0) {
        throw "Expected command to fail but it succeeded: $FilePath $($Arguments -join ' ')"
    }
    return $output.Trim()
}

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Get-TakoVersion {
    param([string]$TakoExe)
    $output = & $TakoExe version 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) {
        throw "$TakoExe version failed: $output"
    }
    $first = ($output -split "`r?`n" | Where-Object { $_.Trim() } | Select-Object -First 1).Trim()
    if ($first -notmatch '^takokit\s+(.+)$') {
        throw "Unexpected Takokit version output: $output"
    }
    return [pscustomobject]@{
        Version = $Matches[1]
        Output = $output.Trim()
    }
}

function Get-UserPath {
    return [Environment]::GetEnvironmentVariable('Path', 'User')
}

function Set-UserPath {
    param([AllowNull()][string]$Value)
    [Environment]::SetEnvironmentVariable('Path', $Value, 'User')
}

function Get-PathEntryCount {
    param([AllowNull()][string]$PathValue, [string]$Entry)
    if ([string]::IsNullOrWhiteSpace($PathValue)) { return 0 }
    $normalizedEntry = $Entry.Trim().TrimEnd('\')
    return @(
        $PathValue -split ';' |
            ForEach-Object { $_.Trim().TrimEnd('\') } |
            Where-Object { $_ -and [string]::Equals($_, $normalizedEntry, [StringComparison]::OrdinalIgnoreCase) }
    ).Count
}

function Wait-UninstallCompletion {
    param(
        [Parameter(Mandatory)][string]$InstalledTako,
        [Parameter(Mandatory)][string]$InstalledBin,
        [Parameter(Mandatory)][string]$GuiShortcutPath,
        [Parameter(Mandatory)][string]$TuiShortcutPath,
        [int]$TimeoutSeconds = 30
    )

    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        $cliGone = -not (Test-Path -LiteralPath $InstalledTako)
        $pathGone = (Get-PathEntryCount (Get-UserPath) $InstalledBin) -eq 0
        $guiShortcutGone = -not (Test-Path -LiteralPath $GuiShortcutPath)
        $tuiShortcutGone = -not (Test-Path -LiteralPath $TuiShortcutPath)
        if ($cliGone -and $pathGone -and $guiShortcutGone -and $tuiShortcutGone) {
            return
        }
        Start-Sleep -Milliseconds 250
    }

    throw "Timed out waiting for Inno uninstall completion (cliGone=$cliGone, pathGone=$pathGone, guiShortcutGone=$guiShortcutGone, tuiShortcutGone=$tuiShortcutGone)."
}

function Copy-FreshInstalledTree {
    param([string]$Destination)
    $source = Join-Path $OutputRoot '_staging\installed'
    Assert-True (Test-Path -LiteralPath $source -PathType Container) "Installed staging tree is missing: $source"
    if (Test-Path -LiteralPath $Destination) {
        Remove-Item -LiteralPath $Destination -Recurse -Force
    }
    Copy-Item -LiteralPath $source -Destination $Destination -Recurse -Force
}

function Wait-UpdateJournal {
    param(
        [string]$TakokitHome,
        [ValidateSet('completed','rolled_back')][string]$ExpectedState,
        [int]$TimeoutSeconds = 90
    )
    $journal = Join-Path $TakokitHome 'runtime\update-journal.json'
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        if (Test-Path -LiteralPath $journal -PathType Leaf) {
            try {
                $value = Get-Content -LiteralPath $journal -Raw | ConvertFrom-Json
                if ($value.state -eq $ExpectedState) { return $value }
                if ($value.state -in @('completed','rolled_back') -and $value.state -ne $ExpectedState) {
                    throw "Updater reached unexpected terminal state '$($value.state)': $($value.message)"
                }
            } catch {
                if ($_.Exception.Message -like 'Updater reached unexpected terminal state*') { throw }
            }
        }
        Start-Sleep -Milliseconds 250
    }
    throw "Timed out waiting for updater state '$ExpectedState' at $journal"
}

function Invoke-TestUpdateApply {
    param(
        [string]$InstallRoot,
        [string]$TakokitHome,
        [string]$Manifest,
        [string]$Signature,
        [AllowNull()][string]$Failpoint,
        [string]$ExpectedState
    )
    $oldHome = $env:TAKOKIT_HOME
    $oldFailpoint = $env:TAKOKIT_UPDATER_TEST_FAILPOINT
    try {
        $env:TAKOKIT_HOME = $TakokitHome
        if ($Failpoint) {
            $env:TAKOKIT_UPDATER_TEST_FAILPOINT = $Failpoint
        } else {
            Remove-Item Env:TAKOKIT_UPDATER_TEST_FAILPOINT -ErrorAction SilentlyContinue
        }
        $tako = Join-Path $InstallRoot 'bin\tako.exe'
        Invoke-Checked $tako 'update' 'apply' '--manifest' $Manifest '--signature' $Signature '--allow-test'
        return Wait-UpdateJournal -TakokitHome $TakokitHome -ExpectedState $ExpectedState
    } finally {
        $env:TAKOKIT_HOME = $oldHome
        if ($null -eq $oldFailpoint) {
            Remove-Item Env:TAKOKIT_UPDATER_TEST_FAILPOINT -ErrorAction SilentlyContinue
        } else {
            $env:TAKOKIT_UPDATER_TEST_FAILPOINT = $oldFailpoint
        }
    }
}

if (-not (Test-Path -LiteralPath $OutputRoot -PathType Container)) {
    throw "Windows distribution output is missing: $OutputRoot"
}

$Installer = Join-Path $OutputRoot "Takokit-v$Version-windows-x86_64-installer.exe"
$PortableZip = Join-Path $OutputRoot "Takokit-v$Version-windows-x86_64.zip"
$Manifest = Join-Path $OutputRoot 'release-manifest.json'
$Signature = Join-Path $OutputRoot 'release-manifest.sig'
$ReleaseTool = Join-Path $RepoRoot 'target\release\takokit-release-tool.exe'
$FixtureRoot = Join-Path $OutputRoot 'test-update'
$FixtureManifest = Join-Path $FixtureRoot 'release-manifest.json'
$FixtureSignature = Join-Path $FixtureRoot 'release-manifest.sig'

foreach ($required in @($Installer, $PortableZip, $Manifest, $Signature, $ReleaseTool, $FixtureManifest, $FixtureSignature)) {
    Assert-True (Test-Path -LiteralPath $required -PathType Leaf) "Required Windows acceptance input is missing: $required"
}

# Release metadata and detached signature must be internally consistent before execution tests.
Invoke-Checked $ReleaseTool 'verify' $Manifest $Signature '--allow-test'
Invoke-Checked $ReleaseTool 'verify' $FixtureManifest $FixtureSignature '--allow-test'

$AcceptanceTemp = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [System.IO.Path]::GetTempPath() }
$TempRoot = Join-Path $AcceptanceTemp ("takokit-slice5-acceptance-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
$OriginalUserPath = Get-UserPath
$OriginalTakokitHome = $env:TAKOKIT_HOME
$Report = [ordered]@{
    version = $Version
    portable = $false
    updater_valid = $false
    updater_bad_signature = $false
    updater_bad_hash = $false
    updater_corrupt_artifact = $false
    updater_incompatible_schema = $false
    updater_after_backup_rollback = $false
    updater_after_replace_rollback = $false
    installer_install = $false
    installer_path_dedupe = $false
    installer_shortcut_workspace = $false
    installer_reinstall = $false
    installer_uninstall = $false
    uninstall_preserved_takokit_home = $false
}

try {
    # Portable ZIP: runnable in place, identifies itself as portable, and self-update is refused.
    $PortableExtract = Join-Path $TempRoot 'portable'
    Expand-Archive -LiteralPath $PortableZip -DestinationPath $PortableExtract -Force
    $PortableFolder = Get-ChildItem -LiteralPath $PortableExtract -Directory | Select-Object -First 1
    Assert-True ($null -ne $PortableFolder) 'Portable ZIP did not contain a top-level Takokit directory.'
    $PortableTako = Join-Path $PortableFolder.FullName 'bin\tako.exe'
    Assert-True (Test-Path -LiteralPath (Join-Path $PortableFolder.FullName 'bin\Takokit.exe')) 'Portable ZIP is missing the Takokit Windows application.'
    Assert-True (Test-Path -LiteralPath (Join-Path $PortableFolder.FullName 'bin\takokit-server.exe')) 'Portable ZIP is missing the server runtime.'
    $env:TAKOKIT_HOME = Join-Path $TempRoot 'portable-home'
    $portableVersion = Get-TakoVersion $PortableTako
    Assert-True ($portableVersion.Version -eq $Version) "Portable version mismatch: $($portableVersion.Version)"
    Assert-True ($portableVersion.Output -match '(?m)^distribution:\s+portable\s*$') 'Portable CLI did not report distribution: portable.'
    $portableUpdateFailure = Invoke-ExpectFailure $PortableTako 'update' 'apply' '--manifest' $FixtureManifest '--signature' $FixtureSignature '--allow-test'
    Assert-True ($portableUpdateFailure -match 'self-update is disabled for portable distributions') 'Portable self-update refusal was not explicit.'
    $Report.portable = $true

    # Signed valid candidate -> next-patch test update.
    $ValidInstall = Join-Path $TempRoot 'Takokit Update Valid ü'
    $ValidHome = Join-Path $TempRoot 'update-home-valid'
    Copy-FreshInstalledTree $ValidInstall
    $env:TAKOKIT_HOME = $ValidHome
    $validTako = Join-Path $ValidInstall 'bin\tako.exe'
    Invoke-Checked $validTako 'update' 'check' '--manifest' $FixtureManifest '--signature' $FixtureSignature '--allow-test'
    Invoke-TestUpdateApply -InstallRoot $ValidInstall -TakokitHome $ValidHome -Manifest $FixtureManifest -Signature $FixtureSignature -Failpoint $null -ExpectedState 'completed' | Out-Null
    $updatedVersion = Get-TakoVersion (Join-Path $ValidInstall 'bin\tako.exe')
    Assert-True ($updatedVersion.Version -eq $FixtureVersion) "Valid updater fixture did not install $FixtureVersion."
    $Report.updater_valid = $true

    # Invalid signature must be rejected before artifact staging.
    $InvalidSignature = Join-Path $FixtureRoot 'release-manifest-invalid-signature.sig'
    $sigFailure = Invoke-ExpectFailure $validTako 'update' 'check' '--manifest' $FixtureManifest '--signature' $InvalidSignature '--allow-test'
    Assert-True ($sigFailure -match 'signature|verification') 'Invalid detached signature did not produce a signature verification failure.'
    $Report.updater_bad_signature = $true

    # Incompatible schema must be rejected despite a valid signature.
    $IncompatibleManifest = Join-Path $FixtureRoot 'release-manifest-incompatible.json'
    $IncompatibleSignature = Join-Path $FixtureRoot 'release-manifest-incompatible.sig'
    $SchemaInstall = Join-Path $TempRoot 'Takokit Update Schema'
    Copy-FreshInstalledTree $SchemaInstall
    $env:TAKOKIT_HOME = Join-Path $TempRoot 'update-home-schema'
    $schemaFailure = Invoke-ExpectFailure (Join-Path $SchemaInstall 'bin\tako.exe') 'update' 'check' '--manifest' $IncompatibleManifest '--signature' $IncompatibleSignature '--allow-test'
    Assert-True ($schemaFailure -match 'schema|compatible|compatibility') 'Incompatible storage schema was not rejected.'
    Assert-True ((Get-TakoVersion (Join-Path $SchemaInstall 'bin\tako.exe')).Version -eq $Version) 'Schema rejection changed the installed version.'
    $Report.updater_incompatible_schema = $true

    # Signed wrong-hash and corrupt-artifact fixtures must fail without replacing the candidate.
    $BadHashManifest = Join-Path $FixtureRoot 'release-manifest-bad-hash.json'
    $BadHashSignature = Join-Path $FixtureRoot 'release-manifest-bad-hash.sig'
    $BadHashInstall = Join-Path $TempRoot 'Takokit Update Bad Hash'
    Copy-FreshInstalledTree $BadHashInstall
    $env:TAKOKIT_HOME = Join-Path $TempRoot 'update-home-bad-hash'
    $hashFailure = Invoke-ExpectFailure (Join-Path $BadHashInstall 'bin\tako.exe') 'update' 'apply' '--manifest' $BadHashManifest '--signature' $BadHashSignature '--allow-test'
    Assert-True ($hashFailure -match 'hash|sha|artifact') 'Bad-hash update fixture did not fail artifact validation.'
    Assert-True ((Get-TakoVersion (Join-Path $BadHashInstall 'bin\tako.exe')).Version -eq $Version) 'Bad-hash fixture replaced the installed version.'
    $Report.updater_bad_hash = $true

    $CorruptManifest = Join-Path $FixtureRoot 'release-manifest-corrupt-artifact.json'
    $CorruptSignature = Join-Path $FixtureRoot 'release-manifest-corrupt-artifact.sig'
    $CorruptInstall = Join-Path $TempRoot 'Takokit Update Corrupt'
    Copy-FreshInstalledTree $CorruptInstall
    $env:TAKOKIT_HOME = Join-Path $TempRoot 'update-home-corrupt'
    $corruptFailure = Invoke-ExpectFailure (Join-Path $CorruptInstall 'bin\tako.exe') 'update' 'apply' '--manifest' $CorruptManifest '--signature' $CorruptSignature '--allow-test'
    Assert-True ($corruptFailure -match 'hash|size|artifact') 'Corrupt update artifact was not rejected.'
    Assert-True ((Get-TakoVersion (Join-Path $CorruptInstall 'bin\tako.exe')).Version -eq $Version) 'Corrupt artifact fixture replaced the installed version.'
    $Report.updater_corrupt_artifact = $true

    # Deterministic interruption at both rename boundaries must restore the original application tree.
    foreach ($failpoint in @('after_backup', 'after_replace')) {
        $FailInstall = Join-Path $TempRoot "Takokit Update $failpoint"
        $FailHome = Join-Path $TempRoot "update-home-$failpoint"
        Copy-FreshInstalledTree $FailInstall
        Invoke-TestUpdateApply -InstallRoot $FailInstall -TakokitHome $FailHome -Manifest $FixtureManifest -Signature $FixtureSignature -Failpoint $failpoint -ExpectedState 'rolled_back' | Out-Null
        Assert-True ((Get-TakoVersion (Join-Path $FailInstall 'bin\tako.exe')).Version -eq $Version) "Updater failpoint $failpoint did not restore v$Version."
        if ($failpoint -eq 'after_backup') { $Report.updater_after_backup_rollback = $true }
        if ($failpoint -eq 'after_replace') { $Report.updater_after_replace_rollback = $true }
    }

    # Installer acceptance in a path containing spaces and Unicode.
    $InstallRoot = Join-Path $TempRoot 'Installed Takokit ü'
    $InstalledBin = Join-Path $InstallRoot 'bin'
    $InstallerHome = Join-Path $TempRoot 'installer-home'
    $env:TAKOKIT_HOME = $InstallerHome
    New-Item -ItemType Directory -Force -Path $InstallerHome | Out-Null
    $Sentinel = Join-Path $InstallerHome 'preserve-me.txt'
    Set-Content -LiteralPath $Sentinel -Value 'preserve' -NoNewline

    $InstallLog = Join-Path $OutputRoot 'installer-acceptance-install.log'
    Invoke-InnoInstaller -Installer $Installer -InstallRoot $InstallRoot -LogPath $InstallLog
    Assert-True (Test-Path -LiteralPath (Join-Path $InstalledBin 'tako.exe') -PathType Leaf) 'Installer did not install tako.exe.'
    Assert-True (Test-Path -LiteralPath (Join-Path $InstalledBin 'Takokit.exe') -PathType Leaf) 'Installer did not install the Takokit Windows application.'
    Assert-True (Test-Path -LiteralPath (Join-Path $InstalledBin 'takokit-server.exe') -PathType Leaf) 'Installer did not install takokit-server.exe.'
    Assert-True (Test-Path -LiteralPath (Join-Path $InstalledBin 'takokit-updater.exe') -PathType Leaf) 'Installer did not install takokit-updater.exe.'
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $InstalledBin 'takokit-tray.exe'))) 'Installer ships the removed takokit-tray.exe.'
    $installedVersion = Get-TakoVersion (Join-Path $InstalledBin 'tako.exe')
    Assert-True ($installedVersion.Version -eq $Version) "Installed version mismatch: $($installedVersion.Version)"
    Assert-True ($installedVersion.Output -match '(?m)^distribution:\s+installed\s*$') 'Installed CLI did not report distribution: installed.'
    $Report.installer_install = $true

    $pathCount = Get-PathEntryCount (Get-UserPath) $InstalledBin
    Assert-True ($pathCount -eq 1) "Installer PATH entry count was $pathCount instead of 1."
    $Report.installer_path_dedupe = $true

    $StartMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Takokit'
    $GuiShortcutPath = Join-Path $StartMenu 'Takokit.lnk'
    $TuiShortcutPath = Join-Path $StartMenu 'Takokit (TUI).lnk'
    Assert-True (Test-Path -LiteralPath $GuiShortcutPath -PathType Leaf) "GUI Start Menu shortcut is missing: $GuiShortcutPath"
    Assert-True (Test-Path -LiteralPath $TuiShortcutPath -PathType Leaf) "TUI Start Menu shortcut is missing: $TuiShortcutPath"
    $Shell = New-Object -ComObject WScript.Shell
    $GuiShortcut = $Shell.CreateShortcut($GuiShortcutPath)
    $TuiShortcut = $Shell.CreateShortcut($TuiShortcutPath)
    $ExpectedDocuments = [Environment]::GetFolderPath('MyDocuments')
    $ExpectedWorkspace = Join-Path $ExpectedDocuments 'Takokit'
    $ExpectedTako = Join-Path $InstalledBin 'tako.exe'
    $ExpectedApplication = Join-Path $InstalledBin 'Takokit.exe'
    Assert-True ([string]::Equals($GuiShortcut.TargetPath, $ExpectedApplication, [StringComparison]::OrdinalIgnoreCase)) 'Primary shortcut does not target the installed Takokit Windows application.'
    Assert-True ([string]::IsNullOrWhiteSpace($GuiShortcut.Arguments)) 'Primary shortcut unexpectedly passes internal arguments.'
    Assert-True ([string]::Equals($TuiShortcut.TargetPath, $ExpectedTako, [StringComparison]::OrdinalIgnoreCase)) 'TUI shortcut does not target the installed tako.exe.'
    Assert-True ($TuiShortcut.Arguments -match '--workspace') 'TUI shortcut does not pass an explicit workspace.'
    Assert-True ($TuiShortcut.Arguments.Contains($ExpectedWorkspace)) "TUI shortcut workspace is not the safe Documents/Takokit path: $($TuiShortcut.Arguments)"
    $Report.installer_shortcut_workspace = $true

    & (Join-Path $PSScriptRoot 'test-windows-resident.ps1') -TakoExe $ExpectedTako -ApplicationExe $ExpectedApplication -OutputRoot (Join-Path $OutputRoot 'resident-installer-acceptance') -Port 5167 -AssertNoLegacyTray
    Assert-True ($LASTEXITCODE -eq 0) 'Installed resident Takokit acceptance failed.'

    # Reinstall/repair must not duplicate the PATH entry.
    $ReinstallLog = Join-Path $OutputRoot 'installer-acceptance-reinstall.log'
    Invoke-InnoInstaller -Installer $Installer -InstallRoot $InstallRoot -LogPath $ReinstallLog
    $reinstallCount = Get-PathEntryCount (Get-UserPath) $InstalledBin
    Assert-True ($reinstallCount -eq 1) "Reinstall duplicated the Takokit PATH entry ($reinstallCount entries)."
    Assert-True ((Get-TakoVersion (Join-Path $InstalledBin 'tako.exe')).Version -eq $Version) 'Reinstall did not leave a runnable candidate CLI.'
    $Report.installer_reinstall = $true

    $Uninstaller = Join-Path $InstallRoot 'unins000.exe'
    Assert-True (Test-Path -LiteralPath $Uninstaller -PathType Leaf) 'Takokit uninstaller is missing.'
    $UninstallLog = Join-Path $OutputRoot 'installer-acceptance-uninstall.log'
    Invoke-InnoUninstaller -Uninstaller $Uninstaller -LogPath $UninstallLog
    $InstalledTako = Join-Path $InstalledBin 'tako.exe'
    Wait-UninstallCompletion -InstalledTako $InstalledTako -InstalledBin $InstalledBin -GuiShortcutPath $GuiShortcutPath -TuiShortcutPath $TuiShortcutPath
    Assert-True (-not (Test-Path -LiteralPath $InstalledTako)) 'Uninstall left the installed CLI behind.'
    Assert-True ((Get-PathEntryCount (Get-UserPath) $InstalledBin) -eq 0) 'Uninstall left the Takokit PATH entry behind.'
    Assert-True (-not (Test-Path -LiteralPath $GuiShortcutPath)) 'Uninstall left the GUI Start Menu shortcut behind.'
    Assert-True (-not (Test-Path -LiteralPath $TuiShortcutPath)) 'Uninstall left the TUI Start Menu shortcut behind.'
    $Report.installer_uninstall = $true
    Assert-True (Test-Path -LiteralPath $Sentinel -PathType Leaf) 'Normal uninstall deleted TAKOKIT_HOME user data.'
    $Report.uninstall_preserved_takokit_home = $true

    $ReportPath = Join-Path $OutputRoot 'acceptance-report.json'
    [System.IO.File]::WriteAllText(
        $ReportPath,
        (($Report | ConvertTo-Json -Depth 6) + "`n"),
        [System.Text.UTF8Encoding]::new($false)
    )
    Write-Host ($Report | ConvertTo-Json -Depth 6)
} finally {
    Set-UserPath $OriginalUserPath
    $env:TAKOKIT_HOME = $OriginalTakokitHome
    Remove-Item Env:TAKOKIT_UPDATER_TEST_FAILPOINT -ErrorAction SilentlyContinue
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
