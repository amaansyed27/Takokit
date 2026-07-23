[CmdletBinding()]
param(
    [string]$SmokeRoot = "$HOME\Downloads\takokit-smoke-inputs",
    [string]$StorageRoot = (Join-Path $env:TEMP "takokit-all-model-smoke"),
    [string]$RvcTarget = "",
    [switch]$PullOnly,
    [switch]$SkipPull,
    [switch]$IncludeWorkstation
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false

function Get-PortOwnerIds {
    return @(
        Get-NetTCPConnection -LocalPort 5050 -State Listen -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty OwningProcess -Unique
    )
}

function Stop-ProcessTree {
    param([int]$ProcessId)

    if (-not (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)) { return }
    try {
        if ($IsWindows -or $env:OS -eq "Windows_NT") {
            & taskkill.exe /PID $ProcessId /T /F 2>&1 | Out-Null
        } else {
            Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
        }
    } catch {}
}

function Read-SmokeDaemonInfo {
    param([string]$Root)

    $path = Join-Path $Root "runtime\daemon.json"
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return $null }
    try {
        return Get-Content -LiteralPath $path -Raw | ConvertFrom-Json -ErrorAction Stop
    } catch {
        return $null
    }
}

function Stop-SmokeDaemon {
    param([string]$Root, [string]$Executable)

    $owners = @(Get-PortOwnerIds)
    if ($owners.Count -eq 0) {
        Remove-Item -LiteralPath (Join-Path $Root "runtime\daemon.json") -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath (Join-Path $Root "runtime\daemon.pid") -Force -ErrorAction SilentlyContinue
        return
    }

    $info = Read-SmokeDaemonInfo $Root
    if (-not $info) {
        throw "Port 5050 is occupied and the smoke storage has no verifiable daemon identity. Run .\scripts\recover_takokit_smoke_storage.ps1 first."
    }

    $recordedRoot = [System.IO.Path]::GetFullPath("$($info.storage_root)")
    $daemonPid = [int]$info.pid
    if ($recordedRoot -ne $Root -or $owners -notcontains $daemonPid) {
        throw "Port 5050 is not owned by the daemon recorded for this smoke storage. Run .\scripts\recover_takokit_smoke_storage.ps1 first."
    }

    try { $null = @(& $Executable daemon stop 2>&1) } catch {}
    Start-Sleep -Milliseconds 600
    if (Get-Process -Id $daemonPid -ErrorAction SilentlyContinue) {
        Stop-ProcessTree $daemonPid
    }
    Start-Sleep -Milliseconds 400

    if ((Get-PortOwnerIds).Count -gt 0) {
        throw "The smoke daemon did not release port 5050."
    }

    Remove-Item -LiteralPath (Join-Path $Root "runtime\daemon.json") -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Join-Path $Root "runtime\daemon.pid") -Force -ErrorAction SilentlyContinue
}

if ($PullOnly -and $SkipPull) {
    throw "-PullOnly and -SkipPull cannot be used together."
}

$Audio = Join-Path $SmokeRoot "test01_20s.wav"
$ReferenceAudio = Join-Path $SmokeRoot "owned-reference.wav"
$ReferenceTextFile = Join-Path $SmokeRoot "owned-reference.txt"
$TrainingSamples = Join-Path $SmokeRoot "gpt-sovits-dataset"
$SmokeRunner = Join-Path $PSScriptRoot "run_all_model_smokes.ps1"
$Tako = (Resolve-Path ".\target\release\tako.exe").Path

$ReferenceText = @'
And so, you don’t have to replace it, but you have to renovate it. And we’ve renovated a massive amount of wall. And in addition to that—and I think very, very importantly—we’ve built a lot of new wall. So it’s all being built.

The new piece, the new section, is very, very exciting, what’s going on there. And you’ll see it, because in January I’m going there. We’re almost having a groundbreaking. It’s such a big section. It’s probably the biggest section we’ll get out. So while we’re fighting over funding, we’re also building.
'@
$ReferenceText = ($ReferenceText -replace '\s+', ' ').Trim()

if (-not (Test-Path -LiteralPath $Audio)) {
    throw "Missing audio file: $Audio"
}
if (-not (Test-Path -LiteralPath (Join-Path $TrainingSamples "train.list"))) {
    throw "Missing GPT-SoVITS train.list: $TrainingSamples"
}
if (-not (Test-Path -LiteralPath (Join-Path $TrainingSamples "wavs"))) {
    throw "Missing GPT-SoVITS wavs folder: $TrainingSamples"
}
if (-not (Test-Path -LiteralPath $SmokeRunner)) {
    throw "Smoke runner not found: $SmokeRunner"
}

Copy-Item -LiteralPath $Audio -Destination $ReferenceAudio -Force
Set-Content -LiteralPath $ReferenceTextFile -Value $ReferenceText -Encoding utf8

$Arguments = @{
    Audio = $Audio
    ReferenceAudio = $ReferenceAudio
    ReferenceText = $ReferenceText
    TrainingSamples = $TrainingSamples
}
if ($PullOnly) { $Arguments.PlanOnly = $true }
if ($SkipPull) { $Arguments.SkipPull = $true }
if ($IncludeWorkstation) { $Arguments.IncludeWorkstation = $true }
if ($RvcTarget) {
    if (-not (Test-Path -LiteralPath $RvcTarget)) {
        throw "RVC target does not exist: $RvcTarget"
    }
    $Arguments.RvcTarget = $RvcTarget
}

$StorageRoot = [System.IO.Path]::GetFullPath(
    [Environment]::ExpandEnvironmentVariables($StorageRoot)
)

Write-Host "Audio:            $Audio"
Write-Host "Reference audio:  $ReferenceAudio"
Write-Host "Reference text:   $ReferenceTextFile"
Write-Host "Training samples: $TrainingSamples"
Write-Host "Smoke storage:    $StorageRoot"
if ($PullOnly) {
    Write-Host "Mode:             pull and readiness verification only" -ForegroundColor Cyan
} elseif ($SkipPull) {
    Write-Host "Mode:             cached plan and inference tests only" -ForegroundColor Cyan
} else {
    Write-Host "Mode:             interleaved pull and inference" -ForegroundColor Cyan
}
if ($IncludeWorkstation) {
    Write-Host "Workstation:      included" -ForegroundColor Yellow
} else {
    Write-Host "Workstation:      skipped as blocked-hardware"
}
if (-not $RvcTarget) {
    Write-Host "RVC:              skipped as blocked-input"
}

$PreviousTakokitHome = $env:TAKOKIT_HOME
$ExitCode = 1
$env:TAKOKIT_HOME = $StorageRoot
try {
    Stop-SmokeDaemon -Root $StorageRoot -Executable $Tako
    & $SmokeRunner @Arguments
    $ExitCode = $LASTEXITCODE
} finally {
    try {
        $env:TAKOKIT_HOME = $StorageRoot
        Stop-SmokeDaemon -Root $StorageRoot -Executable $Tako
    } catch {
        Write-Warning $_.Exception.Message
        $ExitCode = 1
    }
    $env:TAKOKIT_HOME = $PreviousTakokitHome
}
exit $ExitCode
