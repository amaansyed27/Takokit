[CmdletBinding()]
param(
    [string]$StorageRoot = (Join-Path $env:TEMP "takokit-all-model-smoke"),
    [string]$Tako = ".\target\release\tako.exe",
    [string]$EvidenceRoot = "$HOME\takokit-test-evidence",
    [switch]$IncludeWorkstation
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false
$script:ActiveProcess = $null

function Resolve-ExistingFile {
    param([string]$Path, [string]$Label)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "$Label does not exist: $Path"
    }
    return (Resolve-Path -LiteralPath $Path).Path
}

function ConvertTo-ProcessArgument {
    param([AllowEmptyString()][string]$Value)

    if ($Value.Length -eq 0) { return '""' }
    if ($Value -notmatch '[\s"]') { return $Value }

    $escaped = $Value -replace '(\\*)"', '$1$1\"'
    $escaped = $escaped -replace '(\\+)$', '$1$1'
    return '"' + $escaped + '"'
}

function Save-Results {
    param(
        [System.Collections.Generic.List[object]]$Results,
        [string]$RunRoot,
        [bool]$Complete = $false,
        [string]$FatalError = ""
    )

    @($Results) | ConvertTo-Json -Depth 6 |
        Out-File (Join-Path $RunRoot "results.json") -Encoding utf8
    @($Results) | Export-Csv (Join-Path $RunRoot "results.csv") -NoTypeInformation

    @($Results | Group-Object status | Sort-Object Name | ForEach-Object {
        [pscustomobject]@{ status = $_.Name; count = $_.Count }
    }) | ConvertTo-Json |
        Out-File (Join-Path $RunRoot "summary.json") -Encoding utf8

    [pscustomobject]@{
        state       = if ($Complete) { "complete" } else { "in-progress" }
        updated_at  = (Get-Date).ToString("o")
        completed   = $Results.Count
        fatal_error = $FatalError
    } | ConvertTo-Json |
        Out-File (Join-Path $RunRoot "progress.json") -Encoding utf8
}

function Get-PortOwnerIds {
    param([int]$Port)

    return @(
        Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty OwningProcess -Unique
    )
}

function Get-ProcessDescription {
    param([int]$ProcessId)

    $process = Get-CimInstance Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction SilentlyContinue
    if (-not $process) { return "PID $ProcessId" }
    return "PID $ProcessId ($($process.Name)) $($process.CommandLine)"
}

function Stop-ProcessTree {
    param([int]$ProcessId)

    if (-not (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)) { return }

    try {
        if ($IsWindows -or $env:OS -eq "Windows_NT") {
            & taskkill.exe /PID $ProcessId /T /F 2>&1 | Out-Null
        } else {
            Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
        }
    } catch {}
}

function Read-SmokeDaemonInfo {
    param([string]$Root)

    $path = Join-Path $Root "runtime\daemon.json"
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return $null }

    try {
        return Get-Content -LiteralPath $path -Raw | ConvertFrom-Json -ErrorAction Stop
    } catch {
        return $null
    }
}

function Stop-SmokeDaemon {
    param(
        [string]$Root,
        [string]$Executable,
        [switch]$Quiet
    )

    $owners = @(Get-PortOwnerIds 5050)
    if ($owners.Count -eq 0) {
        foreach ($name in @("daemon.json", "daemon.pid")) {
            Remove-Item -LiteralPath (Join-Path $Root "runtime\$name") -Force -ErrorAction SilentlyContinue
        }
        return
    }

    $info = Read-SmokeDaemonInfo $Root
    if (-not $info) {
        $details = $owners | ForEach-Object { Get-ProcessDescription $_ }
        throw "Port 5050 is occupied, but this smoke storage has no verifiable daemon identity: $($details -join '; ')"
    }

    $recordedRoot = [System.IO.Path]::GetFullPath("$($info.storage_root)")
    $daemonPid = [int]$info.pid
    if ($recordedRoot -ne $Root -or $owners -notcontains $daemonPid) {
        $details = $owners | ForEach-Object { Get-ProcessDescription $_ }
        throw "Port 5050 is not owned by the daemon recorded for this smoke storage: $($details -join '; ')"
    }

    if (-not $Quiet) {
        Write-Host "Stopping previous smoke daemon PID $daemonPid..." -ForegroundColor DarkGray
    }

    try {
        $null = @(& $Executable daemon stop 2>&1)
    } catch {}

    Start-Sleep -Milliseconds 600
    if (Get-Process -Id $daemonPid -ErrorAction SilentlyContinue) {
        Stop-ProcessTree $daemonPid
    }

    Start-Sleep -Milliseconds 400
    if ((Get-PortOwnerIds 5050).Count -gt 0) {
        throw "The verified smoke daemon did not release port 5050."
    }

    foreach ($name in @("daemon.json", "daemon.pid")) {
        Remove-Item -LiteralPath (Join-Path $Root "runtime\$name") -Force -ErrorAction SilentlyContinue
    }
}

function Get-LastUsefulLine {
    param([string[]]$Paths)

    foreach ($path in $Paths) {
        if (-not $path -or -not (Test-Path -LiteralPath $path -PathType Leaf)) { continue }

        $line = Get-Content -LiteralPath $path -Tail 30 -ErrorAction SilentlyContinue |
            ForEach-Object { "$_" -split "`r" } |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            Select-Object -Last 1

        if ($line) {
            $line = "$line".Trim()
            if ($line.Length -gt 160) { $line = $line.Substring(0, 157) + "..." }
            return $line
        }
    }
    return ""
}

function Get-TakokitLiveLogPaths {
    param([datetime]$Since)

    if (-not $env:TAKOKIT_HOME) { return @() }

    $patterns = @(
        (Join-Path $env:TAKOKIT_HOME "logs\*.log"),
        (Join-Path $env:TAKOKIT_HOME "runners\*\logs\*.log"),
        (Join-Path $env:TAKOKIT_HOME "runners\python-managed\adapters\*\*.log")
    )

    $files = foreach ($pattern in $patterns) {
        Get-ChildItem -Path $pattern -File -ErrorAction SilentlyContinue |
            Where-Object { $_.LastWriteTime -ge $Since.AddSeconds(-2) }
    }

    return @(
        $files |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 5 -ExpandProperty FullName
    )
}

function New-PulseBar {
    param([long]$ElapsedMilliseconds, [int]$Width = 28)

    $marker = "====>"
    $travel = [Math]::Max(1, $Width - $marker.Length)
    $cycle = $travel * 2
    $step = [int]([Math]::Floor($ElapsedMilliseconds / 180) % $cycle)
    $position = if ($step -gt $travel) { $cycle - $step } else { $step }
    $chars = [char[]]("." * $Width)

    for ($offset = 0; $offset -lt $marker.Length; $offset++) {
        $chars[$position + $offset] = $marker[$offset]
    }
    return "[" + (-join $chars) + "]"
}

function Write-ModelProgress {
    param(
        [string]$Model,
        [string]$Phase,
        [int]$ModelNumber,
        [int]$ModelCount,
        [System.Diagnostics.Stopwatch]$Watch,
        [datetime]$StartedAt
    )

    $overallPercent = [Math]::Min(99, [Math]::Floor((($ModelNumber - 1) / $ModelCount) * 100))
    Write-Progress -Id 1 -Activity "Takokit all-model prefetch" -Status "[$ModelNumber/$ModelCount] $Model - $Phase" -PercentComplete $overallPercent

    $latest = Get-LastUsefulLine @(Get-TakokitLiveLogPaths -Since $StartedAt)
    $elapsed = "{0:hh\:mm\:ss}" -f $Watch.Elapsed
    $status = "$(New-PulseBar -ElapsedMilliseconds $Watch.ElapsedMilliseconds) elapsed $elapsed"
    if ($latest) { $status += " | $latest" }

    Write-Progress -Id 2 -ParentId 1 -Activity "$Phase $Model" -Status $status -PercentComplete -1
}

function Test-TakokitFailureText {
    param([string]$Text)

    if ([string]::IsNullOrWhiteSpace($Text)) { return $false }
    return $Text -match '(?im)^\s*Failed after\b' -or
        $Text -match '(?im)^\s*Error:\s+' -or
        $Text -match '(?i)managed daemon will not take ownership'
}

function Invoke-TakoLive {
    param(
        [string]$Executable,
        [string[]]$Arguments,
        [string]$LogPath,
        [string]$Model,
        [string]$Phase,
        [int]$ModelNumber,
        [int]$ModelCount,
        [switch]$RequireJson,
        [switch]$RequireExecutable
    )

    $started = Get-Date
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $process = $null
    $exitCode = -1
    $stdout = ""
    $stderr = ""
    $json = $null

    try {
        $argumentLine = (@($Arguments | ForEach-Object { ConvertTo-ProcessArgument "$_" })) -join ' '
        $startInfo = New-Object System.Diagnostics.ProcessStartInfo
        $startInfo.FileName = $Executable
        $startInfo.Arguments = $argumentLine
        $startInfo.WorkingDirectory = (Get-Location).Path
        $startInfo.UseShellExecute = $false
        $startInfo.CreateNoWindow = $true
        $startInfo.RedirectStandardOutput = $true
        $startInfo.RedirectStandardError = $true

        $process = New-Object System.Diagnostics.Process
        $process.StartInfo = $startInfo
        if (-not $process.Start()) { throw "Failed to start Takokit process." }
        $script:ActiveProcess = $process

        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()

        while (-not $process.WaitForExit(500)) {
            Write-ModelProgress -Model $Model -Phase $Phase -ModelNumber $ModelNumber -ModelCount $ModelCount -Watch $watch -StartedAt $started
        }

        $process.WaitForExit()
        $stdoutTask.Wait()
        $stderrTask.Wait()
        $process.Refresh()
        $stdout = "$($stdoutTask.Result)"
        $stderr = "$($stderrTask.Result)"
        $exitCode = [int]$process.ExitCode
    } catch {
        $stderr = $_.Exception.Message
        $exitCode = -1
        throw
    } finally {
        $watch.Stop()
        Write-Progress -Id 2 -ParentId 1 -Activity "$Phase $Model" -Completed
        if ($process -and -not $process.HasExited) {
            Stop-ProcessTree $process.Id
        }
        if ($process) { $process.Dispose() }
        $script:ActiveProcess = $null
    }

    $combined = @(
        "=== command ===",
        "$Executable $($Arguments -join ' ')",
        "=== stdout ===",
        $stdout,
        "=== stderr ===",
        $stderr,
        "[reported exit code: $exitCode]"
    ) -join [Environment]::NewLine

    $semanticFailure = Test-TakokitFailureText $combined
    $validationFailure = $false
    if ($RequireJson -and -not $semanticFailure -and $exitCode -eq 0) {
        try {
            $json = $stdout | ConvertFrom-Json -ErrorAction Stop
            if ($RequireExecutable -and $json.executable -ne $true) {
                $validationFailure = $true
                $combined += [Environment]::NewLine + "[validation error: plan is not executable]"
            }
        } catch {
            $validationFailure = $true
            $combined += [Environment]::NewLine + "[validation error: expected valid JSON output]"
        }
    }

    $effectiveExitCode = if ($exitCode -ne 0) {
        $exitCode
    } elseif ($semanticFailure -or $validationFailure) {
        1
    } else {
        0
    }

    $combined += [Environment]::NewLine + "[effective exit code: $effectiveExitCode]"
    $combined | Set-Content -LiteralPath $LogPath -Encoding utf8

    $detail = if ($semanticFailure) {
        "Takokit emitted failure output despite reporting exit code $exitCode."
    } elseif ($validationFailure) {
        "Takokit readiness validation failed."
    } elseif (-not [string]::IsNullOrWhiteSpace($stderr)) {
        ($stderr -split "`r?`n" | Where-Object { $_.Trim() } | Select-Object -Last 1)
    } elseif (-not [string]::IsNullOrWhiteSpace($stdout)) {
        ($stdout -split "`r?`n" | Where-Object { $_.Trim() } | Select-Object -Last 1)
    } else {
        "exit code $effectiveExitCode"
    }

    return [pscustomobject]@{
        exit_code          = [int]$effectiveExitCode
        reported_exit_code = [int]$exitCode
        duration_ms        = [long]$watch.ElapsedMilliseconds
        stdout             = $stdout
        stderr             = $stderr
        json               = $json
        detail             = "$detail"
    }
}

function Test-ReadyInstallRecord {
    param([string]$Root, [string]$Model)

    $record = Join-Path $Root "manifests\installed-models\$Model.toml"
    $manifest = Join-Path $Root "manifests\models\$Model.toml"
    if (-not (Test-Path -LiteralPath $record -PathType Leaf)) { return $false }
    if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) { return $false }

    $source = Get-Content -LiteralPath $record -Raw
    return $source -match '(?im)^\s*status\s*=\s*"ready"\s*$'
}

$Models = @(
    [pscustomobject]@{ Id = "bark-small"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "canary"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "chatterbox"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "cosyvoice2"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "dia"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "distil-whisper-large-v3"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "f5-tts"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "fish-speech"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "gpt-sovits"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "kokoro"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "kyutai-tts-1.6b"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "mms-tts-eng"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "openvoice"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "parakeet"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "piper-lessac"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "qwen2-5-omni"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "qwen3-omni"; WorkstationOnly = $true },
    [pscustomobject]@{ Id = "qwen3-tts"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "qwen3-tts-0.6b-base"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "qwen3-tts-1.7b-base"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "qwen3-tts-1.7b-custom"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "qwen3-tts-1.7b-voice-design"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "rvc"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "sensevoice"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "voxtral"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "wav2vec2-base-960h"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "whisper-base"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "whisper-small"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "whisper-tiny"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "xtts-v2"; WorkstationOnly = $false },
    [pscustomobject]@{ Id = "yourtts"; WorkstationOnly = $false }
)

$Tako = Resolve-ExistingFile $Tako "Takokit executable"
$StorageRoot = [System.IO.Path]::GetFullPath([Environment]::ExpandEnvironmentVariables($StorageRoot))
$EvidenceRoot = [System.IO.Path]::GetFullPath([Environment]::ExpandEnvironmentVariables($EvidenceRoot))
New-Item -ItemType Directory -Force $StorageRoot, $EvidenceRoot | Out-Null

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$RunRoot = Join-Path $EvidenceRoot "pull-all-models-$stamp"
New-Item -ItemType Directory -Force $RunRoot | Out-Null
$Results = [System.Collections.Generic.List[object]]::new()
$FatalError = ""
$RunComplete = $false

[pscustomobject]@{
    started_at          = (Get-Date).ToString("o")
    takokit_executable  = $Tako
    takokit_home        = $StorageRoot
    model_count         = $Models.Count
    include_workstation = [bool]$IncludeWorkstation
} | ConvertTo-Json | Out-File (Join-Path $RunRoot "run.json") -Encoding utf8

Write-Host ""
Write-Host "Takokit sequential all-model prefetch" -ForegroundColor Cyan
Write-Host "Models:   $($Models.Count)"
Write-Host "Storage:  $StorageRoot"
Write-Host "Evidence: $RunRoot"
Write-Host ""
Write-Host "Interrupted downloads remain cached and are resumed, but never marked ready without a completed prefetch marker and ready install record." -ForegroundColor DarkGray
Write-Host "Ctrl+C terminates the active command and the verified smoke daemon tree." -ForegroundColor DarkGray

$PreviousTakokitHome = $env:TAKOKIT_HOME
$PreviousProgressPreference = $ProgressPreference
$PreviousProgressView = $null
$ProgressViewChanged = $false
$ProgressPreference = "Continue"
if (Get-Variable -Name PSStyle -ErrorAction SilentlyContinue) {
    $PreviousProgressView = $PSStyle.Progress.View
    $PSStyle.Progress.View = "Classic"
    $ProgressViewChanged = $true
}

try {
    $env:TAKOKIT_HOME = $StorageRoot
    Stop-SmokeDaemon -Root $StorageRoot -Executable $Tako -Quiet

    for ($index = 0; $index -lt $Models.Count; $index++) {
        $entry = $Models[$index]
        $model = $entry.Id
        $number = $index + 1

        Write-Host ""
        Write-Host "[$number/$($Models.Count)] $model" -ForegroundColor Cyan

        if ($entry.WorkstationOnly -and -not $IncludeWorkstation) {
            Write-Host "  blocked-hardware: use -IncludeWorkstation only on suitable hardware." -ForegroundColor Yellow
            $Results.Add([pscustomobject]@{
                model = $model; status = "blocked-hardware"; duration_ms = 0
                pull_log = ""; plan_log = ""; detail = "Workstation-only model omitted on this machine."
            }) | Out-Null
            Save-Results $Results $RunRoot
            continue
        }

        $safeModel = $model -replace '[^a-zA-Z0-9._-]', '_'
        $pullLog = Join-Path $RunRoot "$safeModel-pull.log"
        Write-Host "  Pulling..."
        Write-Host "  Log: $pullLog" -ForegroundColor DarkGray

        $pull = Invoke-TakoLive -Executable $Tako -Arguments @("pull", $model) -LogPath $pullLog -Model $model -Phase "Pulling" -ModelNumber $number -ModelCount $Models.Count
        if ($pull.exit_code -ne 0) {
            Write-Host "  failed: pull did not complete." -ForegroundColor Red
            Write-Host "  $($pull.detail)" -ForegroundColor Red
            $Results.Add([pscustomobject]@{
                model = $model; status = "failed"; duration_ms = $pull.duration_ms
                pull_log = $pullLog; plan_log = ""; detail = $pull.detail
            }) | Out-Null
            Save-Results $Results $RunRoot
            continue
        }

        $planLog = Join-Path $RunRoot "$safeModel-plan.log"
        Write-Host "  Verifying readiness..."
        $plan = Invoke-TakoLive -Executable $Tako -Arguments @("plan", $model, "--json") -LogPath $planLog -Model $model -Phase "Verifying readiness" -ModelNumber $number -ModelCount $Models.Count -RequireJson -RequireExecutable
        $duration = $pull.duration_ms + $plan.duration_ms

        if ($plan.exit_code -eq 0 -and (Test-ReadyInstallRecord -Root $StorageRoot -Model $model)) {
            $elapsed = [TimeSpan]::FromMilliseconds($duration)
            Write-Host ("  passed in {0:hh\:mm\:ss}" -f $elapsed) -ForegroundColor Green
            $status = "passed"
            $detail = "Pull, executable plan and ready install record verified."
        } else {
            Write-Host "  failed: no trustworthy ready installation was produced." -ForegroundColor Red
            $status = "failed"
            $detail = if ($plan.exit_code -ne 0) { $plan.detail } else { "Ready model record or manifest is missing." }
        }

        $Results.Add([pscustomobject]@{
            model = $model; status = $status; duration_ms = $duration
            pull_log = $pullLog; plan_log = $planLog; detail = $detail
        }) | Out-Null
        Save-Results $Results $RunRoot
    }

    Write-Host ""
    Write-Host "Installed model list from smoke storage" -ForegroundColor Cyan
    $installedLog = Join-Path $RunRoot "installed-models.log"
    $installed = Invoke-TakoLive -Executable $Tako -Arguments @("list") -LogPath $installedLog -Model "catalog" -Phase "Listing installed models" -ModelNumber $Models.Count -ModelCount $Models.Count
    if ($installed.exit_code -ne 0) { throw "tako list failed after prefetch. See $installedLog" }

    if (-not [string]::IsNullOrWhiteSpace($installed.stdout)) { Write-Host $installed.stdout }

    $missingFromList = @(
        $Results |
            Where-Object { $_.status -eq "passed" } |
            Where-Object { $installed.stdout -notmatch "(?m)(^|\s)$([regex]::Escape($_.model))(\s|$)" } |
            Select-Object -ExpandProperty model
    )
    if ($missingFromList.Count -gt 0) {
        throw "Models passed internal checks but are absent from tako list: $($missingFromList -join ', ')"
    }

    $RunComplete = $true
} catch {
    $FatalError = $_.Exception.Message
    Write-Host ""
    Write-Host "Prefetch aborted: $FatalError" -ForegroundColor Red
    Write-Host "No model/cache directories were deleted. Evidence: $RunRoot" -ForegroundColor DarkGray
} finally {
    if ($script:ActiveProcess -and -not $script:ActiveProcess.HasExited) {
        Stop-ProcessTree $script:ActiveProcess.Id
    }
    try {
        $env:TAKOKIT_HOME = $StorageRoot
        Stop-SmokeDaemon -Root $StorageRoot -Executable $Tako -Quiet
    } catch {
        if (-not $FatalError) { $FatalError = $_.Exception.Message }
        Write-Warning $_.Exception.Message
    }

    Write-Progress -Id 2 -ParentId 1 -Activity "Takokit model operation" -Completed
    Write-Progress -Id 1 -Activity "Takokit all-model prefetch" -Completed
    $env:TAKOKIT_HOME = $PreviousTakokitHome
    $ProgressPreference = $PreviousProgressPreference
    if ($ProgressViewChanged) { $PSStyle.Progress.View = $PreviousProgressView }
    Save-Results $Results $RunRoot -Complete $RunComplete -FatalError $FatalError
}

Write-Host ""
@($Results | Group-Object status | Sort-Object Name | ForEach-Object {
    [pscustomobject]@{ status = $_.Name; count = $_.Count }
}) | Format-Table -AutoSize
Write-Host "Evidence: $RunRoot"

if ($FatalError -or $Results.status -contains "failed") { exit 1 }
Write-Host "Next: .\scripts\run_takokit_all_smokes.ps1 -StorageRoot `"$StorageRoot`" -SkipPull" -ForegroundColor Cyan
exit 0
