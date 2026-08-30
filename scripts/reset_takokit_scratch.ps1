[CmdletBinding(SupportsShouldProcess, ConfirmImpact = "High")]
param(
    [switch]$CleanGlobalUvCache,
    [switch]$CleanGlobalUvPython
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Get-CanonicalPath {
    param([Parameter(Mandatory)][string]$Path)

    try {
        return [System.IO.Path]::GetFullPath(
            [Environment]::ExpandEnvironmentVariables($Path)
        ).TrimEnd([System.IO.Path]::DirectorySeparatorChar)
    }
    catch {
        return $null
    }
}

function Test-SafeTakokitRoot {
    param([Parameter(Mandatory)][string]$Path)

    $Canonical = Get-CanonicalPath $Path
    if (-not $Canonical) {
        return $false
    }

    $HomePath = Get-CanonicalPath $HOME
    $TempPath = Get-CanonicalPath $env:TEMP
    $DriveRoot = [System.IO.Path]::GetPathRoot($Canonical).TrimEnd("\")
    if ($Canonical -eq $DriveRoot -or $Canonical -eq $HomePath -or $Canonical -eq $TempPath) {
        return $false
    }

    $Leaf = Split-Path -Leaf $Canonical
    return (
        $Leaf -eq ".takokit" -or
        $Leaf -like "takokit-*" -or
        ($TempPath -and $Canonical.StartsWith("$TempPath\takokit-", [StringComparison]::OrdinalIgnoreCase))
    )
}

$ConfiguredHome = $env:TAKOKIT_HOME
$DefaultHome = Join-Path $HOME ".takokit"
$ManagedUv = Join-Path $DefaultHome "tools\uv\uv.exe"
if ($ConfiguredHome) {
    $ConfiguredUv = Join-Path $ConfiguredHome "tools\uv\uv.exe"
    if (Test-Path -LiteralPath $ConfiguredUv) {
        $ManagedUv = $ConfiguredUv
    }
}

Write-Host "Stopping Takokit-owned processes..."
$RootHints = @($ConfiguredHome, $DefaultHome) |
    Where-Object { $_ } |
    ForEach-Object { Get-CanonicalPath $_ } |
    Sort-Object -Unique

$Processes = Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
    Where-Object {
        $Name = [string]$_.Name
        $CommandLine = [string]$_.CommandLine
        $ExecutablePath = [string]$_.ExecutablePath
        $IsTakokitBinary = $Name -in @("tako.exe", "Takokit.exe", "takokit-server.exe", "takokit.exe")
        $IsManagedChild = $Name -in @("python.exe", "pythonw.exe", "uv.exe") -and
            ($RootHints | Where-Object {
                ($CommandLine -and $CommandLine.IndexOf($_, [StringComparison]::OrdinalIgnoreCase) -ge 0) -or
                ($ExecutablePath -and $ExecutablePath.IndexOf($_, [StringComparison]::OrdinalIgnoreCase) -ge 0)
            })
        $IsTakokitBinary -or $IsManagedChild
    }

foreach ($Process in $Processes) {
    if ($PSCmdlet.ShouldProcess("PID $($Process.ProcessId) ($($Process.Name))", "Stop process")) {
        Stop-Process -Id $Process.ProcessId -Force -ErrorAction SilentlyContinue
    }
}

if ($CleanGlobalUvCache -and (Test-Path -LiteralPath $ManagedUv)) {
    if ($PSCmdlet.ShouldProcess("uv's shared cache for all projects", "Run uv cache clean")) {
        & $ManagedUv cache clean
        if ($LASTEXITCODE -ne 0) {
            throw "uv cache clean failed with exit code $LASTEXITCODE"
        }
    }
}

if ($CleanGlobalUvPython -and (Test-Path -LiteralPath $ManagedUv)) {
    if ($PSCmdlet.ShouldProcess("uv-managed Python interpreters for all projects", "Run uv python uninstall --all")) {
        & $ManagedUv python uninstall --all
        if ($LASTEXITCODE -ne 0) {
            throw "uv python uninstall --all failed with exit code $LASTEXITCODE"
        }
    }
}

$Roots = @(
    $ConfiguredHome
    $DefaultHome
    (Join-Path $env:TEMP "takokit-all-model-smoke")
    (Join-Path $env:TEMP "takokit-release-test")
) |
    Where-Object { $_ } |
    ForEach-Object { Get-CanonicalPath $_ } |
    Sort-Object -Unique

$Roots += Get-ChildItem -LiteralPath $env:TEMP -Directory -Force -Filter "takokit-*" -ErrorAction SilentlyContinue |
    ForEach-Object { Get-CanonicalPath $_.FullName }
$Roots = $Roots | Where-Object { $_ } | Sort-Object -Unique

foreach ($Root in $Roots) {
    if (-not (Test-SafeTakokitRoot $Root)) {
        throw "Refusing to remove unsafe path: $Root"
    }
    if ((Test-Path -LiteralPath $Root) -and $PSCmdlet.ShouldProcess($Root, "Remove Takokit storage")) {
        Remove-Item -LiteralPath $Root -Recurse -Force
    }
}

$Evidence = Join-Path $HOME "takokit-test-evidence"
if ((Test-Path -LiteralPath $Evidence) -and $PSCmdlet.ShouldProcess($Evidence, "Remove Takokit test evidence")) {
    Remove-Item -LiteralPath $Evidence -Recurse -Force
}

$TakokitEnvironmentVariables = @(
    "TAKOKIT_HOME"
    "TAKOKIT_API_BASE"
    "TAKOKIT_BASE_URL"
    "TAKOKIT_DAEMON_PORT"
    "TAKOKIT_HOST"
    "TAKOKIT_PORT"
)
foreach ($Name in $TakokitEnvironmentVariables) {
    if ($PSCmdlet.ShouldProcess("User environment variable $Name", "Clear")) {
        [Environment]::SetEnvironmentVariable($Name, $null, "User")
        Remove-Item "Env:$Name" -ErrorAction SilentlyContinue
    }
}

foreach ($Name in @("SmokeStorage", "Tako", "PreviousHome", "Log")) {
    Remove-Variable -Name $Name -Scope Global -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "Takokit scratch reset complete."
Write-Host "Removed model stores, managed Python runtimes, Takokit caches, partial downloads, logs and test evidence."
if (-not $CleanGlobalUvCache) {
    Write-Host "The global uv cache was preserved. Use -CleanGlobalUvCache to reclaim it too (this affects other uv projects)."
}
if (-not $CleanGlobalUvPython) {
    Write-Host "Globally managed uv Python interpreters were preserved. Use -CleanGlobalUvPython to remove them too (this affects other uv projects)."
}
Write-Host "Open a new PowerShell window before rebuilding and pulling models."
