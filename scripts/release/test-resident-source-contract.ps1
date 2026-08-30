$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

$Cargo = Get-Content -LiteralPath (Join-Path $Root 'apps\cli\Cargo.toml') -Raw
$Installer = Get-Content -LiteralPath (Join-Path $Root 'packaging\windows\Takokit.iss') -Raw
$Resident = Get-Content -LiteralPath (Join-Path $Root 'apps\cli\src\resident.rs') -Raw
$Build = Get-Content -LiteralPath (Join-Path $Root 'scripts\release\build-windows.ps1') -Raw

Assert-True ($Cargo -notmatch 'name\s*=\s*"takokit-tray"') 'Cargo still declares a takokit-tray binary.'
Assert-True ($Build -notmatch "--bin'\s+'takokit-tray") 'Windows packaging still builds takokit-tray.'
Assert-True ($Installer -match 'Filename:\s*"\{app\}\\bin\\tako\.exe"; Parameters:\s*"--resident"') 'Primary Takokit shortcut does not launch integrated resident mode.'
Assert-True ($Installer -notmatch 'Name:\s*"\{group\}\\Takokit Tray"') 'Installer still creates a separate Takokit Tray shortcut.'
Assert-True ($Installer -match 'ValueName:\s*"Takokit"') 'Installer does not own the canonical Takokit startup value.'
Assert-True ($Installer -match '\[InstallDelete\][\s\S]*takokit-tray\.exe') 'Upgrade does not remove the legacy tray executable.'
Assert-True ($Resident -match 'check_update_async\(hwnd, true\)') 'Resident startup does not schedule an asynchronous update check.'
Assert-True ($Resident -match 'UPDATE_INTERVAL_MS') 'Resident mode has no periodic update cadence.'
Assert-True ($Resident -match 'NIF_INFO') 'Resident mode has no native update notification.'
Assert-True ($Resident -match 'should_stop_owned') 'Quit does not enforce explicit managed-server ownership.'
Assert-True ($Resident -match 'config\.local_base_url\(\)') 'Copy API URL does not use the active runtime configuration.'

$IconPath = Join-Path $Root 'assets\favicon\favicon.ico'
$Bytes = [IO.File]::ReadAllBytes($IconPath)
$Count = [BitConverter]::ToUInt16($Bytes, 4)
$Sizes = @()
for ($Index = 0; $Index -lt $Count; $Index++) {
    $Offset = 6 + (16 * $Index)
    $Width = if ($Bytes[$Offset] -eq 0) { 256 } else { [int]$Bytes[$Offset] }
    $Height = if ($Bytes[$Offset + 1] -eq 0) { 256 } else { [int]$Bytes[$Offset + 1] }
    if ($Width -eq $Height) { $Sizes += $Width }
}
foreach ($Required in @(16, 20, 24, 32, 48, 256)) {
    Assert-True ($Sizes -contains $Required) "Takokit ICO is missing the ${Required}x${Required} frame."
}

[ordered]@{
    integrated_resident = $true
    separate_tray_binary_removed = $true
    primary_shortcut_resident = $true
    startup_registration_resident = $true
    legacy_upgrade_cleanup = $true
    asynchronous_update_check = $true
    periodic_update_check = $true
    native_update_notification = $true
    owned_shutdown = $true
    configured_api_url = $true
    icon_sizes = $Sizes
} | ConvertTo-Json
