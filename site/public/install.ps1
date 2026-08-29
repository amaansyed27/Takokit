[CmdletBinding()]
param(
    [string]$ReleaseMetadataUrl = 'https://takokit.dawnlightlabs.com/v1/releases/stable/windows-x86_64.json',
    [switch]$AllowTestFixture,
    [switch]$AllowInsecureLoopbackForTesting,
    [string]$InstallDirectory,
    [string]$ArchitectureOverrideForTesting
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$ProductName = 'Takokit'
$ExpectedPlatform = 'windows'
$ExpectedArchitecture = 'x86_64'
$TestSigningKeyId = 'takokit-test-fixture-v1'

function Stop-Install {
    param([Parameter(Mandatory)][string]$Message)
    throw "Takokit installer: $Message"
}

function Test-LoopbackHost {
    param([Parameter(Mandatory)][System.Uri]$Uri)
    return $Uri.IsLoopback -or $Uri.Host -in @('localhost', '127.0.0.1', '::1')
}

function Get-TrustedUri {
    param(
        [Parameter(Mandatory)][string]$Value,
        [Parameter(Mandatory)][string]$Purpose
    )

    $uri = $null
    if (-not [System.Uri]::TryCreate($Value, [System.UriKind]::Absolute, [ref]$uri)) {
        Stop-Install "$Purpose URL is invalid."
    }
    if ($uri.Scheme -eq 'https') {
        return $uri
    }
    if (
        $AllowInsecureLoopbackForTesting -and
        $uri.Scheme -eq 'http' -and
        (Test-LoopbackHost -Uri $uri)
    ) {
        return $uri
    }
    Stop-Install "$Purpose must use HTTPS."
}

function Get-RequiredProperty {
    param(
        [Parameter(Mandatory)]$Object,
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Description
    )

    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property -or $null -eq $property.Value) {
        Stop-Install "$Description is missing from release metadata."
    }
    return $property.Value
}

function Get-ReleaseMetadata {
    param([Parameter(Mandatory)][System.Uri]$Uri)

    try {
        return Invoke-RestMethod -Uri $Uri.AbsoluteUri -Method Get -Headers @{ Accept = 'application/json' } -UseBasicParsing
    } catch {
        Stop-Install "release metadata is unavailable: $($_.Exception.Message)"
    }
}

function Confirm-ReleaseMetadata {
    param([Parameter(Mandatory)]$Metadata)

    $schemaVersion = Get-RequiredProperty $Metadata 'schema_version' 'schema_version'
    $product = [string](Get-RequiredProperty $Metadata 'product' 'product')
    $version = [string](Get-RequiredProperty $Metadata 'version' 'version')
    $channel = [string](Get-RequiredProperty $Metadata 'channel' 'channel')
    $platform = [string](Get-RequiredProperty $Metadata 'platform' 'platform')
    $architecture = [string](Get-RequiredProperty $Metadata 'architecture' 'architecture')
    $signingKeyId = [string](Get-RequiredProperty $Metadata 'signing_key_id' 'signing_key_id')
    $testFixture = [bool](Get-RequiredProperty $Metadata 'test_fixture' 'test_fixture')
    $installer = Get-RequiredProperty $Metadata 'installer' 'installer'
    $installerUrl = [string](Get-RequiredProperty $installer 'url' 'installer.url')
    $installerSha256 = [string](Get-RequiredProperty $installer 'sha256' 'installer.sha256')
    $installerName = [string](Get-RequiredProperty $installer 'name' 'installer.name')

    if ([int]$schemaVersion -ne 1) { Stop-Install 'release metadata schema is not supported.' }
    if ($product -ne $ProductName) { Stop-Install 'release metadata product is not Takokit.' }
    if ($platform -ne $ExpectedPlatform) { Stop-Install 'release metadata is not for Windows.' }
    if ($architecture -ne $ExpectedArchitecture) { Stop-Install 'release metadata is not for Windows x86_64.' }
    if ($version -notmatch '^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$') { Stop-Install 'release metadata version is malformed.' }
    if ($installerName -notmatch '^Takokit-v.+-windows-x86_64-installer\.exe$') { Stop-Install 'release metadata installer name is invalid.' }
    if ($installerSha256 -notmatch '^[0-9a-fA-F]{64}$') { Stop-Install 'release metadata installer SHA-256 is invalid.' }

    if ($testFixture) {
        if (-not $AllowTestFixture) { Stop-Install 'stable release metadata points to a test fixture.' }
        if ($channel -ne 'test' -or $signingKeyId -ne $TestSigningKeyId) {
            Stop-Install 'test release metadata has an invalid trust identity.'
        }
    } else {
        if ($channel -ne 'stable') { Stop-Install 'release metadata is not on the stable channel.' }
        if ([string]::IsNullOrWhiteSpace($signingKeyId) -or $signingKeyId -eq $TestSigningKeyId) {
            Stop-Install 'stable release metadata does not have a production signing identity.'
        }
    }

    $trustedInstallerUri = Get-TrustedUri -Value $installerUrl -Purpose 'installer'
    return [pscustomobject]@{
        version = $version
        installer_name = $installerName
        installer_uri = $trustedInstallerUri
        installer_sha256 = $installerSha256.ToLowerInvariant()
    }
}

function Get-Sha256Hex {
    param([Parameter(Mandatory)][string]$Path)

    $stream = $null
    $sha256 = $null
    try {
        $stream = [System.IO.File]::OpenRead($Path)
        $sha256 = [System.Security.Cryptography.SHA256]::Create()
        $hash = $sha256.ComputeHash($stream)
        return ([System.BitConverter]::ToString($hash)).Replace('-', '').ToLowerInvariant()
    } finally {
        if ($null -ne $sha256) { $sha256.Dispose() }
        if ($null -ne $stream) { $stream.Dispose() }
    }
}

function Quote-WindowsArgument {
    param([Parameter(Mandatory)][string]$Argument)
    if ($Argument -notmatch '[\s"]') { return $Argument }
    if ($Argument.Contains('"')) { Stop-Install 'installer argument contains an unsupported quote character.' }
    return '"' + $Argument + '"'
}

function Invoke-Installer {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string[]]$Arguments
    )

    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $Path
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.Arguments = (($Arguments | ForEach-Object { Quote-WindowsArgument $_ }) -join ' ')
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $startInfo
    if (-not $process.Start()) { Stop-Install 'installer process could not be started.' }
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { Stop-Install "installer failed with exit code $($process.ExitCode)." }
}

function Invoke-NativeCapture {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string[]]$Arguments
    )

    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $Path
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.Arguments = (($Arguments | ForEach-Object { Quote-WindowsArgument $_ }) -join ' ')

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $startInfo
    if (-not $process.Start()) { Stop-Install 'installed CLI validation process could not be started.' }
    try {
        $stdout = $process.StandardOutput.ReadToEnd()
        $stderr = $process.StandardError.ReadToEnd()
        $process.WaitForExit()
        return [pscustomobject]@{
            ExitCode = $process.ExitCode
            StdOut = $stdout
            StdErr = $stderr
        }
    } finally {
        $process.Dispose()
    }
}

function Get-TakokitInstallRoot {
    param([string]$PreferredRoot)

    if ($PreferredRoot) {
        return [System.IO.Path]::GetFullPath($PreferredRoot)
    }

    $uninstallRoot = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall'
    if (Test-Path -LiteralPath $uninstallRoot) {
        $match = Get-ChildItem -LiteralPath $uninstallRoot -ErrorAction SilentlyContinue |
            ForEach-Object { Get-ItemProperty -LiteralPath $_.PSPath -ErrorAction SilentlyContinue } |
            Where-Object {
                $_.DisplayName -like 'Takokit*' -and
                $_.Publisher -eq 'Dawnlight Labs' -and
                -not [string]::IsNullOrWhiteSpace($_.InstallLocation)
            } |
            Select-Object -First 1
        if ($match) { return [System.IO.Path]::GetFullPath([string]$match.InstallLocation) }
    }

    return [System.IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Programs\Takokit'))
}

function Confirm-InstalledTakokit {
    param(
        [Parameter(Mandatory)][string]$Version,
        [string]$PreferredRoot
    )

    $installRoot = Get-TakokitInstallRoot -PreferredRoot $PreferredRoot
    $takoExe = Join-Path $installRoot 'bin\tako.exe'
    if (-not (Test-Path -LiteralPath $takoExe -PathType Leaf)) {
        Stop-Install "installed CLI was not found at $takoExe"
    }

    $result = Invoke-NativeCapture -Path $takoExe -Arguments @('version')
    if ($result.ExitCode -ne 0) {
        Stop-Install "installed CLI failed its version check with exit code $($result.ExitCode)."
    }
    $output = ($result.StdOut + "`n" + $result.StdErr).Trim()
    if ($output -notmatch [regex]::Escape($Version)) {
        Stop-Install "installed CLI version does not match Takokit $Version."
    }
    return $takoExe
}

if ([System.Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    Stop-Install 'Windows is required.'
}

if ($ArchitectureOverrideForTesting -and -not $AllowTestFixture) {
    Stop-Install 'architecture override is only available with explicit test-fixture mode.'
}
$osArchitecture = if ($ArchitectureOverrideForTesting) {
    $ArchitectureOverrideForTesting
} else {
    [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
}
if ($osArchitecture -ne 'X64') {
    Stop-Install "Windows x86_64 is required; detected $osArchitecture."
}

$metadataUri = Get-TrustedUri -Value $ReleaseMetadataUrl -Purpose 'release metadata'
Write-Host 'Takokit installer'
Write-Host 'Finding latest Windows release...'
$metadata = Get-ReleaseMetadata -Uri $metadataUri
$release = Confirm-ReleaseMetadata -Metadata $metadata

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("Takokit install " + [System.Guid]::NewGuid().ToString('N'))
$installerPath = Join-Path $tempRoot $release.installer_name

try {
    New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
    Write-Host "Downloading Takokit $($release.version)..."
    try {
        Invoke-WebRequest -Uri $release.installer_uri.AbsoluteUri -OutFile $installerPath -UseBasicParsing
    } catch {
        Stop-Install "download failed: $($_.Exception.Message)"
    }

    Write-Host 'Verifying download...'
    $actualSha256 = Get-Sha256Hex -Path $installerPath
    if ($actualSha256 -ne $release.installer_sha256) {
        Stop-Install "checksum mismatch. Expected $($release.installer_sha256), got $actualSha256."
    }

    $installerArguments = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CURRENTUSER')
    if ($InstallDirectory) {
        $installerArguments += '/DIR=' + [System.IO.Path]::GetFullPath($InstallDirectory)
    }

    Write-Host 'Installing Takokit...'
    Invoke-Installer -Path $installerPath -Arguments $installerArguments
    $null = Confirm-InstalledTakokit -Version $release.version -PreferredRoot $InstallDirectory

    Write-Host 'Takokit installed successfully.'
    Write-Host ''
    Write-Host 'Open a new terminal and run:'
    Write-Host '  tako'
} finally {
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
