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

if ($PullOnly -and $SkipPull) {
    throw "-PullOnly and -SkipPull cannot be used together."
}

$Audio = Join-Path $SmokeRoot "test01_20s.wav"
$ReferenceAudio = Join-Path $SmokeRoot "owned-reference.wav"
$ReferenceTextFile = Join-Path $SmokeRoot "owned-reference.txt"
$TrainingSamples = Join-Path $SmokeRoot "gpt-sovits-dataset"
$SmokeRunner = Join-Path $PSScriptRoot "run_all_model_smokes.ps1"

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

if ($PullOnly) {
    $Arguments.PlanOnly = $true
}
if ($SkipPull) {
    $Arguments.SkipPull = $true
}
if ($IncludeWorkstation) {
    $Arguments.IncludeWorkstation = $true
}
if ($RvcTarget) {
    if (-not (Test-Path -LiteralPath $RvcTarget)) {
        throw "RVC target does not exist: $RvcTarget"
    }
    $Arguments.RvcTarget = $RvcTarget
}

Write-Host "Audio:            $Audio"
Write-Host "Reference audio:  $ReferenceAudio"
Write-Host "Reference text:   $ReferenceTextFile"
$StorageRoot = [System.IO.Path]::GetFullPath(
    [Environment]::ExpandEnvironmentVariables($StorageRoot)
)
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
$env:TAKOKIT_HOME = $StorageRoot
try {
    & $SmokeRunner @Arguments
    $ExitCode = $LASTEXITCODE
} finally {
    $env:TAKOKIT_HOME = $PreviousTakokitHome
}
exit $ExitCode
