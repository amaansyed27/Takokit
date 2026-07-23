[CmdletBinding()]
param(
    [string]$SmokeRoot = "$HOME\Downloads\takokit-smoke-inputs",
    [string]$StorageRoot = (Join-Path $env:TEMP "takokit-all-model-smoke"),
    [string]$RvcTarget = ""
)

$ErrorActionPreference = "Stop"

$Runner = Join-Path $PSScriptRoot "run_takokit_all_smokes.ps1"
if (-not (Test-Path -LiteralPath $Runner)) {
    throw "Smoke helper not found: $Runner"
}

$Common = @{
    SmokeRoot = $SmokeRoot
    StorageRoot = $StorageRoot
}
if ($RvcTarget) {
    $Common.RvcTarget = $RvcTarget
}

Write-Host "" 
Write-Host "Takokit staged all-model validation" -ForegroundColor Cyan
Write-Host "Phase 1/2: pull every model and verify readiness" -ForegroundColor Cyan
Write-Host "Inference will not start until this phase completes without pull failures."

& $Runner @Common -PullOnly
$PullExitCode = $LASTEXITCODE
if ($PullExitCode -ne 0) {
    Write-Host "" 
    Write-Host "Acquisition phase reported one or more failed pulls." -ForegroundColor Red
    Write-Host "Review the newest failures.csv under $HOME\takokit-test-evidence."
    Write-Host "Fix or resume those pulls, then run this staged command again. Cached successful pulls will be reused."
    exit $PullExitCode
}

Write-Host "" 
Write-Host "Phase 1/2 passed: all supported models are cached and executable." -ForegroundColor Green
Write-Host "Phase 2/2: run cached plan and inference smoke tests" -ForegroundColor Cyan

& $Runner @Common -SkipPull
exit $LASTEXITCODE
