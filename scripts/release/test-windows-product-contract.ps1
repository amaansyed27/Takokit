[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$OutputRoot
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$OutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)
$Version = '0.2.0'
$AppId = '{C5EC7671-2A42-43A6-9ED4-BC9FE091BC91}'
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\${AppId}_is1"

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Get-UserPath {
    return [Environment]::GetEnvironmentVariable('Path', 'User')
}

function Get-PathEntryCount {
    param([AllowNull()][string]$PathValue, [string]$Entry)
    if ([string]::IsNullOrWhiteSpace($PathValue)) { return 0 }
    $normalizedEntry = $Entry.Trim().TrimEnd('\')
    return @(
        $PathValue -split ';' |
            ForEach-Object { $_.Trim().TrimEnd('\') } |
            Where-Object {
                $_ -and [string]::Equals(
                    $_,
                    $normalizedEntry,
                    [StringComparison]::OrdinalIgnoreCase
                )
            }
    ).Count
}

function Invoke-InnoProcess {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [Parameter(Mandatory)][string[]]$Arguments
    )
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FilePath
    $startInfo.UseShellExecute = $false
    foreach ($argument in $Arguments) {
        $startInfo.ArgumentList.Add($argument)
    }
    $process = [System.Diagnostics.Process]::Start($startInfo)
    if ($null -eq $process) { throw "Failed to start $FilePath" }
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) {
        throw "$FilePath failed with exit code $($process.ExitCode)"
    }
}

function Test-TcpPort {
    param([string]$HostName, [int]$Port)
    $client = [System.Net.Sockets.TcpClient]::new()
    try {
        $pending = $client.BeginConnect($HostName, $Port, $null, $null)
        if (-not $pending.AsyncWaitHandle.WaitOne(500)) { return $false }
        $client.EndConnect($pending)
        return $client.Connected
    } catch {
        return $false
    } finally {
        $client.Dispose()
    }
}

function Test-ProcessAlive {
    param([uint32]$ProcessId)
    return $null -ne (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)
}

function Wait-ManagedDaemon {
    param([string]$TakokitHome, [int]$TimeoutSeconds = 20)
    $infoPath = Join-Path $TakokitHome 'runtime\daemon.json'
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        if (Test-Path -LiteralPath $infoPath -PathType Leaf) {
            try {
                $info = Get-Content -LiteralPath $infoPath -Raw | ConvertFrom-Json
                if ((Test-ProcessAlive ([uint32]$info.pid)) -and
                    (Test-TcpPort -HostName ([string]$info.host) -Port ([int]$info.port))) {
                    return $info
                }
            } catch {
                # The daemon publishes this file atomically, but tolerate a concurrent read retry.
            }
        }
        Start-Sleep -Milliseconds 250
    }
    throw "Timed out waiting for managed daemon at $infoPath"
}

function Wait-DaemonStopped {
    param([uint32]$ProcessId, [string]$HostName, [int]$Port, [int]$TimeoutSeconds = 20)
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        if ((-not (Test-ProcessAlive $ProcessId)) -and (-not (Test-TcpPort -HostName $HostName -Port $Port))) {
            return
        }
        Start-Sleep -Milliseconds 250
    }
    throw "Timed out waiting for managed daemon $ProcessId to stop and release $HostName`:$Port."
}

function Wait-InstalledUninstall {
    param(
        [string]$InstalledTako,
        [string]$InstalledBin,
        [string]$GuiShortcut,
        [string]$TuiShortcut,
        [uint32]$DaemonPid,
        [string]$DaemonHost,
        [int]$DaemonPort,
        [int]$TimeoutSeconds = 30
    )
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        $complete =
            (-not (Test-Path -LiteralPath $InstalledTako)) -and
            ((Get-PathEntryCount (Get-UserPath) $InstalledBin) -eq 0) -and
            (-not (Test-Path -LiteralPath $GuiShortcut)) -and
            (-not (Test-Path -LiteralPath $TuiShortcut)) -and
            (-not (Test-Path -LiteralPath $UninstallKey)) -and
            (-not (Test-ProcessAlive $DaemonPid)) -and
            (-not (Test-TcpPort -HostName $DaemonHost -Port $DaemonPort))
        if ($complete) { return }
        Start-Sleep -Milliseconds 250
    }
    throw 'Timed out waiting for installed application, shell integration, and managed daemon cleanup.'
}

function Verify-Sha256Sums {
    param([string]$Root)
    $checksumPath = Join-Path $Root 'SHA256SUMS.txt'
    Assert-True (Test-Path -LiteralPath $checksumPath -PathType Leaf) 'SHA256SUMS.txt is missing.'
    foreach ($line in Get-Content -LiteralPath $checksumPath) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        if ($line -notmatch '^([0-9a-fA-F]{64})  (.+)$') {
            throw "Invalid SHA256SUMS line: $line"
        }
        $expected = $Matches[1].ToLowerInvariant()
        $artifact = Join-Path $Root $Matches[2]
        Assert-True (Test-Path -LiteralPath $artifact -PathType Leaf) "Checksummed artifact is missing: $artifact"
        $actual = (Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash.ToLowerInvariant()
        Assert-True ($actual -eq $expected) "SHA-256 mismatch for $artifact"
    }
}

function Invoke-TakoGui {
    param(
        [string]$TakoExe,
        [string]$Workspace,
        [string]$WorkingDirectory
    )
    $oldPath = $env:Path
    $oldLocation = Get-Location
    try {
        $env:Path = "$env:WINDIR\System32;$env:WINDIR;$env:WINDIR\System32\Wbem"
        Set-Location $WorkingDirectory
        $output = & $TakoExe gui --workspace $Workspace 2>&1 | Out-String
        if ($LASTEXITCODE -ne 0) {
            throw "tako gui failed from $WorkingDirectory`: $output"
        }
        Assert-True ($output -match 'Takokit local web GUI') "tako gui did not report the localhost GUI URL: $output"
        return $output.Trim()
    } finally {
        $env:Path = $oldPath
        Set-Location $oldLocation
    }
}

function Assert-GuiServed {
    param([string]$HostName, [int]$Port)
    $guiUrl = "http://${HostName}:$Port/gui"
    $response = Invoke-WebRequest -Uri $guiUrl -UseBasicParsing
    Assert-True ($response.StatusCode -eq 200) "Packaged GUI returned HTTP $($response.StatusCode)."
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$response.Content)) 'Packaged GUI returned an empty index document.'

    $assetPath = @(
        [regex]::Matches([string]$response.Content, '(?:src|href)="([^"]+)"') |
            ForEach-Object { $_.Groups[1].Value } |
            Where-Object { $_ -like '/gui/assets/*' }
    ) | Select-Object -First 1
    Assert-True (-not [string]::IsNullOrWhiteSpace($assetPath)) 'Packaged GUI index did not reference a /gui/assets production asset.'
    $assetResponse = Invoke-WebRequest -Uri "http://${HostName}:$Port$assetPath" -UseBasicParsing
    Assert-True ($assetResponse.StatusCode -eq 200) "Packaged GUI asset $assetPath returned HTTP $($assetResponse.StatusCode)."
    Assert-True ([string]$assetResponse.Content -ne '') "Packaged GUI asset $assetPath was empty."

    $identity = Invoke-RestMethod -Uri "http://${HostName}:$Port/v1/daemon/identity"
    Assert-True ($null -ne $identity) 'Packaged GUI daemon/API identity endpoint did not respond.'
    return $guiUrl
}

$Installer = Join-Path $OutputRoot "Takokit-v$Version-windows-x86_64-installer.exe"
$PortableZip = Join-Path $OutputRoot "Takokit-v$Version-windows-x86_64.zip"
foreach ($required in @($Installer, $PortableZip)) {
    Assert-True (Test-Path -LiteralPath $required -PathType Leaf) "Missing product-contract input: $required"
}

$OriginalTakokitHome = $env:TAKOKIT_HOME
$OriginalProcessPath = $env:Path
$OriginalLocation = Get-Location
$OriginalUserPath = Get-UserPath
$ContractTemp = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [System.IO.Path]::GetTempPath() }
$TempRoot = Join-Path $ContractTemp ("takokit-product-contract-" + [Guid]::NewGuid().ToString('N'))
$StartMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Takokit'
$GuiShortcutPath = Join-Path $StartMenu 'Takokit.lnk'
$TuiShortcutPath = Join-Path $StartMenu 'Takokit (TUI).lnk'
$DefaultHome = Join-Path $HOME '.takokit'
$DefaultHomeExisted = Test-Path -LiteralPath $DefaultHome
$DefaultSentinel = Join-Path $DefaultHome ("slice4-preserve-" + [Guid]::NewGuid().ToString('N') + '.txt')
$Report = [ordered]@{
    sha256sums_verified = $false
    packaged_product = $false
    rvc_resources_packaged = $false
    portable_side_effects = $false
    portable_gui_served = $false
    gui_cli_launch = $false
    gui_web_assets = $false
    gui_safe_workspace = $false
    installed_daemon_owned = $false
    uninstall_daemon_cleanup = $false
    uninstall_preserved_default_home = $false
    uninstall_preserved_workspace = $false
}

try {
    New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
    Verify-Sha256Sums -Root $OutputRoot
    $Report.sha256sums_verified = $true

    # Portable package must be independently runnable and must not register Windows shell state.
    Assert-True (-not (Test-Path -LiteralPath $StartMenu)) 'Portable precondition failed: Takokit Start Menu group already exists.'
    Assert-True (-not (Test-Path -LiteralPath $UninstallKey)) 'Portable precondition failed: Takokit uninstall registration already exists.'
    $PortableExtract = Join-Path $TempRoot 'portable'
    Expand-Archive -LiteralPath $PortableZip -DestinationPath $PortableExtract -Force
    $PortableFolder = Get-ChildItem -LiteralPath $PortableExtract -Directory | Select-Object -First 1
    Assert-True ($null -ne $PortableFolder) 'Portable ZIP has no top-level directory.'
    $PortableRoot = $PortableFolder.FullName
    $PortableTako = Join-Path $PortableRoot 'bin\tako.exe'
    foreach ($relative in @(
        'bin\tako.exe',
        'bin\takokit.exe',
        'bin\takokit-updater.exe',
        'bin\takokit-tray.exe',
        'resources\gui\index.html',
        'resources\registry\index.json',
        'resources\registry\models\rvc.toml',
        'resources\registry\runners\takokit-python-managed.toml',
        'distribution.json',
        'release-metadata.json',
        'build-provenance.json'
    )) {
        $path = Join-Path $PortableRoot $relative
        Assert-True (Test-Path -LiteralPath $path -PathType Leaf) "Packaged product resource is missing: $relative"
    }
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $PortableRoot 'Takokit.exe'))) 'Portable package still contains the removed native desktop wrapper.'
    $Report.packaged_product = $true
    $Report.rvc_resources_packaged = $true

    $PortableUserPathBefore = Get-UserPath
    $OutsideRepo = Join-Path $TempRoot 'outside-repository'
    $PortableHome = Join-Path $TempRoot 'portable-home'
    $PortableWorkspace = Join-Path $TempRoot 'Portable Workspace ü'
    New-Item -ItemType Directory -Force -Path $OutsideRepo, $PortableWorkspace | Out-Null
    Set-Location $OutsideRepo
    $env:TAKOKIT_HOME = $PortableHome
    $env:Path = "$env:WINDIR\System32;$env:WINDIR;$env:WINDIR\System32\Wbem"
    $versionOutput = & $PortableTako version 2>&1 | Out-String
    Assert-True ($LASTEXITCODE -eq 0) "Portable tako version failed: $versionOutput"
    Assert-True ($versionOutput -match '(?m)^distribution:\s+portable\s*$') 'Portable CLI did not identify itself as portable.'
    $rvcOutput = & $PortableTako --direct voice rvc presets 2>&1 | Out-String
    Assert-True ($LASTEXITCODE -eq 0) "Packaged RVC command failed without repository/toolchain PATH: $rvcOutput"
    $env:Path = $OriginalProcessPath

    $PortableUnsafeTakoPaths = @(
        (Join-Path $PortableRoot '.tako'),
        (Join-Path $HOME '.tako'),
        (Join-Path $env:WINDIR 'System32\.tako'),
        (Join-Path $OutsideRepo '.tako')
    )
    foreach ($unsafe in $PortableUnsafeTakoPaths) {
        Assert-True (-not (Test-Path -LiteralPath $unsafe)) "Portable GUI unsafe .tako precondition failed: $unsafe already exists."
    }
    $portableGuiOutput = Invoke-TakoGui -TakoExe $PortableTako -Workspace $PortableWorkspace -WorkingDirectory $OutsideRepo
    $portableDaemon = Wait-ManagedDaemon -TakokitHome $PortableHome
    $portableGuiUrl = Assert-GuiServed -HostName ([string]$portableDaemon.host) -Port ([int]$portableDaemon.port)
    Assert-True ($portableGuiOutput.Contains($portableGuiUrl)) 'Portable tako gui output did not contain the served GUI URL.'
    foreach ($unsafe in $PortableUnsafeTakoPaths) {
        Assert-True (-not (Test-Path -LiteralPath $unsafe)) "Portable GUI launch created unsafe workspace state: $unsafe"
    }
    $portableStop = & $PortableTako daemon stop 2>&1 | Out-String
    Assert-True ($LASTEXITCODE -eq 0) "Portable daemon stop failed: $portableStop"
    Wait-DaemonStopped -ProcessId ([uint32]$portableDaemon.pid) -HostName ([string]$portableDaemon.host) -Port ([int]$portableDaemon.port)
    $Report.portable_gui_served = $true

    Set-Location $OriginalLocation
    Assert-True ([string]::Equals($PortableUserPathBefore, (Get-UserPath), [StringComparison]::Ordinal)) 'Portable execution modified user PATH.'
    Assert-True (-not (Test-Path -LiteralPath $StartMenu)) 'Portable execution created Start Menu entries.'
    Assert-True (-not (Test-Path -LiteralPath $UninstallKey)) 'Portable execution registered an uninstaller.'
    Assert-True (@(Get-ChildItem -LiteralPath $PortableRoot -Filter 'unins*.exe' -Recurse -ErrorAction SilentlyContinue).Count -eq 0) 'Portable tree contains an installer-managed uninstaller.'
    $Report.portable_side_effects = $true

    # Install and prove the real CLI-launched web GUI uses packaged binaries/assets,
    # avoids unsafe inherited workspace locations, and is fully cleaned up by uninstall.
    Remove-Item Env:TAKOKIT_HOME -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $DefaultHome | Out-Null
    Set-Content -LiteralPath $DefaultSentinel -Value 'preserve' -NoNewline
    $WorkspaceRoot = Join-Path $TempRoot 'Workspace Preserve ü'
    $WorkspaceTako = Join-Path $WorkspaceRoot '.tako'
    New-Item -ItemType Directory -Force -Path $WorkspaceTako | Out-Null
    $WorkspaceSentinel = Join-Path $WorkspaceTako 'preserve-me.txt'
    Set-Content -LiteralPath $WorkspaceSentinel -Value 'preserve' -NoNewline

    $InstallRoot = Join-Path $TempRoot 'Installed Contract ü'
    $InstalledBin = Join-Path $InstallRoot 'bin'
    $InstallLog = Join-Path $OutputRoot 'product-contract-install.log'
    Invoke-InnoProcess -FilePath $Installer -Arguments @(
        '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CURRENTUSER',
        "/DIR=$InstallRoot", "/LOG=$InstallLog"
    )
    $InstalledTako = Join-Path $InstalledBin 'tako.exe'
    foreach ($relative in @(
        'bin\tako.exe',
        'bin\takokit.exe',
        'bin\takokit-updater.exe',
        'bin\takokit-tray.exe',
        'resources\gui\index.html',
        'resources\registry\index.json'
    )) {
        Assert-True (Test-Path -LiteralPath (Join-Path $InstallRoot $relative) -PathType Leaf) "Contract install is missing $relative."
    }
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $InstallRoot 'Takokit.exe'))) 'Contract install still contains the removed native desktop wrapper.'
    Assert-True ((Get-PathEntryCount (Get-UserPath) $InstalledBin) -eq 1) 'Contract install did not register exactly one owned PATH entry.'
    Assert-True (Test-Path -LiteralPath $GuiShortcutPath -PathType Leaf) 'Contract install is missing GUI Start Menu shortcut.'
    Assert-True (Test-Path -LiteralPath $TuiShortcutPath -PathType Leaf) 'Contract install is missing TUI Start Menu shortcut.'
    Assert-True (Test-Path -LiteralPath $UninstallKey) 'Contract install did not register its per-user uninstaller.'

    $Shell = New-Object -ComObject WScript.Shell
    $GuiShortcut = $Shell.CreateShortcut($GuiShortcutPath)
    $TuiShortcut = $Shell.CreateShortcut($TuiShortcutPath)
    $ExpectedGuiTarget = $InstalledTako
    $ExpectedTuiTarget = $InstalledTako
    $ExpectedDocuments = [Environment]::GetFolderPath('MyDocuments')
    $ExpectedWorkspace = Join-Path $ExpectedDocuments 'Takokit'
    Assert-True ([string]::Equals($GuiShortcut.TargetPath, $ExpectedGuiTarget, [StringComparison]::OrdinalIgnoreCase)) 'GUI shortcut does not target the packaged CLI executable.'
    Assert-True ($GuiShortcut.Arguments -match '(^|\s)gui(\s|$)') 'GUI shortcut does not invoke the gui command.'
    Assert-True ($GuiShortcut.Arguments -match '--workspace') 'GUI shortcut has no deliberate workspace argument.'
    Assert-True ($GuiShortcut.Arguments.Contains($ExpectedWorkspace)) "GUI shortcut workspace is not the safe Documents/Takokit path: $($GuiShortcut.Arguments)"
    Assert-True ([string]::Equals($TuiShortcut.TargetPath, $ExpectedTuiTarget, [StringComparison]::OrdinalIgnoreCase)) 'TUI shortcut target is not the packaged CLI executable.'
    Assert-True ($TuiShortcut.Arguments -match '--workspace') 'TUI shortcut has no deliberate workspace argument.'
    Assert-True ($TuiShortcut.Arguments.Contains($ExpectedWorkspace)) "TUI shortcut workspace is not the safe Documents/Takokit path: $($TuiShortcut.Arguments)"

    $GuiWorkingDir = $GuiShortcut.WorkingDirectory
    Assert-True (-not [string]::IsNullOrWhiteSpace($GuiWorkingDir)) 'GUI shortcut has no working directory.'
    New-Item -ItemType Directory -Force -Path $GuiWorkingDir | Out-Null
    $UnsafeTakoPaths = @(
        (Join-Path $InstallRoot '.tako'),
        (Join-Path $HOME '.tako'),
        (Join-Path $env:WINDIR 'System32\.tako'),
        (Join-Path $GuiWorkingDir '.tako')
    )
    foreach ($unsafe in $UnsafeTakoPaths) {
        Assert-True (-not (Test-Path -LiteralPath $unsafe)) "Unsafe .tako precondition failed: $unsafe already exists."
    }

    $guiOutput = Invoke-TakoGui -TakoExe $ExpectedGuiTarget -Workspace $ExpectedWorkspace -WorkingDirectory $GuiWorkingDir
    $daemon = Wait-ManagedDaemon -TakokitHome $DefaultHome
    $guiUrl = Assert-GuiServed -HostName ([string]$daemon.host) -Port ([int]$daemon.port)
    Assert-True ($guiOutput.Contains($guiUrl)) 'Installed tako gui output did not contain the served GUI URL.'
    foreach ($unsafe in $UnsafeTakoPaths) {
        Assert-True (-not (Test-Path -LiteralPath $unsafe)) "GUI launch created unsafe workspace state: $unsafe"
    }
    $Report.gui_cli_launch = $true
    $Report.gui_web_assets = $true
    $Report.gui_safe_workspace = $true
    $Report.installed_daemon_owned = $true

    Assert-True (Test-ProcessAlive ([uint32]$daemon.pid)) 'Managed daemon did not remain available after tako gui returned.'
    Assert-True (Test-TcpPort -HostName ([string]$daemon.host) -Port ([int]$daemon.port)) 'Managed daemon port was not available before uninstall.'

    $Uninstaller = Join-Path $InstallRoot 'unins000.exe'
    Assert-True (Test-Path -LiteralPath $Uninstaller -PathType Leaf) 'Contract uninstaller is missing.'
    $UninstallLog = Join-Path $OutputRoot 'product-contract-uninstall.log'
    Invoke-InnoProcess -FilePath $Uninstaller -Arguments @(
        '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=$UninstallLog"
    )
    Wait-InstalledUninstall `
        -InstalledTako $InstalledTako `
        -InstalledBin $InstalledBin `
        -GuiShortcut $GuiShortcutPath `
        -TuiShortcut $TuiShortcutPath `
        -DaemonPid ([uint32]$daemon.pid) `
        -DaemonHost ([string]$daemon.host) `
        -DaemonPort ([int]$daemon.port)

    Assert-True (-not (Test-ProcessAlive ([uint32]$daemon.pid))) 'Uninstall left the owned daemon process running.'
    Assert-True (-not (Test-TcpPort -HostName ([string]$daemon.host) -Port ([int]$daemon.port))) 'Uninstall left the owned daemon port open.'
    Assert-True (Test-Path -LiteralPath $DefaultSentinel -PathType Leaf) 'Uninstall deleted default %USERPROFILE%\.takokit user data.'
    Assert-True (Test-Path -LiteralPath $WorkspaceSentinel -PathType Leaf) 'Uninstall deleted workspace .tako data.'
    foreach ($unsafe in $UnsafeTakoPaths) {
        Assert-True (-not (Test-Path -LiteralPath $unsafe)) "Uninstall/GUI lifecycle left unsafe workspace state: $unsafe"
    }
    $Report.uninstall_daemon_cleanup = $true
    $Report.uninstall_preserved_default_home = $true
    $Report.uninstall_preserved_workspace = $true

    $ReportPath = Join-Path $OutputRoot 'product-contract-report.json'
    [System.IO.File]::WriteAllText(
        $ReportPath,
        (($Report | ConvertTo-Json -Depth 6) + "`n"),
        [System.Text.UTF8Encoding]::new($false)
    )
    Write-Host ($Report | ConvertTo-Json -Depth 6)
} finally {
    $env:Path = $OriginalProcessPath
    Set-Location $OriginalLocation
    if ($null -eq $OriginalTakokitHome) {
        Remove-Item Env:TAKOKIT_HOME -ErrorAction SilentlyContinue
    } else {
        $env:TAKOKIT_HOME = $OriginalTakokitHome
    }
    [Environment]::SetEnvironmentVariable('Path', $OriginalUserPath, 'User')
    Remove-Item -LiteralPath $DefaultSentinel -Force -ErrorAction SilentlyContinue
    if (-not $DefaultHomeExisted -and (Test-Path -LiteralPath $DefaultHome)) {
        Remove-Item -LiteralPath $DefaultHome -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
