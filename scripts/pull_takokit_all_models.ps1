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

function Stop-ProcessTree {
    param([int]$ProcessId)

    if (-not (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)) { return }

    if ($env:OS -eq "Windows_NT") {
        & taskkill.exe /PID $ProcessId /T /F 2>&1 | Out-Null
    } else {
        Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
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
            if ($line.Length -gt 170) { $line = $line.Substring(0, 167) + "..." }
            return $line
        }
    }
    return ""
}

function Get-TakokitLogPaths {
    param([datetime]$Since)

    $patterns = @(
        (Join-Path $StorageRoot "logs\*.log"),
        (Join-Path $StorageRoot "runners\*\logs\*.log"),
        (Join-Path $StorageRoot "runners\python-managed\adapters\*\*.log")
    )

    return @(
        foreach ($pattern in $patterns) {
            Get-ChildItem -Path $pattern -File -ErrorAction SilentlyContinue |
                Where-Object { $_.LastWriteTime -ge $Since.AddSeconds(-2) }
        } |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 6 -ExpandProperty FullName
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

    $latest = Get-LastUsefulLine @(Get-TakokitLogPaths -Since $StartedAt)
    $elapsed = "{0:hh\:mm\:ss}" -f $Watch.Elapsed
    $status = "$(New-PulseBar -ElapsedMilliseconds $Watch.ElapsedMilliseconds) elapsed $elapsed"
    if ($latest) { $status += " | $latest" }
    Write-Progress -Id 2 -ParentId 1 -Activity "$Phase $Model" -Status $status -PercentComplete -1
}

function Test-FailureText {
    param([string]$Text)

    if ([string]::IsNullOrWhiteSpace($Text)) { return $false }
    return $Text -match '(?im)^\s*Failed after\b' -or
        $Text -match '(?im)^\s*Error:\s+' -or
        $Text -match '(?im)^\s*thread .* panicked'
}

function Invoke-TakoDirect {
    param(
        [string[]]$Arguments,
        [string]$LogPath,
        [string]$Model,
        [string]$Phase,
        [int]$ModelNumber,
        [int]$ModelCount,
        [switch]$RequireExecutablePlan
    )

    $allArguments = @("--direct") + $Arguments
    $argumentLine = (@($allArguments | ForEach-Object { ConvertTo-ProcessArgument "$_" })) -join ' '
    $stdoutLog = "$LogPath.stdout.tmp"
    $stderrLog = "$LogPath.stderr.tmp"
    Remove-Item -LiteralPath $stdoutLog, $stderrLog -Force -ErrorAction SilentlyContinue

    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $Tako
    $startInfo.Arguments = $argumentLine
    $startInfo.WorkingDirectory = (Get-Location).Path
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $startInfo
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $startedAt = Get-Date
    $reportedExitCode = -1
    $stdout = ""
    $stderr = ""
    $json = $null

    try {
        if (-not $process.Start()) { throw "Failed to start Takokit process." }
        $script:ActiveProcess = $process
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()

        while (-not $process.WaitForExit(500)) {
            Write-ModelProgress -Model $Model -Phase $Phase -ModelNumber $ModelNumber -ModelCount $ModelCount -Watch $watch -StartedAt $startedAt
        }

        $process.WaitForExit()
        $stdoutTask.Wait()
        $stderrTask.Wait()
        $process.Refresh()
        $stdout = "$($stdoutTask.Result)"
        $stderr = "$($stderrTask.Result)"
        $reportedExitCode = [int]$process.ExitCode
    } finally {
        $watch.Stop()
        Write-Progress -Id 2 -ParentId 1 -Activity "$Phase $Model" -Completed
        if ($process -and -not $process.HasExited) {
            Stop-ProcessTree $process.Id
        }
        $script:ActiveProcess = $null
        $process.Dispose()
    }

    $combined = @(
        "=== command ===",
        "$Tako $($allArguments -join ' ')",
        "=== stdout ===",
        $stdout,
        "=== stderr ===",
        $stderr,
        "[reported exit code: $reportedExitCode]"
    ) -join [Environment]::NewLine

    $failed = $reportedExitCode -ne 0 -or (Test-FailureText $combined)
    if ($RequireExecutablePlan -and -not $failed) {
        try {
            $json = $stdout | ConvertFrom-Json -ErrorAction Stop
            if ($json.executable -ne $true) {
                $failed = $true
                $combined += [Environment]::NewLine + "[validation error: plan executable was not true]"
            }
        } catch {
            $failed = $true
            $combined += [Environment]::NewLine + "[validation error: plan output was not valid JSON]"
        }
    }

    $effectiveExitCode = if ($failed) { 1 } else { 0 }
    $combined += [Environment]::NewLine + "[effective exit code: $effectiveExitCode]"
    $combined | Set-Content -LiteralPath $LogPath -Encoding utf8
    Remove-Item -LiteralPath $stdoutLog, $stderrLog -Force -ErrorAction SilentlyContinue

    $detail = if ($failed) {
        Get-LastUsefulLine @($LogPath)
    } elseif (-not [string]::IsNullOrWhiteSpace($stdout)) {
        ($stdout -split "`r?`n" | Where-Object { $_.Trim() } | Select-Object -Last 1)
    } else {
        "completed"
    }

    return [pscustomobject]@{
        exit_code = $effectiveExitCode
        duration_ms = [long]$watch.ElapsedMilliseconds
        stdout = $stdout
        stderr = $stderr
        json = $json
        detail = "$detail"
    }
}

function Test-ReadyRecord {
    param([string]$Model)

    $record = Join-Path $StorageRoot "manifests\installed-models\$Model.toml"
    $manifest = Join-Path $StorageRoot "manifests\models\$Model.toml"
    if (-not (Test-Path -LiteralPath $record -PathType Leaf)) { return $false }
    if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) { return $false }

    $source = Get-Content -LiteralPath $record -Raw
    return $source -match '(?im)^\s*status\s*=\s*"ready"\s*$'
}

function Save-Results {
    param(
        [System.Collections.Generic.List[object]]$Results,
        [string]$RunRoot,
        [bool]$Complete = $false,
        [string]$FatalError = ""
    )

    @($Results) | ConvertTo-Json -Depth 6 | Out-File (Join-Path $RunRoot "results.json") -Encoding utf8
    @($Results) | Export-Csv (Join-Path $RunRoot "results.csv") -NoTypeInformation
    @($Results | Group-Object status | Sort-Object Name | ForEach-Object {
        [pscustomobject]@{ status = $_.Name; count = $_.Count }
    }) | ConvertTo-Json | Out-File (Join-Path $RunRoot "summary.json") -Encoding utf8
    [pscustomobject]@{
        state = if ($Complete) { "complete" } else { "in-progress" }
        updated_at = (Get-Date).ToString("o")
        completed = $Results.Count
        fatal_error = $FatalError
    } | ConvertTo-Json | Out-File (Join-Path $RunRoot "progress.json") -Encoding utf8
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
    started_at = (Get-Date).ToString("o")
    takokit_executable = $Tako
    takokit_home = $StorageRoot
    model_count = $Models.Count
    mode = "direct"
    include_workstation = [bool]$IncludeWorkstation
} | ConvertTo-Json | Out-File (Join-Path $RunRoot "run.json") -Encoding utf8

Write-Host ""
Write-Host "Takokit sequential all-model prefetch" -ForegroundColor Cyan
Write-Host "Models:   $($Models.Count)"
Write-Host "Storage:  $StorageRoot"
Write-Host "Evidence: $RunRoot"
Write-Host "Mode:     direct; port 5050 and managed daemons are not used" -ForegroundColor Green
Write-Host ""
Write-Host "Interrupted downloads stay in cache and are resumed or revalidated on rerun." -ForegroundColor DarkGray

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

    for ($index = 0; $index -lt $Models.Count; $index++) {
        $entry = $Models[$index]
        $model = $entry.Id
        $number = $index + 1
        Write-Host ""
        Write-Host "[$number/$($Models.Count)] $model" -ForegroundColor Cyan

        if ($entry.WorkstationOnly -and -not $IncludeWorkstation) {
            Write-Host "  blocked-hardware" -ForegroundColor Yellow
            $Results.Add([pscustomobject]@{
                model = $model; status = "blocked-hardware"; duration_ms = 0
                pull_log = ""; plan_log = ""; detail = "Workstation-only model omitted."
            }) | Out-Null
            Save-Results $Results $RunRoot
            continue
        }

        $safeModel = $model -replace '[^a-zA-Z0-9._-]', '_'
        $pullLog = Join-Path $RunRoot "$safeModel-pull.log"
        $planLog = Join-Path $RunRoot "$safeModel-plan.log"

        Write-Host "  Pulling directly..."
        Write-Host "  Log: $pullLog" -ForegroundColor DarkGray
        $pull = Invoke-TakoDirect -Arguments @("pull", $model) -LogPath $pullLog -Model $model -Phase "Pulling" -ModelNumber $number -ModelCount $Models.Count

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

        Write-Host "  Verifying executable plan..."
        $plan = Invoke-TakoDirect -Arguments @("plan", $model, "--json") -LogPath $planLog -Model $model -Phase "Verifying" -ModelNumber $number -ModelCount $Models.Count -RequireExecutablePlan
        $duration = $pull.duration_ms + $plan.duration_ms

        if ($plan.exit_code -ne 0 -or -not (Test-ReadyRecord $model)) {
            $detail = if ($plan.exit_code -ne 0) { $plan.detail } else { "ready install record is missing" }
            Write-Host "  failed: $detail" -ForegroundColor Red
            $status = "failed"
        } else {
            $elapsed = [TimeSpan]::FromMilliseconds($duration)
            Write-Host ("  passed in {0:hh\:mm\:ss}" -f $elapsed) -ForegroundColor Green
            $detail = "Direct pull, ready install record and executable plan verified."
            $status = "passed"
        }

        $Results.Add([pscustomobject]@{
            model = $model; status = $status; duration_ms = $duration
            pull_log = $pullLog; plan_log = $planLog; detail = $detail
        }) | Out-Null
        Save-Results $Results $RunRoot
    }

    $passed = @($Results | Where-Object { $_.status -eq "passed" } | Select-Object -ExpandProperty model)
    $readyRecords = @(
        Get-ChildItem -LiteralPath (Join-Path $StorageRoot "manifests\installed-models") -Filter "*.toml" -File -ErrorAction SilentlyContinue |
            Where-Object { (Get-Content -LiteralPath $_.FullName -Raw) -match '(?im)^\s*status\s*=\s*"ready"\s*$' } |
            Select-Object -ExpandProperty BaseName
    )
    $missingPassed = @($passed | Where-Object { $readyRecords -notcontains $_ })
    if ($missingPassed.Count -gt 0) {
        throw "Passed models missing ready records: $($missingPassed -join ', ')"
    }

    Write-Host ""
    Write-Host "Ready models in isolated storage" -ForegroundColor Cyan
    $readyRecords | Sort-Object | ForEach-Object { Write-Host "  $_" }
    $RunComplete = $true
} catch {
    $FatalError = $_.Exception.Message
    Write-Host ""
    Write-Host "Prefetch aborted: $FatalError" -ForegroundColor Red
} finally {
    if ($script:ActiveProcess -and -not $script:ActiveProcess.HasExited) {
        Stop-ProcessTree $script:ActiveProcess.Id
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