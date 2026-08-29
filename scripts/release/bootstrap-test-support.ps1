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
    $reader = $null
    try {
        $pending = $client.BeginConnect('127.0.0.1', $Port, $null, $null)
        if (-not $pending.AsyncWaitHandle.WaitOne(300)) { return $false }
        $client.EndConnect($pending)
        if (-not $client.Connected) { return $false }

        $stream = $client.GetStream()
        $stream.ReadTimeout = 1000
        $request = [System.Text.Encoding]::ASCII.GetBytes("GET /__bootstrap_health__ HTTP/1.1`r`nHost: 127.0.0.1`r`nConnection: close`r`n`r`n")
        $stream.Write($request, 0, $request.Length)
        $stream.Flush()
        $reader = [System.IO.StreamReader]::new($stream, [System.Text.Encoding]::ASCII, $false, 1024, $true)
        $statusLine = $reader.ReadLine()
        return -not [string]::IsNullOrWhiteSpace($statusLine) -and $statusLine.StartsWith('HTTP/1.1 ')
    } catch { return $false }
    finally {
        if ($null -ne $reader) { $reader.Dispose() }
        $client.Dispose()
    }
}

function Start-BootstrapFixtureServer {
    param([Parameter(Mandatory)][string]$Root)

    $port = Get-FreeLoopbackPort
    $job = Start-Job -ArgumentList $Root, $port -ScriptBlock {
        param([string]$Root, [int]$Port)
        $ErrorActionPreference = 'Stop'
        $rootFull = [System.IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
        $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, $Port)
        $listener.Start()
        try {
            while ($true) {
                $client = $listener.AcceptTcpClient()
                $reader = $null
                $file = $null
                try {
                    $client.NoDelay = $true
                    $stream = $client.GetStream()
                    $reader = [System.IO.StreamReader]::new(
                        $stream,
                        [System.Text.Encoding]::ASCII,
                        $false,
                        1024,
                        $true
                    )
                    $requestLine = $reader.ReadLine()
                    if ([string]::IsNullOrWhiteSpace($requestLine)) { continue }
                    while (($header = $reader.ReadLine()) -ne $null -and $header -ne '') {}

                    $parts = $requestLine.Split(' ')
                    $status = '200 OK'
                    $candidate = $null
                    $contentLength = [long]0
                    $contentType = 'application/octet-stream'
                    if ($parts.Count -lt 2 -or $parts[0] -ne 'GET') {
                        $status = '405 Method Not Allowed'
                    } else {
                        $urlPath = $parts[1].Split('?')[0]
                        $decoded = [System.Uri]::UnescapeDataString($urlPath).TrimStart('/')
                        $relative = $decoded.Replace('/', [System.IO.Path]::DirectorySeparatorChar)
                        $candidate = [System.IO.Path]::GetFullPath((Join-Path $Root $relative))
                        if (
                            -not $candidate.StartsWith($rootFull, [StringComparison]::OrdinalIgnoreCase) -or
                            -not (Test-Path -LiteralPath $candidate -PathType Leaf)
                        ) {
                            $status = '404 Not Found'
                            $candidate = $null
                        } else {
                            $contentLength = (Get-Item -LiteralPath $candidate).Length
                            if ($candidate.EndsWith('.json', [StringComparison]::OrdinalIgnoreCase)) {
                                $contentType = 'application/json; charset=utf-8'
                            }
                        }
                    }

                    $headerText = "HTTP/1.1 $status`r`nContent-Type: $contentType`r`nContent-Length: $contentLength`r`nConnection: close`r`n`r`n"
                    $headerBytes = [System.Text.Encoding]::ASCII.GetBytes($headerText)
                    $stream.Write($headerBytes, 0, $headerBytes.Length)
                    if ($null -ne $candidate) {
                        $file = [System.IO.File]::OpenRead($candidate)
                        $buffer = New-Object byte[] 65536
                        while (($read = $file.Read($buffer, 0, $buffer.Length)) -gt 0) {
                            $stream.Write($buffer, 0, $read)
                        }
                    }
                    $stream.Flush()
                } finally {
                    if ($null -ne $file) { $file.Dispose() }
                    if ($null -ne $reader) { $reader.Dispose() }
                    $client.Dispose()
                }
            }
        } finally {
            $listener.Stop()
        }
    }

    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    while ([DateTime]::UtcNow -lt $deadline) {
        if ($job.State -eq 'Failed') {
            $failure = (Receive-Job -Job $job -Keep -ErrorAction SilentlyContinue | Out-String).Trim()
            Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
            throw "Bootstrap fixture server failed: $failure"
        }
        if (Test-LoopbackPort -Port $port) {
            return [pscustomobject]@{ Job = $job; Port = $port }
        }
        Start-Sleep -Milliseconds 100
    }
    Stop-Job -Job $job -ErrorAction SilentlyContinue
    Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
    throw 'Timed out starting local bootstrap fixture server.'
}

function Stop-BootstrapFixtureServer {
    param($Server)
    if ($null -eq $Server -or $null -eq $Server.Job) { return }
    Stop-Job -Job $Server.Job -ErrorAction SilentlyContinue
    Remove-Job -Job $Server.Job -Force -ErrorAction SilentlyContinue
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
    foreach ($argument in @('-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', $BootstrapScript,
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
    $startInfo.EnvironmentVariables['NO_PROXY'] = '127.0.0.1,localhost'
    $startInfo.EnvironmentVariables['no_proxy'] = '127.0.0.1,localhost'
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
