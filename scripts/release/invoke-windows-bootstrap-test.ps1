[CmdletBinding()]
param(
    [string]$OutputRoot,
    [string]$BootstrapScript,
    [string]$InstallDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'bootstrap-test-support.ps1')

$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
if (-not $OutputRoot) {
    $artifactParent = Split-Path -Parent $PSScriptRoot
    if (Test-Path -LiteralPath (Join-Path $artifactParent 'release-manifest.json')) {
        $OutputRoot = $artifactParent
    } else {
        $OutputRoot = Join-Path $RepoRoot 'dist\windows-ci'
    }
}
$OutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)

if (-not $BootstrapScript) {
    $artifactScript = Join-Path $OutputRoot 'install.ps1'
    $BootstrapScript = if (Test-Path -LiteralPath $artifactScript) {
        $artifactScript
    } else {
        Join-Path $RepoRoot 'site\public\install.ps1'
    }
}
$BootstrapScript = [System.IO.Path]::GetFullPath($BootstrapScript)
Assert-BootstrapTest (Test-Path -LiteralPath $BootstrapScript -PathType Leaf) "Missing bootstrap script: $BootstrapScript"

$FixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("Takokit bootstrap fixture " + [Guid]::NewGuid().ToString('N'))
$Server = $null
try {
    New-Item -ItemType Directory -Force -Path $FixtureRoot | Out-Null
    $fixture = New-BootstrapFixtureRoot -OutputRoot $OutputRoot -Root $FixtureRoot
    $Server = Start-BootstrapFixtureServer -Root $FixtureRoot
    Set-BootstrapFixtureUrls -Fixture $fixture -Port $Server.Port
    $metadataUrl = "http://127.0.0.1:$($Server.Port)/v1/releases/stable/windows-x86_64.json"

    Write-Host 'Takokit pre-release bootstrap test'
    $result = Invoke-BootstrapScriptProcess `
        -BootstrapScript $BootstrapScript `
        -MetadataUrl $metadataUrl `
        -InstallDirectory $InstallDirectory
    Write-Host $result.StdOut.TrimEnd()
    Write-Host 'Bootstrap test installation completed using the CI-built canonical Inno installer.'
} finally {
    Stop-BootstrapFixtureServer -Server $Server
    if (Test-Path -LiteralPath $FixtureRoot) {
        Remove-Item -LiteralPath $FixtureRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
