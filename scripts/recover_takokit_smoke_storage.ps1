[CmdletBinding()]
param(
    [string]$StorageRoot = (Join-Path $env:TEMP "takokit-all-model-smoke"),
    [string]$Tako = ".\target\release\tako.exe",
    [string]$EvidenceRoot = "$HOME\takokit-test-evidence",
    [switch]$ForceForeignPortOwner
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false

function Resolve-ExistingFile {
    param([string]$Path, [string]$Label)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "$Label does not exist: $Path"
    }
    return (Resolve-Path -LiteralPath $Path).Path
}

function Get-PortOwnerIds {
    param([int]$Port)
    return @(
        Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty OwningProcess -Unique
    )
}

function Get-ProcessRecord {
    param([int]$ProcessId)
    Get-CimInstance Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction SilentlyContinue
}

function Get-ProcessDescription {
    param([int]$ProcessId)
    $process = Get-ProcessRecord $ProcessId
    if (-not $process) { return "PID $ProcessId" }
    return "PID $ProcessId ($($process.Name)) $($process.CommandLine)"
}

function Stop-ProcessTree {
    param([int]$ProcessId)
    if (-not (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)) { return }
    if ($env:OS -eq "Windows_NT") {
        & taskkill.exe /PID $ProcessId /T /F 2>&1 | Out-Null
    } else {
        Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    }
}

function Test-TakokitDaemonProcess {
    param([int]$ProcessId)
    $process = Get-ProcessRecord $ProcessId
    if (-not $process) { return $false }
    return $process.Name -match '^takokit(\.exe)?$' -and
        $process.CommandLine -match '(?i)\bserve\b' -and
        $process.CommandLine -match '(?i)--daemon-child'
}

function Invoke-TakoDirectList {
    param([string]$LogPath)

    $stdoutLog = "$LogPath.stdout.tmp"
    $stderrLog = "$LogPath.stderr.tmp"
    Remove-Item -LiteralPath $stdoutLog, $stderrLog -Force -ErrorAction SilentlyContinue

    $process = Start-Process -FilePath $Tako `
        -ArgumentList @("--direct", "list") `
        -WorkingDirectory (Get-Location).Path `
        -RedirectStandardOutput $stdoutLog `
        -RedirectStandardError $stderrLog `
        -NoNewWindow `
        -PassThru `
        -Wait

    $stdout = if (Test-Path -LiteralPath $stdoutLog) { Get-Content -LiteralPath $stdoutLog -Raw } else { "" }
    $stderr = if (Test-Path -LiteralPath $stderrLog) { Get-Content -LiteralPath $stderrLog -Raw } else { "" }
    @(
        "=== command ===",
        "$Tako --direct list",
        "=== stdout ===",
        $stdout,
        "=== stderr ===",
        $stderr,
        "[exit code: $($process.ExitCode)]"
    ) | Set-Content -LiteralPath $LogPath -Encoding utf8
    Remove-Item -LiteralPath $stdoutLog, $stderrLog -Force -ErrorAction SilentlyContinue

    if ($process.ExitCode -ne 0 -or "$stdout`n$stderr" -match '(?im)^\s*(Failed after|Error:)') {
        throw "direct inventory failed; see $LogPath"
    }
    return $stdout
}

$Tako = Resolve-ExistingFile $Tako "Takokit executable"
$StorageRoot = [System.IO.Path]::GetFullPath([Environment]::ExpandEnvironmentVariables($StorageRoot))
$EvidenceRoot = [System.IO.Path]::GetFullPath([Environment]::ExpandEnvironmentVariables($EvidenceRoot))
New-Item -ItemType Directory -Force $StorageRoot, $EvidenceRoot | Out-Null

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$RecoveryRoot = Join-Path $EvidenceRoot "smoke-recovery-$stamp"
New-Item -ItemType Directory -Force $RecoveryRoot | Out-Null

Write-Host ""
Write-Host "Takokit smoke storage recovery" -ForegroundColor Cyan
Write-Host "Storage:  $StorageRoot"
Write-Host "Evidence: $RecoveryRoot"
Write-Host ""
Write-Host "Models, caches, environments and partial downloads are preserved." -ForegroundColor DarkGray

$PreviousTakokitHome = $env:TAKOKIT_HOME
$env:TAKOKIT_HOME = $StorageRoot

try {
    foreach ($directory in @("manifests", "runtime", "logs")) {
        $source = Join-Path $StorageRoot $directory
        if (Test-Path -LiteralPath $source) {
            Copy-Item -LiteralPath $source -Destination (Join-Path $RecoveryRoot $directory) -Recurse -Force
        }
    }

    $owners = @(Get-PortOwnerIds 5050)
    if ($owners.Count -gt 0) {
        Write-Host "Port 5050 owners:" -ForegroundColor Yellow
        $owners | ForEach-Object { Write-Host "  $(Get-ProcessDescription $_)" }

        foreach ($processId in $owners) {
            if (Test-TakokitDaemonProcess $processId) {
                Write-Host "Stopping orphaned Takokit daemon tree PID $processId..." -ForegroundColor Cyan
                Stop-ProcessTree $processId
                continue
            }
            if (-not $ForceForeignPortOwner) {
                throw "Port 5050 is owned by an unverified process: $(Get-ProcessDescription $processId). Use -ForceForeignPortOwner only after reviewing it."
            }
            Write-Host "Stopping explicitly approved foreign port owner PID $processId..." -ForegroundColor Yellow
            Stop-ProcessTree $processId
        }
    }

    Start-Sleep -Milliseconds 500
    if ((Get-PortOwnerIds 5050).Count -gt 0) {
        throw "Port 5050 is still occupied after cleanup."
    }

    foreach ($name in @("daemon.json", "daemon.pid")) {
        Remove-Item -LiteralPath (Join-Path $StorageRoot "runtime\$name") -Force -ErrorAction SilentlyContinue
    }

    $partial = @(
        Get-ChildItem -LiteralPath $StorageRoot -Recurse -Force -ErrorAction SilentlyContinue |
            Where-Object {
                $_.Name -match '\.part$' -or
                $_.Name -match '^\..+\.download-' -or
                $_.Name -eq 'source.download'
            } |
            Select-Object FullName, Length, LastWriteTime
    )
    $partial | Export-Csv (Join-Path $RecoveryRoot "incomplete-paths.csv") -NoTypeInformation

    $markers = @(
        Get-ChildItem -LiteralPath (Join-Path $StorageRoot "models") -Filter ".takokit-prefetch.json" -Recurse -File -ErrorAction SilentlyContinue |
            Select-Object FullName, Length, LastWriteTime
    )
    $markers | Export-Csv (Join-Path $RecoveryRoot "prefetch-markers.csv") -NoTypeInformation

    $records = @(
        Get-ChildItem -LiteralPath (Join-Path $StorageRoot "manifests\installed-models") -Filter "*.toml" -File -ErrorAction SilentlyContinue |
            ForEach-Object {
                $source = Get-Content -LiteralPath $_.FullName -Raw
                [pscustomobject]@{
                    model = $_.BaseName
                    ready = $source -match '(?im)^\s*status\s*=\s*"ready"\s*$'
                    path = $_.FullName
                    modified = $_.LastWriteTime
                }
            }
    )
    $records | Export-Csv (Join-Path $RecoveryRoot "installed-records.csv") -NoTypeInformation

    Write-Host ""
    Write-Host "Recovered storage inventory" -ForegroundColor Cyan
    Write-Host "Install records:   $($records.Count)"
    Write-Host "Ready records:     $(@($records | Where-Object ready).Count)"
    Write-Host "Prefetch markers:  $($markers.Count)"
    Write-Host "Partial paths kept: $($partial.Count)"

    Write-Host ""
    Write-Host "Ready model records" -ForegroundColor Cyan
    @($records | Where-Object ready | Sort-Object model) | ForEach-Object { Write-Host "  $($_.model)" }

    $listLog = Join-Path $RecoveryRoot "direct-list.log"
    $listOutput = Invoke-TakoDirectList -LogPath $listLog
    if (-not [string]::IsNullOrWhiteSpace($listOutput)) {
        Write-Host ""
        Write-Host "Direct CLI inventory" -ForegroundColor Cyan
        Write-Host $listOutput
    }

    Write-Host ""
    Write-Host "Recovery completed. No download data was removed." -ForegroundColor Green
    Write-Host "Future prefetch and smoke scripts use --direct and do not use port 5050." -ForegroundColor Green
} finally {
    $env:TAKOKIT_HOME = $PreviousTakokitHome
}
