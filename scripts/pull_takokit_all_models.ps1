[CmdletBinding()]
param(
    [string]$StorageRoot = (Join-Path $env:TEMP "takokit-all-model-smoke"),
    [string]$Tako = ".\target\release\tako.exe",
    [string]$EvidenceRoot = "$HOME\takokit-test-evidence",
    [switch]$IncludeWorkstation
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false

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

    @($Results) | ConvertTo-Json -Depth 5 |
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

function Get-LastUsefulLine {
    param([string[]]$Paths)

    foreach ($path in $Paths) {
        if (-not $path -or -not (Test-Path -LiteralPath $path -PathType Leaf)) {
            continue
        }

        $line = Get-Content -LiteralPath $path -Tail 30 -ErrorAction SilentlyContinue |
            ForEach-Object { "$_" -split "`r" } |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            Select-Object -Last 1

        if ($line) {
            $line = "$line".Trim()
            if ($line.Length -gt 160) {
                $line = $line.Substring(0, 157) + "..."
            }
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

    $overallPercent = [Math]::Min(
        99,
        [Math]::Floor((($ModelNumber - 1) / $ModelCount) * 100)
    )

    Write-Progress `
        -Id 1 `
        -Activity "Takokit all-model prefetch" `
        -Status "[$ModelNumber/$ModelCount] $Model - $Phase" `
        -PercentComplete $overallPercent

    $liveLogs = @(Get-TakokitLiveLogPaths -Since $StartedAt)
    $latest = Get-LastUsefulLine $liveLogs
    $elapsed = "{0:hh\:mm\:ss}" -f $Watch.Elapsed
    $status = "$(New-PulseBar -ElapsedMilliseconds $Watch.ElapsedMilliseconds) elapsed $elapsed"
    if ($latest) { $status += " | $latest" }

    Write-Progress `
        -Id 2 `
        -ParentId 1 `
        -Activity "$Phase $Model" `
        -Status $status `
        -PercentComplete -1
}

function Test-TakokitFailureText {
    param([string]$Text)

    if ([string]::IsNullOrWhiteSpace($Text)) { return $false }

    return $Text -match '(?im)^\s*Failed after\b' -or
        $Text -match '(?im)^\s*Error:\s+' -or
        $Text -match '(?i)port\s+5050\s+is\s+occupied' -or
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
        [switch]$RequireJson
    )

    $started = Get-Date
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $process = $null
    $exitCode = -1
    $stdout = ""
    $stderr = ""

    try {
        $argumentLine = (@($Arguments | ForEach-Object {
            ConvertTo-ProcessArgument "$_"
        })) -join ' '

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

        if (-not $process.Start()) {
            throw "Failed to start Takokit process."
        }

        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()

        while (-not $process.WaitForExit(500)) {
            Write-ModelProgress `
                -Model $Model `
                -Phase $Phase `
                -ModelNumber $ModelNumber `
                -ModelCount $ModelCount `
                -Watch $watch `
                -StartedAt $started
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
    } finally {
        $watch.Stop()
        Write-Progress -Id 2 -ParentId 1 -Activity "$Phase $Model" -Completed
        if ($process) { $process.Dispose() }
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
    $jsonFailure = $false

    if ($RequireJson -and -not $semanticFailure -and $exitCode -eq 0) {
        try {
            $null = $stdout | ConvertFrom-Json -ErrorAction Stop
        } catch {
            $jsonFailure = $true
            $combined += [Environment]::NewLine + "[validation error: expected valid JSON output]"
        }
    }

    $effectiveExitCode = if ($exitCode -ne 0) {
        $exitCode
    } elseif ($semanticFailure -or $jsonFailure) {
        1
    } else {
        0
    }

    $combined += [Environment]::NewLine + "[effective exit code: $effectiveExitCode]"
    $combined | Set-Content -LiteralPath $LogPath -Encoding utf8

    $detail = if ($semanticFailure) {
        "Takokit emitted failure output despite reporting exit code $exitCode."
    } elseif ($jsonFailure) {
        "Takokit did not return valid JSON."
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
        detail             = "$detail"
    }
}

function Assert-PortAvailable {
    param([int]$Port)

    $listeners = @(
        Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
    )

    if ($listeners.Count -eq 0) { return }

    $owners = @($listeners | Select-Object -ExpandProperty OwningProcess -Unique)
    $details = foreach ($processId in $owners) {
        $process = Get-Process -Id $processId -ErrorAction SilentlyContinue
        if ($process) {
            "PID $processId ($($process.ProcessName))"
        } else {
            "PID $processId"
        }
    }

    throw "Port $Port is already occupied by $($details -join ', '). Stop that process before starting the isolated prefetch run."
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
$StorageRoot = [System.IO.Path]::GetFullPath(
    [Environment]::ExpandEnvironmentVariables($StorageRoot)
)
$EvidenceRoot = [System.IO.Path]::GetFullPath(
    [Environment]::ExpandEnvironmentVariables($EvidenceRoot)
)
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
Write-Host "The run fails closed on daemon conflicts and invalid Takokit output." -ForegroundColor DarkGray
Write-Host "Already verified models are reused when the same storage root is valid." -ForegroundColor DarkGray

$PreviousTakokitHome = $env:TAKOKIT_HOME
$PreviousProgressPreference = $ProgressPreference
$ProgressPreference = "Continue"
$PreviousProgressView = $null
$ProgressViewChanged = $false
$ProgressStyleVariable = Get-Variable -Name PSStyle -ErrorAction SilentlyContinue
if ($ProgressStyleVariable) {
    $PreviousProgressView = $PSStyle.Progress.View
    $PSStyle.Progress.View = "Classic"
    $ProgressViewChanged = $true
}

try {
    Assert-PortAvailable -Port 5050
    $env:TAKOKIT_HOME = $StorageRoot

    for ($index = 0; $index -lt $Models.Count; $index++) {
        $entry = $Models[$index]
        $model = $entry.Id
        $number = $index + 1

        Write-Host ""
        Write-Host "[$number/$($Models.Count)] $model" -ForegroundColor Cyan

        if ($entry.WorkstationOnly -and -not $IncludeWorkstation) {
            Write-Host "  blocked-hardware: use -IncludeWorkstation only on suitable hardware." -ForegroundColor Yellow
            $Results.Add([pscustomobject]@{
                model       = $model
                status      = "blocked-hardware"
                duration_ms = 0
                pull_log    = ""
                plan_log    = ""
                detail      = "Workstation-only model omitted on the primary 8 GB test machine."
            }) | Out-Null
            Save-Results $Results $RunRoot
            continue
        }

        $safeModel = $model -replace '[^a-zA-Z0-9._-]', '_'
        $pullLog = Join-Path $RunRoot "$safeModel-pull.log"
        Write-Host "  Pulling..."
        Write-Host "  Log: $pullLog" -ForegroundColor DarkGray

        $pull = Invoke-TakoLive `
            -Executable $Tako `
            -Arguments @("pull", $model) `
            -LogPath $pullLog `
            -Model $model `
            -Phase "Pulling" `
            -ModelNumber $number `
            -ModelCount $Models.Count

        if ($pull.exit_code -ne 0) {
            Write-Host "  failed: pull did not complete." -ForegroundColor Red
            Write-Host "  $($pull.detail)" -ForegroundColor Red
            $Results.Add([pscustomobject]@{
                model       = $model
                status      = "failed"
                duration_ms = $pull.duration_ms
                pull_log    = $pullLog
                plan_log    = ""
                detail      = $pull.detail
            }) | Out-Null
            Save-Results $Results $RunRoot
            continue
        }

        $planLog = Join-Path $RunRoot "$safeModel-plan.log"
        Write-Host "  Verifying readiness..."

        $plan = Invoke-TakoLive `
            -Executable $Tako `
            -Arguments @("plan", $model, "--json") `
            -LogPath $planLog `
            -Model $model `
            -Phase "Verifying readiness" `
            -ModelNumber $number `
            -ModelCount $Models.Count `
            -RequireJson

        $duration = $pull.duration_ms + $plan.duration_ms
        if ($plan.exit_code -eq 0) {
            $elapsed = [TimeSpan]::FromMilliseconds($duration)
            Write-Host ("  passed in {0:hh\:mm\:ss}" -f $elapsed) -ForegroundColor Green
            $status = "passed"
            $detail = "Pull and readiness plan succeeded."
        } else {
            Write-Host "  failed: readiness verification did not complete." -ForegroundColor Red
            Write-Host "  $($plan.detail)" -ForegroundColor Red
            $status = "failed"
            $detail = $plan.detail
        }

        $Results.Add([pscustomobject]@{
            model       = $model
            status      = $status
            duration_ms = $duration
            pull_log    = $pullLog
            plan_log    = $planLog
            detail      = $detail
        }) | Out-Null
        Save-Results $Results $RunRoot
    }

    Write-Progress -Id 1 -Activity "Takokit all-model prefetch" -Status "Listing installed models" -PercentComplete 99
    Write-Host ""
    Write-Host "Installed model list" -ForegroundColor Cyan

    $installedLog = Join-Path $RunRoot "installed-models.log"
    $installed = Invoke-TakoLive `
        -Executable $Tako `
        -Arguments @("list") `
        -LogPath $installedLog `
        -Model "catalog" `
        -Phase "Listing installed models" `
        -ModelNumber $Models.Count `
        -ModelCount $Models.Count

    if ($installed.exit_code -ne 0) {
        throw "tako list failed after prefetch. See $installedLog"
    }

    if (-not [string]::IsNullOrWhiteSpace($installed.stdout)) {
        Write-Host $installed.stdout
    }

    $RunComplete = $true
} catch {
    $FatalError = $_.Exception.Message
    Write-Host ""
    Write-Host "Prefetch aborted: $FatalError" -ForegroundColor Red
    Write-Host "Evidence: $RunRoot" -ForegroundColor DarkGray
} finally {
    Write-Progress -Id 2 -ParentId 1 -Activity "Takokit model operation" -Completed
    Write-Progress -Id 1 -Activity "Takokit all-model prefetch" -Completed
    $env:TAKOKIT_HOME = $PreviousTakokitHome
    $ProgressPreference = $PreviousProgressPreference
    if ($ProgressViewChanged) {
        $PSStyle.Progress.View = $PreviousProgressView
    }
    Save-Results $Results $RunRoot -Complete $RunComplete -FatalError $FatalError
}

Write-Host ""
@($Results | Group-Object status | Sort-Object Name | ForEach-Object {
    [pscustomobject]@{ status = $_.Name; count = $_.Count }
}) | Format-Table -AutoSize
Write-Host "Evidence: $RunRoot"

if ($FatalError -or $Results.status -contains "failed") {
    exit 1
}

Write-Host "Next: .\scripts\run_takokit_all_smokes.ps1 -SkipPull" -ForegroundColor Cyan
exit 0
