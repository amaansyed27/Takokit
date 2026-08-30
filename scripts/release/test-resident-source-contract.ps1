$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

$Cargo = Get-Content -LiteralPath (Join-Path $Root 'apps\cli\Cargo.toml') -Raw
$Installer = Get-Content -LiteralPath (Join-Path $Root 'packaging\windows\Takokit.iss') -Raw
$Resident = Get-Content -LiteralPath (Join-Path $Root 'apps\cli\src\resident.rs') -Raw
$Application = Get-Content -LiteralPath (Join-Path $Root 'apps\cli\src\bin\takokit-app.rs') -Raw
$ApplicationLifecycle = Get-Content -LiteralPath (Join-Path $Root 'apps\cli\src\resident\application.rs') -Raw
$Build = Get-Content -LiteralPath (Join-Path $Root 'scripts\release\build-windows.ps1') -Raw

Assert-True ($Cargo -notmatch 'name\s*=\s*"takokit-tray"') 'Cargo still declares a takokit-tray binary.'
Assert-True ($Build -notmatch "--bin'\s+'takokit-tray") 'Windows packaging still builds takokit-tray.'
Assert-True ($Cargo -match 'name\s*=\s*"Takokit"') 'Cargo does not declare the main Takokit Windows application.'
Assert-True ($Cargo -match 'name\s*=\s*"takokit-server"') 'Cargo does not declare the distinct internal server runtime.'
Assert-True ($Application -match 'windows_subsystem\s*=\s*"windows"') 'Takokit application is not a Windows GUI-subsystem executable.'
Assert-True ($ApplicationLifecycle -match 'creation_flags\(0x0000_0008 \| 0x0000_0200\)') 'CLI resident launch is not detached from its invoking console/pipeline.'
Assert-True ($ApplicationLifecycle -match 'windows_handle_inheritance::suppress\(\)') 'CLI resident launch can inherit and hold its invoking pipeline handles.'
Assert-True ($Installer -match 'Filename:\s*"\{app\}\\bin\\Takokit\.exe"') 'Primary Takokit shortcut does not launch the main Windows application.'
Assert-True ($Installer -notmatch 'Name:\s*"\{group\}\\Takokit Tray"') 'Installer still creates a separate Takokit Tray shortcut.'
Assert-True ($Installer -match 'ValueName:\s*"Takokit"') 'Installer does not own the canonical Takokit startup value.'
Assert-True ($Installer -match '\[InstallDelete\][\s\S]*takokit-tray\.exe') 'Upgrade does not remove the legacy tray executable.'
Assert-True ($Resident -match 'check_update_async\(hwnd, true\)') 'Resident startup does not schedule an asynchronous update check.'
Assert-True ($Resident -match 'UPDATE_INTERVAL_MS') 'Resident mode has no periodic update cadence.'
Assert-True ($Resident -match 'NIF_INFO') 'Resident mode has no native update notification.'
Assert-True ($Resident -match 'stop_verified_server') 'Quit does not use the identity-verified unified server shutdown.'
Assert-True ($Resident -match 'TaskbarCreated') 'Resident does not restore its notification icon after Explorer recreation.'
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
    windows_application = $true
    cli_launch_detached = $true
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
