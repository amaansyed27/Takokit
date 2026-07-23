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

function Get-ProcessDescription {
    param([int]$ProcessId)

    $process = Get-CimInstance Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction SilentlyContinue
    if (-not $process) {
        return "PID $ProcessId"
    }
    return "PID $ProcessId ($($process.Name)) $($process.CommandLine)"
}

function Stop-ProcessTree {
    param([int]$ProcessId)

    if (-not (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)) {
        return
    }

    if ($IsWindows -or $env:OS -eq "Windows_NT") {
        & taskkill.exe /PID $ProcessId /T /F 2>&1 | Out-Null
    } else {
        Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    }
}

function Read-SmokeDaemonInfo {
    param([string]$Root)

    $path = Join-Path $Root "runtime\daemon.json"
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        return $null
    }

    try {
        return Get-Content -LiteralPath $path -Raw | ConvertFrom-Json -ErrorAction Stop
    } catch {
        Write-Warning "Could not parse $path; it will be preserved in the recovery evidence."
        return $null
    }
}

$Tako = Resolve-ExistingFile $Tako "Takokit executable"
$StorageRoot = [System.IO.Path]::GetFullPath(
    [Environment]::ExpandEnvironmentVariables($StorageRoot)
)
$EvidenceRoot = [System.IO.Path]::GetFullPath(
    [Environment]::ExpandEnvironmentVariables($EvidenceRoot)
)
New-Item -ItemType Directory -Force $StorageRoot, $EvidenceRoot | Out-Null

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$RecoveryRoot = Join-Path $EvidenceRoot "smoke-recovery-$stamp"
New-Item -ItemType Directory -Force $RecoveryRoot | Out-Null

Write-Host ""
Write-Host "Takokit smoke storage recovery" -ForegroundColor Cyan
Write-Host "Storage:  $StorageRoot"
Write-Host "Evidence: $RecoveryRoot"
Write-Host ""
Write-Host "This helper preserves model files, caches, environments and partial downloads." -ForegroundColor DarkGray

$PreviousTakokitHome = $env:TAKOKIT_HOME
$env:TAKOKIT_HOME = $StorageRoot

try {
    foreach ($directory in @("manifests", "runtime", "logs")) {
        $source = Join-Path $StorageRoot $directory
        if (Test-Path -LiteralPath $source) {
            Copy-Item -LiteralPath $source -Destination (Join-Path $RecoveryRoot $directory) -Recurse -Force
        }
    }

    $daemonInfo = Read-SmokeDaemonInfo $StorageRoot
    $portOwners = @(Get-PortOwnerIds 5050)

    if ($portOwners.Count -gt 0) {
        Write-Host "Port 5050 owners:" -ForegroundColor Yellow
        $portOwners | ForEach-Object { Write-Host "  $(Get-ProcessDescription $_)" }
    }

    $ownedPid = $null
    if ($daemonInfo) {
        $recordedRoot = [System.IO.Path]::GetFullPath("$($daemonInfo.storage_root)")
        if ($recordedRoot -eq $StorageRoot -and $portOwners -contains [int]$daemonInfo.pid) {
            $ownedPid = [int]$daemonInfo.pid
            Write-Host "Stopping the managed smoke daemon PID $ownedPid..." -ForegroundColor Cyan

            try {
                $stopOutput = @(& $Tako daemon stop 2>&1)
                $stopOutput | Set-Content -LiteralPath (Join-Path $RecoveryRoot "daemon-stop.log") -Encoding utf8
            } catch {
                $_.Exception.Message | Set-Content -LiteralPath (Join-Path $RecoveryRoot "daemon-stop.log") -Encoding utf8
            }

            Start-Sleep -Milliseconds 750
            if (Get-Process -Id $ownedPid -ErrorAction SilentlyContinue) {
                Write-Host "Graceful stop did not finish; terminating the verified smoke daemon tree." -ForegroundColor Yellow
                Stop-ProcessTree $ownedPid
            }
        }
    }

    Start-Sleep -Milliseconds 500
    $remainingOwners = @(Get-PortOwnerIds 5050)
    if ($remainingOwners.Count -gt 0) {
        $foreign = @($remainingOwners | Where-Object { $_ -ne $ownedPid })
        if ($foreign.Count -gt 0 -and -not $ForceForeignPortOwner) {
            $descriptions = $foreign | ForEach-Object { Get-ProcessDescription $_ }
            throw "Port 5050 is still owned by a process not proven to belong to this smoke storage: $($descriptions -join '; '). Rerun with -ForceForeignPortOwner only after reviewing it."
        }
        foreach ($processId in $remainingOwners) {
            Write-Host "Terminating port owner $(Get-ProcessDescription $processId)" -ForegroundColor Yellow
            Stop-ProcessTree $processId
        }
    }

    Start-Sleep -Milliseconds 500
    if ((Get-PortOwnerIds 5050).Count -gt 0) {
        throw "Port 5050 is still occupied after recovery."
    }

    $runtime = Join-Path $StorageRoot "runtime"
    foreach ($name in @("daemon.json", "daemon.pid")) {
        $path = Join-Path $runtime $name
        if (Test-Path -LiteralPath $path) {
            Remove-Item -LiteralPath $path -Force
        }
    }

    $incomplete = @(
        Get-ChildItem -LiteralPath $StorageRoot -Recurse -Force -ErrorAction SilentlyContinue |
            Where-Object {
                $_.Name -match '\.part$' -or
                $_.Name -match '^\..+\.download-' -or
                $_.Name -eq 'source.download'
            } |
            Select-Object FullName, Length, LastWriteTime
    )
    $incomplete | Export-Csv (Join-Path $RecoveryRoot "incomplete-paths.csv") -NoTypeInformation

    $markers = @(
        Get-ChildItem -LiteralPath (Join-Path $StorageRoot "models") -Filter ".takokit-prefetch.json" -Recurse -File -ErrorAction SilentlyContinue |
            Select-Object FullName, Length, LastWriteTime
    )
    $markers | Export-Csv (Join-Path $RecoveryRoot "prefetch-markers.csv") -NoTypeInformation

    $records = @(
        Get-ChildItem -LiteralPath (Join-Path $StorageRoot "manifests\installed-models") -Filter "*.toml" -File -ErrorAction SilentlyContinue |
            Select-Object BaseName, FullName, Length, LastWriteTime
    )
    $records | Export-Csv (Join-Path $RecoveryRoot "installed-records.csv") -NoTypeInformation

    Write-Host ""
    Write-Host "Recovered storage inventory" -ForegroundColor Cyan
    Write-Host "Installed records: $($records.Count)"
    Write-Host "Prefetch markers:  $($markers.Count)"
    Write-Host "Partial paths kept: $($incomplete.Count)"

    $listLog = Join-Path $RecoveryRoot "tako-list.log"
    $listOutput = @(& $Tako list 2>&1)
    $listExitCode = $LASTEXITCODE
    $listOutput | Set-Content -LiteralPath $listLog -Encoding utf8
    $listOutput | ForEach-Object { Write-Host $_ }

    if ($listExitCode -ne 0 -or ($listOutput -join "`n") -match '(?im)^\s*(Failed after|Error:)') {
        throw "tako list failed against the recovered smoke storage. See $listLog"
    }

    Write-Host ""
    Write-Host "Storage recovery completed without deleting downloads." -ForegroundColor Green
    Write-Host "Use this storage in the current shell:" -ForegroundColor Cyan
    Write-Host '$env:TAKOKIT_HOME = $SmokeStorage'
    Write-Host '& $Tako list'
} finally {
    try {
        $env:TAKOKIT_HOME = $StorageRoot
        $null = @(& $Tako daemon stop 2>&1)
    } catch {}
    $env:TAKOKIT_HOME = $PreviousTakokitHome
}
