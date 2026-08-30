[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$TakoExe,
    [string]$OutputRoot = (Join-Path $PSScriptRoot '..\..\dist\resident-acceptance'),
    [switch]$AssertNoSibling,
    [int]$Port = 5167
)

$ErrorActionPreference = 'Stop'

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function Start-Hidden([string]$FilePath, [string[]]$Arguments) {
    Start-Process -FilePath $FilePath -ArgumentList $Arguments -PassThru -WindowStyle Hidden
}

function Wait-Until([scriptblock]$Condition, [int]$Seconds, [string]$Failure) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        if (& $Condition) { return }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw $Failure
}

function Test-Port([int]$TargetPort) {
    $client = [Net.Sockets.TcpClient]::new()
    try {
        $result = $client.BeginConnect('127.0.0.1', $TargetPort, $null, $null)
        if (-not $result.AsyncWaitHandle.WaitOne(250)) { return $false }
        $client.EndConnect($result)
        return $true
    } catch {
        return $false
    } finally {
        $client.Dispose()
    }
}

$TakoExe = (Resolve-Path -LiteralPath $TakoExe).Path
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
$RunRoot = Join-Path $OutputRoot ([guid]::NewGuid().ToString('N'))
$HomeRoot = Join-Path $RunRoot 'home'
New-Item -ItemType Directory -Force -Path $HomeRoot | Out-Null
$InfoPath = Join-Path $HomeRoot 'runtime\daemon.json'
$OriginalHome = $env:TAKOKIT_HOME
$OriginalPort = $env:TAKOKIT_PORT
$OwnedProcesses = [Collections.Generic.List[Diagnostics.Process]]::new()

$Report = [ordered]@{
    integrated_executable = $false
    no_separate_tray_executable = $false
    single_instance = $false
    startup_ensures_managed_server = $false
    server_death_keeps_resident_alive = $false
    start_after_server_death = $false
    quit_stops_owned_server = $false
    port_released_after_quit = $false
    foreground_server_not_killed = $false
    foreign_port_process_not_killed = $false
}

try {
    $env:TAKOKIT_HOME = $HomeRoot
    $env:TAKOKIT_PORT = [string]$Port
    Assert-True ($TakoExe.EndsWith('\tako.exe', [StringComparison]::OrdinalIgnoreCase)) 'Resident mode is not hosted by tako.exe.'
    if ($AssertNoSibling) {
        Assert-True (-not (Test-Path -LiteralPath (Join-Path (Split-Path $TakoExe) 'takokit-tray.exe'))) 'A separate takokit-tray.exe is present.'
    }
    $Report.integrated_executable = $true
    $Report.no_separate_tray_executable = -not (Test-Path -LiteralPath (Join-Path (Split-Path $TakoExe) 'takokit-tray.exe'))

    $Resident = Start-Hidden $TakoExe @('--resident')
    $OwnedProcesses.Add($Resident)
    Wait-Until { Test-Path -LiteralPath $InfoPath } 15 'Resident startup did not publish a managed server identity.'
    Wait-Until { Test-Port $Port } 10 'Resident startup did not open the configured port.'
    $Report.startup_ensures_managed_server = $true

    $Second = Start-Hidden $TakoExe @('--resident')
    $OwnedProcesses.Add($Second)
    Assert-True ($Second.WaitForExit(5000)) 'A second resident Takokit instance remained running.'
    Assert-True (-not $Resident.HasExited) 'The primary resident instance exited unexpectedly.'
    $Report.single_instance = $true

    $Stop = Start-Hidden $TakoExe @('server', 'stop')
    $OwnedProcesses.Add($Stop)
    Assert-True ($Stop.WaitForExit(15000)) 'Managed server stop timed out.'
    Wait-Until { -not (Test-Port $Port) } 10 'Managed server port remained open after simulated server death.'
    Assert-True (-not $Resident.HasExited) 'Resident exited when its managed server stopped.'
    $Report.server_death_keeps_resident_alive = $true

    $StartAction = Start-Hidden $TakoExe @('--resident', '--resident-action', 'start')
    $OwnedProcesses.Add($StartAction)
    Assert-True ($StartAction.WaitForExit(5000)) 'Start Server resident action did not return.'
    Wait-Until { (Test-Path -LiteralPath $InfoPath) -and (Test-Port $Port) } 15 'Start Server did not restore the managed server.'
    Start-Sleep -Milliseconds 500
    $Report.start_after_server_death = $true

    $Quit = Start-Hidden $TakoExe @('--resident', '--resident-quit')
    $OwnedProcesses.Add($Quit)
    Assert-True ($Quit.WaitForExit(5000)) 'Resident quit request did not return.'
    Assert-True ($Resident.WaitForExit(15000)) 'Resident did not exit after Quit Takokit.'
    Wait-Until { -not (Test-Port $Port) } 10 'Resident-owned server port remained open after Quit Takokit.'
    $Report.quit_stops_owned_server = $true
    $Report.port_released_after_quit = $true

    $Foreground = Start-Hidden $TakoExe @('serve', '--port', [string]$Port)
    $OwnedProcesses.Add($Foreground)
    Wait-Until { Test-Port $Port } 15 'Foreground tako serve did not start.'
    $AttachedResident = Start-Hidden $TakoExe @('--resident')
    $OwnedProcesses.Add($AttachedResident)
    Start-Sleep -Milliseconds 750
    Assert-True (-not $AttachedResident.HasExited) 'Resident did not remain alive beside foreground tako serve.'
    $AttachedQuit = Start-Hidden $TakoExe @('--resident', '--resident-quit')
    $OwnedProcesses.Add($AttachedQuit)
    Assert-True ($AttachedQuit.WaitForExit(5000)) 'Attached resident quit request did not return.'
    Assert-True ($AttachedResident.WaitForExit(10000)) 'Attached resident did not exit.'
    Assert-True (-not $Foreground.HasExited) 'Quit Takokit killed developer-owned foreground tako serve.'
    $Report.foreground_server_not_killed = $true
    Stop-Process -Id $Foreground.Id
    $Foreground.WaitForExit(5000) | Out-Null
    Wait-Until { -not (Test-Port $Port) } 10 'Foreground test server did not release its port.'

    $Python = (Get-Command python -ErrorAction Stop).Source
    $Foreign = Start-Hidden $Python @('-m', 'http.server', [string]$Port, '--bind', '127.0.0.1')
    $OwnedProcesses.Add($Foreign)
    Wait-Until { Test-Port $Port } 10 'Foreign port fixture did not start.'
    $ForeignResident = Start-Hidden $TakoExe @('--resident')
    $OwnedProcesses.Add($ForeignResident)
    Start-Sleep -Milliseconds 750
    $ForeignQuit = Start-Hidden $TakoExe @('--resident', '--resident-quit')
    $OwnedProcesses.Add($ForeignQuit)
    Assert-True ($ForeignQuit.WaitForExit(5000)) 'Foreign-port resident quit request did not return.'
    Assert-True ($ForeignResident.WaitForExit(10000)) 'Foreign-port resident did not exit.'
    Assert-True (-not $Foreign.HasExited) 'Quit Takokit killed an unrelated foreign port process.'
    $Report.foreign_port_process_not_killed = $true

    $ReportPath = Join-Path $RunRoot 'resident-acceptance.json'
    [IO.File]::WriteAllText($ReportPath, (($Report | ConvertTo-Json) + "`n"), [Text.UTF8Encoding]::new($false))
    Write-Host ($Report | ConvertTo-Json)
} finally {
    foreach ($Process in $OwnedProcesses) {
        if ($null -ne $Process -and -not $Process.HasExited) {
            Stop-Process -Id $Process.Id -ErrorAction SilentlyContinue
        }
    }
    if ($null -eq $OriginalHome) { Remove-Item Env:TAKOKIT_HOME -ErrorAction SilentlyContinue } else { $env:TAKOKIT_HOME = $OriginalHome }
    if ($null -eq $OriginalPort) { Remove-Item Env:TAKOKIT_PORT -ErrorAction SilentlyContinue } else { $env:TAKOKIT_PORT = $OriginalPort }
}
