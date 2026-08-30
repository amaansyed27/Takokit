[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$TakoExe,
    [string]$ApplicationExe,
    [string]$OutputRoot = (Join-Path $PSScriptRoot '..\..\dist\resident-acceptance'),
    [switch]$AssertNoLegacyTray,
    [int]$Port = 5167
)

$ErrorActionPreference = 'Stop'
function Assert-True([bool]$Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
function Start-Background([string]$FilePath, [string[]]$Arguments, [string]$Stdout = $null, [string]$Stderr = $null) {
    $params = @{ FilePath = $FilePath; ArgumentList = $Arguments; PassThru = $true; WindowStyle = 'Hidden' }
    if ($Stdout) { $params.RedirectStandardOutput = $Stdout }
    if ($Stderr) { $params.RedirectStandardError = $Stderr }
    Start-Process @params
}
function Wait-Until([scriptblock]$Condition, [int]$Seconds, [string]$Failure) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    do { if (& $Condition) { return }; Start-Sleep -Milliseconds 100 } while ([DateTime]::UtcNow -lt $deadline)
    throw $Failure
}
function Test-Port([int]$TargetPort) {
    $client = [Net.Sockets.TcpClient]::new()
    try {
        $result = $client.BeginConnect('127.0.0.1', $TargetPort, $null, $null)
        if (-not $result.AsyncWaitHandle.WaitOne(250)) { return $false }
        $client.EndConnect($result); return $true
    } catch { return $false } finally { $client.Dispose() }
}

$TakoExe = (Resolve-Path -LiteralPath $TakoExe).Path
if (-not $ApplicationExe) { $ApplicationExe = Join-Path (Split-Path $TakoExe) 'Takokit.exe' }
$ApplicationExe = (Resolve-Path -LiteralPath $ApplicationExe).Path
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
$RunRoot = Join-Path $OutputRoot ([guid]::NewGuid().ToString('N'))
$HomeRoot = Join-Path $RunRoot 'home'
New-Item -ItemType Directory -Force -Path $HomeRoot | Out-Null
$InfoPath = Join-Path $HomeRoot 'runtime\daemon.json'
$OriginalHome = $env:TAKOKIT_HOME
$OriginalPort = $env:TAKOKIT_PORT
$OwnedProcesses = [Collections.Generic.List[Diagnostics.Process]]::new()
$Report = [ordered]@{
    windows_application = $false; no_legacy_tray_executable = $false; single_resident_instance = $false
    startup_ensures_managed_server = $false; serve_detects_running_server = $false; top_level_stop = $false
    server_death_keeps_resident_alive = $false; start_after_server_death = $false
    quit_stops_verified_server = $false; port_released_after_quit = $false
    foreground_stopped_by_top_level_stop = $false; foreign_port_process_not_killed = $false
}

try {
    $env:TAKOKIT_HOME = $HomeRoot
    $env:TAKOKIT_PORT = [string]$Port
    Assert-True ($ApplicationExe.EndsWith('\Takokit.exe', [StringComparison]::OrdinalIgnoreCase)) 'Resident is not hosted by Takokit.exe.'
    if ($AssertNoLegacyTray) { Assert-True (-not (Test-Path (Join-Path (Split-Path $TakoExe) 'takokit-tray.exe'))) 'Legacy takokit-tray.exe is present.' }
    $Report.windows_application = $true
    $Report.no_legacy_tray_executable = -not (Test-Path (Join-Path (Split-Path $TakoExe) 'takokit-tray.exe'))

    $Resident = Start-Background $ApplicationExe @('--background'); $OwnedProcesses.Add($Resident)
    Wait-Until { Test-Path $InfoPath } 15 'Application startup did not publish a managed server identity.'
    Wait-Until { Test-Port $Port } 10 'Application startup did not open the configured port.'
    $Report.startup_ensures_managed_server = $true
    $Second = Start-Background $ApplicationExe @('--background'); $OwnedProcesses.Add($Second)
    Assert-True ($Second.WaitForExit(5000)) 'A second resident instance remained running.'
    Assert-True (-not $Resident.HasExited) 'Primary resident exited unexpectedly.'
    $Report.single_resident_instance = $true

    $AlreadyOut = Join-Path $RunRoot 'already.out'; $AlreadyErr = Join-Path $RunRoot 'already.err'
    $Already = Start-Background $TakoExe @('serve', '--port', [string]$Port) $AlreadyOut $AlreadyErr; $OwnedProcesses.Add($Already)
    Assert-True ($Already.WaitForExit(10000)) 'tako serve did not return for an existing server.'
    $AlreadyText = ((Get-Content $AlreadyOut -Raw) + (Get-Content $AlreadyErr -Raw))
    Assert-True ($Already.ExitCode -eq 0) "tako serve treated the existing server as an error: $AlreadyText"
    Assert-True ($AlreadyText -match 'already running') 'tako serve did not report the existing server.'
    Assert-True ($AlreadyText -notmatch '10048|failed to bind|socket address') 'tako serve leaked a raw bind error.'
    $Report.serve_detects_running_server = $true

    $Stop = Start-Background $TakoExe @('stop'); $OwnedProcesses.Add($Stop)
    Assert-True ($Stop.WaitForExit(15000)) 'tako stop timed out.'; Assert-True ($Stop.ExitCode -eq 0) 'tako stop failed.'
    Wait-Until { -not (Test-Port $Port) } 10 'tako stop did not release the managed port.'
    Assert-True (-not $Resident.HasExited) 'Resident exited when its server stopped.'
    $Report.top_level_stop = $true; $Report.server_death_keeps_resident_alive = $true
    $StartAction = Start-Background $ApplicationExe @('--action', 'start'); $OwnedProcesses.Add($StartAction)
    Assert-True ($StartAction.WaitForExit(5000)) 'Start Server action did not return.'
    Wait-Until { (Test-Path $InfoPath) -and (Test-Port $Port) } 15 'Start Server did not restore the server.'
    $Report.start_after_server_death = $true

    $Quit = Start-Background $ApplicationExe @('--quit'); $OwnedProcesses.Add($Quit)
    Assert-True ($Quit.WaitForExit(5000)) 'Quit request did not return.'
    Assert-True ($Resident.WaitForExit(15000)) 'Resident did not exit after Quit Takokit.'
    Wait-Until { -not (Test-Port $Port) } 10 'Server port remained open after Quit Takokit.'
    $Report.quit_stops_verified_server = $true; $Report.port_released_after_quit = $true

    $Foreground = Start-Background $TakoExe @('serve', '--port', [string]$Port); $OwnedProcesses.Add($Foreground)
    Wait-Until { Test-Port $Port } 15 'Foreground tako serve did not start.'
    $AttachedResident = Start-Background $ApplicationExe @('--background'); $OwnedProcesses.Add($AttachedResident)
    Start-Sleep -Milliseconds 750; Assert-True (-not $AttachedResident.HasExited) 'Resident did not coexist with foreground serve.'
    $StopForeground = Start-Background $TakoExe @('stop'); $OwnedProcesses.Add($StopForeground)
    Assert-True ($StopForeground.WaitForExit(15000)) 'tako stop did not return for foreground serve.'
    Assert-True ($Foreground.WaitForExit(15000)) 'Foreground serve did not exit after tako stop.'
    Wait-Until { -not (Test-Port $Port) } 10 'Foreground port remained open.'
    Assert-True (-not $AttachedResident.HasExited) 'Resident exited with foreground server.'
    $Report.foreground_stopped_by_top_level_stop = $true
    $AttachedQuit = Start-Background $ApplicationExe @('--quit'); $OwnedProcesses.Add($AttachedQuit)
    $AttachedQuit.WaitForExit(5000) | Out-Null; $AttachedResident.WaitForExit(10000) | Out-Null

    $Python = (Get-Command python -ErrorAction Stop).Source
    $Foreign = Start-Background $Python @('-m', 'http.server', [string]$Port, '--bind', '127.0.0.1'); $OwnedProcesses.Add($Foreign)
    Wait-Until { Test-Port $Port } 10 'Foreign port fixture did not start.'
    $ForeignResident = Start-Background $ApplicationExe @('--background'); $OwnedProcesses.Add($ForeignResident)
    Start-Sleep -Milliseconds 750
    $ForeignQuit = Start-Background $ApplicationExe @('--quit'); $OwnedProcesses.Add($ForeignQuit)
    $ForeignQuit.WaitForExit(5000) | Out-Null
    Assert-True ($ForeignResident.WaitForExit(10000)) 'Resident did not exit beside a foreign process.'
    Assert-True (-not $Foreign.HasExited) 'Quit Takokit killed an unrelated process.'
    $Report.foreign_port_process_not_killed = $true

    $ReportPath = Join-Path $RunRoot 'resident-acceptance.json'
    [IO.File]::WriteAllText($ReportPath, (($Report | ConvertTo-Json) + "`n"), [Text.UTF8Encoding]::new($false))
    Write-Host ($Report | ConvertTo-Json)
} finally {
    foreach ($Process in $OwnedProcesses) { if ($null -ne $Process -and -not $Process.HasExited) { Stop-Process -Id $Process.Id -ErrorAction SilentlyContinue } }
    if ($null -eq $OriginalHome) { Remove-Item Env:TAKOKIT_HOME -ErrorAction SilentlyContinue } else { $env:TAKOKIT_HOME = $OriginalHome }
    if ($null -eq $OriginalPort) { Remove-Item Env:TAKOKIT_PORT -ErrorAction SilentlyContinue } else { $env:TAKOKIT_PORT = $OriginalPort }
}
