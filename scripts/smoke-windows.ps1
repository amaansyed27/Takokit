[CmdletBinding()]
param(
    [switch]$Isolated,
    [switch]$Full
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Push-Location $repo
try {
    if ($Isolated) {
        $env:TAKOKIT_HOME = Join-Path ([System.IO.Path]::GetTempPath()) ("takokit-smoke-" + [guid]::NewGuid().ToString('N'))
        Write-Host "Using isolated TAKOKIT_HOME: $env:TAKOKIT_HOME"
    }
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed" }
    $takokit = Join-Path $repo 'target\release\takokit.exe'
    & $takokit doctor
    if ($Full) {
        & $takokit quickstart --full
    } else {
        & $takokit quickstart
    }
    & $takokit samples create
    & $takokit speak 'Hello from Takokit' --model kokoro
    $sample = Join-Path $env:TAKOKIT_HOME 'samples\hello.wav'
    if (-not $Isolated) { $sample = Join-Path $HOME '.takokit\samples\hello.wav' }
    & $takokit transcribe $sample --model whisper-tiny
    & $takokit test --suite fast --run
    Write-Host "Outputs: $(Join-Path ($(if ($env:TAKOKIT_HOME) { $env:TAKOKIT_HOME } else { Join-Path $HOME '.takokit' })) 'outputs')"
    Write-Host "Logs: $(Join-Path ($(if ($env:TAKOKIT_HOME) { $env:TAKOKIT_HOME } else { Join-Path $HOME '.takokit' })) 'logs')"
} finally {
    Pop-Location
}
