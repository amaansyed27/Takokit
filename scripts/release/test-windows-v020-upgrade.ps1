[CmdletBinding()]
param([Parameter(Mandatory)][string]$OutputRoot)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$global:LASTEXITCODE = 0
$OutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)
$Manifest = Join-Path $OutputRoot 'release-manifest.json'
$Signature = Join-Path $OutputRoot 'release-manifest.sig'
$ExpectedArchiveSha = '99b2605abbeaed97cc297fc2d8a0aeebd23ffc86144120632fef72f83963ab38'
$ArchiveUrl = 'https://github.com/amaansyed27/Takokit/releases/download/v0.2.0/Takokit-v0.2.0-windows-x86_64.zip'
$TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('takokit-v020-upgrade-' + [Guid]::NewGuid().ToString('N'))
$Archive = Join-Path $TempRoot 'v0.2.0.zip'
$Extract = Join-Path $TempRoot 'extract'
$InstallRoot = Join-Path $TempRoot 'Takokit legacy install ü'
$TakokitHome = Join-Path $TempRoot 'home'
$OldHome = $env:TAKOKIT_HOME

function Get-Version([string]$Tako) {
    $global:LASTEXITCODE = 0
    $line = (& $Tako version | Select-Object -First 1)
    if ($LASTEXITCODE -ne 0) { throw "$Tako version failed" }
    return $line.Trim()
}

try {
    New-Item -ItemType Directory -Force -Path $TempRoot, $Extract, $TakokitHome | Out-Null
    Invoke-WebRequest -Uri $ArchiveUrl -OutFile $Archive
    $actual = (Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $ExpectedArchiveSha) { throw "Public v0.2.0 archive SHA-256 mismatch: $actual" }
    Expand-Archive -LiteralPath $Archive -DestinationPath $Extract
    $source = Get-ChildItem -LiteralPath $Extract -Directory | Where-Object { Test-Path (Join-Path $_.FullName 'bin\tako.exe') } | Select-Object -First 1
    if (-not $source) { throw 'Public v0.2.0 archive has no Takokit distribution root.' }
    Copy-Item -LiteralPath $source.FullName -Destination $InstallRoot -Recurse
    $distributionPath = Join-Path $InstallRoot 'distribution.json'
    $distribution = Get-Content -LiteralPath $distributionPath -Raw | ConvertFrom-Json
    $distribution.mode = 'installed'
    $distribution.install_root = $InstallRoot
    $distribution.update_manifest_url = $null
    [System.IO.File]::WriteAllText($distributionPath, (($distribution | ConvertTo-Json -Depth 8) + "`n"), [System.Text.UTF8Encoding]::new($false))
    $env:TAKOKIT_HOME = $TakokitHome
    Set-Content -LiteralPath (Join-Path $TakokitHome 'preserve.txt') -Value 'preserve' -NoNewline
    $oldTako = Join-Path $InstallRoot 'bin\tako.exe'
    if ((Get-Version $oldTako) -ne 'takokit 0.2.0') { throw 'Legacy fixture is not Takokit 0.2.0.' }
    $global:LASTEXITCODE = 0
    & $oldTako update apply --manifest $Manifest --signature $Signature --allow-test
    if ($LASTEXITCODE -ne 0) { throw 'v0.2.0 updater rejected the v0.3.0 compatibility manifest.' }
    $journal = Join-Path $TakokitHome 'runtime\update-journal.json'
    $deadline = (Get-Date).AddMinutes(3)
    do {
        Start-Sleep -Milliseconds 250
        if (Test-Path -LiteralPath $journal) {
            $state = Get-Content -LiteralPath $journal -Raw | ConvertFrom-Json
            if ($state.state -eq 'completed') { break }
            if ($state.state -eq 'rolled_back') { throw "Legacy upgrade rolled back: $($state.message)" }
        }
    } while ((Get-Date) -lt $deadline)
    if (-not (Test-Path -LiteralPath $journal)) { throw 'Legacy upgrade produced no journal.' }
    $state = Get-Content -LiteralPath $journal -Raw | ConvertFrom-Json
    if ($state.state -ne 'completed') { throw "Legacy upgrade did not complete: $($state.state)" }
    $newTako = Join-Path $InstallRoot 'bin\tako.exe'
    if ((Get-Version $newTako) -ne 'takokit 0.3.0') { throw 'Legacy upgrade did not install Takokit 0.3.0.' }
    if (-not (Test-Path -LiteralPath (Join-Path $TakokitHome 'preserve.txt'))) { throw 'Legacy upgrade removed user data.' }
    [ordered]@{
        from = '0.2.0'
        to = '0.3.0'
        manifest = 'release-manifest.json'
        signing_key_id = 'takokit-test-fixture-v1'
        completed = $true
        user_data_preserved = $true
    } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $OutputRoot 'v020-upgrade-report.json') -Encoding utf8
} finally {
    $env:TAKOKIT_HOME = $OldHome
    $uninstaller = Join-Path $InstallRoot 'unins000.exe'
    if (Test-Path -LiteralPath $uninstaller) {
        Start-Process -FilePath $uninstaller -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART') -Wait | Out-Null
    }
    if (Test-Path -LiteralPath $TempRoot) { Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue }
}
