$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-BootstrapTest {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Write-TestJson {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)]$Value)
    $parent = Split-Path -Parent $Path
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    $json = $Value | ConvertTo-Json -Depth 10
    [System.IO.File]::WriteAllText($Path, $json + "`n", [System.Text.UTF8Encoding]::new($false))
}

function Join-WindowsProcessArguments {
    param([Parameter(Mandatory)][string[]]$Arguments)
    return (($Arguments | ForEach-Object {
        if ($_ -notmatch '[\s"]') { $_ }
        elseif ($_.Contains('"')) { throw 'Test process argument contains an unsupported quote character.' }
        else { '"' + $_ + '"' }
    }) -join ' ')
}

function Get-FreeLoopbackPort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    try { return ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port }
    finally { $listener.Stop() }
}

function Test-LoopbackPort {
    param([int]$Port)
    $client = [System.Net.Sockets.TcpClient]::new()
    try {
        $pending = $client.BeginConnect('127.0.0.1', $Port, $null, $null)
        if (-not $pending.AsyncWaitHandle.WaitOne(300)) { return $false }
        $client.EndConnect($pending)
        return $client.Connected
    } catch { return $false }
    finally { $client.Dispose() }
}

function Start-BootstrapFixtureServer {
    param([Parameter(Mandatory)][string]$Root)

    $python = Get-Command python -ErrorAction SilentlyContinue
    $pythonArgs = @('-m', 'http.server')
    if (-not $python) {
        $python = Get-Command py -ErrorAction SilentlyContinue
        if (-not $python) { throw 'Python is required only for the local bootstrap test fixture server.' }
        $pythonArgs = @('-3', '-m', 'http.server')
    }

    $port = Get-FreeLoopbackPort
    $arguments = @($pythonArgs) + @([string]$port, '--bind', '127.0.0.1', '--directory', $Root)
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $python.Source
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.Arguments = Join-WindowsProcessArguments -Arguments $arguments
    $process = [System.Diagnostics.Process]::Start($startInfo)
    if ($null -eq $process) { throw 'Failed to start local bootstrap fixture server.' }

    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    while ([DateTime]::UtcNow -lt $deadline) {
        if ($process.HasExited) {
            $errorText = $process.StandardError.ReadToEnd()
            throw "Bootstrap fixture server exited early: $errorText"
        }
        if (Test-LoopbackPort -Port $port) {
            return [pscustomobject]@{ Process = $process; Port = $port }
        }
        Start-Sleep -Milliseconds 100
    }
    try { $process.Kill() } catch {}
    throw 'Timed out starting local bootstrap fixture server.'
}

function Stop-BootstrapFixtureServer {
    param($Server)
    if ($null -eq $Server -or $null -eq $Server.Process) { return }
    if (-not $Server.Process.HasExited) {
        try { $Server.Process.Kill() } catch {}
        try { $Server.Process.WaitForExit(5000) } catch {}
    }
    $Server.Process.Dispose()
}

function Get-BootstrapInstallerRecord {
    param([Parameter(Mandatory)][string]$OutputRoot)
    $manifestPath = Join-Path $OutputRoot 'release-manifest.json'
    Assert-BootstrapTest (Test-Path -LiteralPath $manifestPath -PathType Leaf) "Missing $manifestPath"
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    $installer = @($manifest.artifacts | Where-Object { $_.role -eq 'installer' }) | Select-Object -First 1
    Assert-BootstrapTest ($null -ne $installer) 'Release manifest has no installer artifact.'
    $installerPath = Join-Path $OutputRoot ([string]$installer.name)
    Assert-BootstrapTest (Test-Path -LiteralPath $installerPath -PathType Leaf) "Missing installer $installerPath"
    return [pscustomobject]@{
        Manifest = $manifest
        Installer = $installer
        InstallerPath = $installerPath
    }
}

function New-BootstrapFixtureRoot {
    param(
        [Parameter(Mandatory)][string]$OutputRoot,
        [Parameter(Mandatory)][string]$Root,
        [string]$Sha256Override,
        [string]$InstallerSource,
        [string]$InstallerName,
        [string]$VersionOverride,
        [string]$ChannelOverride,
        [Nullable[bool]]$TestFixtureOverride,
        [string]$SigningKeyOverride
    )

    $record = Get-BootstrapInstallerRecord -OutputRoot $OutputRoot
    $source = if ($InstallerSource) { $InstallerSource } else { $record.InstallerPath }
    $name = if ($InstallerName) { $InstallerName } else { [string]$record.Installer.name }
    $artifactRoot = Join-Path $Root 'artifacts'
    New-Item -ItemType Directory -Force -Path $artifactRoot | Out-Null
    Copy-Item -LiteralPath $source -Destination (Join-Path $artifactRoot $name) -Force
    $sha256 = if ($Sha256Override) {
        $Sha256Override
    } else {
        (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    $testFixture = if ($null -ne $TestFixtureOverride) { [bool]$TestFixtureOverride } else { $true }
    $metadata = [ordered]@{
        schema_version = 1
        product = 'Takokit'
        version = if ($VersionOverride) { $VersionOverride } else { [string]$record.Manifest.version }
        channel = if ($ChannelOverride) { $ChannelOverride } else { 'test' }
        platform = 'windows'
        architecture = 'x86_64'
        signing_key_id = if ($SigningKeyOverride) { $SigningKeyOverride } else { 'takokit-test-fixture-v1' }
        test_fixture = $testFixture
        installer = [ordered]@{
            name = $name
            url = '__INSTALLER_URL__'
            sha256 = $sha256
            size = (Get-Item -LiteralPath $source).Length
        }
    }
    $metadataPath = Join-Path $Root 'v1\releases\stable\windows-x86_64.json'
    Write-TestJson -Path $metadataPath -Value $metadata
    return [pscustomobject]@{ MetadataPath = $metadataPath; Metadata = $metadata }
}

function Set-BootstrapFixtureUrls {
    param([Parameter(Mandatory)]$Fixture, [Parameter(Mandatory)][int]$Port)
    $metadata = Get-Content -LiteralPath $Fixture.MetadataPath -Raw | ConvertFrom-Json
    $metadata.installer.url = "http://127.0.0.1:$Port/artifacts/$($metadata.installer.name)"
    Write-TestJson -Path $Fixture.MetadataPath -Value $metadata
}

function Invoke-BootstrapScriptProcess {
    param(
        [Parameter(Mandatory)][string]$BootstrapScript,
        [Parameter(Mandatory)][string]$MetadataUrl,
        [string]$InstallDirectory,
        [string]$TempDirectory,
        [string]$ArchitectureOverride,
        [switch]$ExpectFailure
    )

    $shell = (Get-Command powershell.exe -ErrorAction SilentlyContinue)
    if (-not $shell) { $shell = Get-Command pwsh -ErrorAction Stop }
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $shell.Source
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $arguments = [System.Collections.Generic.List[string]]::new()
    foreach ($argument in @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $BootstrapScript,
        '-ReleaseMetadataUrl', $MetadataUrl, '-AllowTestFixture', '-AllowInsecureLoopbackForTesting')) {
        $arguments.Add($argument)
    }
    if ($InstallDirectory) {
        $arguments.Add('-InstallDirectory')
        $arguments.Add($InstallDirectory)
    }
    if ($ArchitectureOverride) {
        $arguments.Add('-ArchitectureOverrideForTesting')
        $arguments.Add($ArchitectureOverride)
    }
    $startInfo.Arguments = Join-WindowsProcessArguments -Arguments $arguments.ToArray()
    if ($TempDirectory) {
        $startInfo.EnvironmentVariables['TEMP'] = $TempDirectory
        $startInfo.EnvironmentVariables['TMP'] = $TempDirectory
    }

    $process = [System.Diagnostics.Process]::Start($startInfo)
    if ($null -eq $process) { throw 'Failed to start bootstrap PowerShell process.' }
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    $result = [pscustomobject]@{ ExitCode = $process.ExitCode; StdOut = $stdout; StdErr = $stderr }
    $process.Dispose()

    if ($ExpectFailure) {
        Assert-BootstrapTest ($result.ExitCode -ne 0) "Bootstrap unexpectedly succeeded.`n$stdout`n$stderr"
    } else {
        Assert-BootstrapTest ($result.ExitCode -eq 0) "Bootstrap failed with $($result.ExitCode).`n$stdout`n$stderr"
    }
    return $result
}

function Get-UserPathEntryCount {
    param([AllowNull()][string]$PathValue, [string]$Entry)
    if ([string]::IsNullOrWhiteSpace($PathValue)) { return 0 }
    $normalized = $Entry.Trim().TrimEnd('\')
    return @($PathValue -split ';' | ForEach-Object { $_.Trim().TrimEnd('\') } | Where-Object {
        $_ -and [string]::Equals($_, $normalized, [StringComparison]::OrdinalIgnoreCase)
    }).Count
}

function Invoke-BootstrapUninstall {
    param([Parameter(Mandatory)][string]$InstallRoot)
    $uninstaller = Get-ChildItem -LiteralPath $InstallRoot -Filter 'unins*.exe' -File -ErrorAction SilentlyContinue |
        Select-Object -First 1
    Assert-BootstrapTest ($null -ne $uninstaller) "No Inno uninstaller found in $InstallRoot"
    $process = Start-Process -FilePath $uninstaller.FullName -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART') -Wait -PassThru
    Assert-BootstrapTest ($process.ExitCode -eq 0) "Uninstaller failed with exit code $($process.ExitCode)"
}
