[CmdletBinding()]
param(
    [string]$StorageRoot = $env:TAKOKIT_HOME,
    [string]$EvidenceRoot = "$HOME\takokit-test-evidence",
    [string]$WorkspaceRoot = (Get-Location).Path,
    [switch]$IncludeEvidence,
    [switch]$IncludeWorkspaceOutputs,
    [switch]$AllowNonTempStorage,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

function Get-FullPath {
    param([string]$Path)
    return [System.IO.Path]::GetFullPath(
        [Environment]::ExpandEnvironmentVariables($Path)
    ).TrimEnd([char[]]"\/")
}

function Assert-SafeDeleteRoot {
    param([string]$Path)

    $full = Get-FullPath $Path
    $driveRoot = [System.IO.Path]::GetPathRoot($full).TrimEnd([char[]]"\/")
    $tempRoot = Get-FullPath $env:TEMP
    $userRoot = Get-FullPath $HOME

    if ($full -eq $driveRoot -or $full -eq $tempRoot -or $full -eq $userRoot) {
        throw "Refusing to delete unsafe root: $full"
    }
    if (-not $AllowNonTempStorage -and
        -not $full.StartsWith($tempRoot + [System.IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Storage is outside TEMP: $full. Pass -AllowNonTempStorage only if this exact directory is disposable."
    }
    return $full
}

if (-not $StorageRoot) {
    throw "TAKOKIT_HOME is empty. Pass -StorageRoot with the exact disposable smoke-test directory."
}

$storage = Assert-SafeDeleteRoot $StorageRoot
$targets = [System.Collections.Generic.List[string]]::new()
$targets.Add($storage)

if ($IncludeEvidence) {
    $evidence = Get-FullPath $EvidenceRoot
    if ($evidence -eq (Get-FullPath $HOME)) {
        throw "Refusing to delete the user profile."
    }
    $targets.Add($evidence)
}

if ($IncludeWorkspaceOutputs) {
    $sessions = Get-FullPath (Join-Path $WorkspaceRoot ".tako\sessions")
    $workspace = Get-FullPath $WorkspaceRoot
    if (-not $sessions.StartsWith($workspace + [System.IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Resolved session path escaped the workspace."
    }
    $targets.Add($sessions)
}

Write-Host "Takokit smoke cleanup targets:" -ForegroundColor Cyan
$targets | ForEach-Object {
    $exists = Test-Path -LiteralPath $_
    Write-Host "  $_ (exists: $exists)"
}

if (-not $Force) {
    Write-Host ""
    Write-Host "Dry run only. Re-run with -Force to delete these targets." -ForegroundColor Yellow
    exit 0
}

foreach ($target in $targets) {
    if (Test-Path -LiteralPath $target) {
        Write-Host "Deleting $target ..." -ForegroundColor Yellow
        Remove-Item -LiteralPath $target -Recurse -Force
    }
}

Write-Host "Smoke-test storage cleanup complete." -ForegroundColor Green
