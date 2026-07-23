[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Audio,
    [string]$ReferenceAudio,
    [string]$ReferenceText = "Hello from Takokit.",
    [string]$TrainingSamples,
    [string]$RvcTarget,
    [string]$Tako = ".\target\release\tako.exe",
    [string]$EvidenceRoot = "$HOME\takokit-test-evidence",
    [ValidateSet("all", "tts", "stt", "clone", "convert", "train", "omni")]
    [string]$Category = "all",
    [switch]$SkipPull,
    [switch]$PlanOnly,
    [switch]$IncludeWorkstation
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false
$script:ActiveProcess = $null

function Resolve-ExistingPath {
    param([string]$Path, [string]$Label)
    if (-not $Path -or -not (Test-Path -LiteralPath $Path)) {
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
            if ($line.Length -gt 180) { $line = $line.Substring(0, 177) + "..." }
            return $line
        }
    }
    return ""
}

function Get-TakokitLogPaths {
    param([datetime]$Since)
    if (-not $env:TAKOKIT_HOME) { return @() }
    $patterns = @(
        (Join-Path $env:TAKOKIT_HOME "logs\*.log"),
        (Join-Path $env:TAKOKIT_HOME "runners\*\logs\*.log"),
        (Join-Path $env:TAKOKIT_HOME "runners\python-managed\adapters\*\*.log")
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

function Test-FailureText {
    param([string]$Text)
    if ([string]::IsNullOrWhiteSpace($Text)) { return $false }
    return $Text -match '(?im)^\s*Failed after\b' -or
        $Text -match '(?im)^\s*Error:\s+' -or
        $Text -match '(?im)^\s*thread .* panicked'
}

function Save-Reports {
    param([bool]$Complete = $false, [string]$FatalError = "")
    if (-not $script:RunRoot) { return }

    @($script:Results) | ConvertTo-Json -Depth 6 | Out-File (Join-Path $script:RunRoot "results.json") -Encoding utf8
    @($script:Results) | Export-Csv (Join-Path $script:RunRoot "results.csv") -NoTypeInformation
    @($script:Results | Group-Object status | Sort-Object Name | ForEach-Object {
        [pscustomobject]@{ status = $_.Name; count = $_.Count }
    }) | ConvertTo-Json | Out-File (Join-Path $script:RunRoot "summary.json") -Encoding utf8
    @($script:Results | Where-Object status -eq "failed") |
        Export-Csv (Join-Path $script:RunRoot "failures.csv") -NoTypeInformation
    [pscustomobject]@{
        state = if ($Complete) { "complete" } else { "in-progress" }
        completed_steps = $script:CompletedSteps
        total_steps = $script:TotalSteps
        current_model = $script:CurrentModel
        current_phase = $script:CurrentPhase
        updated_at = (Get-Date).ToString("o")
        fatal_error = $FatalError
    } | ConvertTo-Json | Out-File (Join-Path $script:RunRoot "progress.json") -Encoding utf8
}

function Add-Result {
    param(
        [string]$Model,
        [string]$Phase,
        [string]$Status,
        [long]$DurationMs,
        [string]$Log,
        [string]$Detail
    )
    $script:Results.Add([pscustomobject]@{
        model = $Model
        phase = $Phase
        status = $Status
        duration_ms = $DurationMs
        log = $Log
        detail = $Detail
    }) | Out-Null
    Save-Reports
}

function Write-StepProgress {
    param([string]$Extra = "")
    if ($script:TotalSteps -le 0) { return }
    $step = [Math]::Min($script:CompletedSteps + 1, $script:TotalSteps)
    $percent = [Math]::Floor(($script:CompletedSteps / $script:TotalSteps) * 100)
    $status = "[$step/$($script:TotalSteps)] $($script:CurrentModel) - $($script:CurrentPhase)"
    if ($Extra) { $status += " | $Extra" }
    Write-Progress -Id 1 -Activity "Takokit all-model smoke tests" -Status $status -PercentComplete $percent
}

function Complete-Step {
    param([string]$Status, [long]$DurationMs)
    $script:CompletedSteps++
    $elapsed = [TimeSpan]::FromMilliseconds($DurationMs)
    $duration = if ($DurationMs -lt 1000) { "${DurationMs}ms" } else { "{0:hh\:mm\:ss}" -f $elapsed }
    $color = if ($Status -eq "passed") { "Green" } elseif ($Status -eq "failed") { "Red" } else { "Yellow" }
    Write-Host "Result: $Status ($duration)" -ForegroundColor $color
    Write-StepProgress $Status
    Save-Reports
}

function Invoke-TakoDirect {
    param(
        [string]$Model,
        [string]$Phase,
        [string[]]$Arguments,
        [switch]$RequireExecutablePlan
    )

    $script:CurrentModel = $Model
    $script:CurrentPhase = $Phase
    $safeModel = $Model -replace '[^a-zA-Z0-9._-]', '_'
    $safePhase = $Phase -replace '[^a-zA-Z0-9._-]', '_'
    $log = Join-Path $script:RunRoot "$safeModel-$safePhase.log"
    Write-Host ""
    Write-Host "[$([Math]::Min($script:CompletedSteps + 1, $script:TotalSteps))/$($script:TotalSteps)] $Model :: $Phase" -ForegroundColor Cyan
    Write-Host "Log: $log" -ForegroundColor DarkGray
    Write-StepProgress "starting"

    $allArguments = @("--direct") + $Arguments
    $argumentLine = (@($allArguments | ForEach-Object { ConvertTo-ProcessArgument "$_" })) -join ' '
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

        while (-not $process.WaitForExit(750)) {
            $tail = Get-LastUsefulLine @(Get-TakokitLogPaths -Since $startedAt)
            $status = "elapsed {0:hh\:mm\:ss}" -f $watch.Elapsed
            if ($tail) { $status += " | $tail" }
            Write-StepProgress $status
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
        if ($process -and -not $process.HasExited) { Stop-ProcessTree $process.Id }
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
            $combined += [Environment]::NewLine + "[validation error: invalid plan JSON]"
        }
    }

    $effectiveExitCode = if ($failed) { 1 } else { 0 }
    $combined += [Environment]::NewLine + "[effective exit code: $effectiveExitCode]"
    $combined | Set-Content -LiteralPath $log -Encoding utf8

    if (-not [string]::IsNullOrWhiteSpace($stdout)) {
        $stdout.TrimEnd() -split "`r?`n" | ForEach-Object { Write-Host "  $_" }
    }
    if ($failed -and -not [string]::IsNullOrWhiteSpace($stderr)) {
        $stderr.TrimEnd() -split "`r?`n" | ForEach-Object { Write-Host "  $_" -ForegroundColor Red }
    }

    if ($failed) {
        $detail = Get-LastUsefulLine @($log)
        if (-not $detail) { $detail = "command failed" }
        Add-Result $Model $Phase "failed" $watch.ElapsedMilliseconds $log $detail
        Complete-Step "failed" $watch.ElapsedMilliseconds
        return $false
    }

    Add-Result $Model $Phase "passed" $watch.ElapsedMilliseconds $log "Direct command completed and validated."
    Complete-Step "passed" $watch.ElapsedMilliseconds
    return $true
}

function Add-SkippedStep {
    param([string]$Model, [string]$Phase, [string]$Status, [string]$Detail)
    $script:CurrentModel = $Model
    $script:CurrentPhase = $Phase
    Write-Host ""
    Write-Host "[$([Math]::Min($script:CompletedSteps + 1, $script:TotalSteps))/$($script:TotalSteps)] $Model :: $Phase" -ForegroundColor Cyan
    Add-Result $Model $Phase $Status 0 "" $Detail
    Complete-Step $Status 0
}

function Test-Category {
    param([hashtable]$Case)
    if ($Category -eq "all") { return $true }
    if ($Category -eq "omni") { return [bool]$Case.Omni }
    return $Case.Modes -contains $Category
}

$Audio = Resolve-ExistingPath $Audio "Audio sample"
if (-not $ReferenceAudio) { $ReferenceAudio = $Audio }
$ReferenceAudio = Resolve-ExistingPath $ReferenceAudio "Reference audio"
if ($TrainingSamples) { $TrainingSamples = Resolve-ExistingPath $TrainingSamples "Training dataset" }
if ($RvcTarget) { $RvcTarget = Resolve-ExistingPath $RvcTarget "RVC target checkpoint or directory" }
$Tako = Resolve-ExistingPath $Tako "Takokit executable"
$EvidenceRoot = [System.IO.Path]::GetFullPath([Environment]::ExpandEnvironmentVariables($EvidenceRoot))
New-Item -ItemType Directory -Force $EvidenceRoot | Out-Null

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$script:RunRoot = Join-Path $EvidenceRoot "all-models-$stamp"
New-Item -ItemType Directory -Force $script:RunRoot | Out-Null
$script:Results = [System.Collections.Generic.List[object]]::new()
$script:CompletedSteps = 0
$script:CurrentModel = "initializing"
$script:CurrentPhase = "environment"
$script:TotalSteps = 0

$Cases = @(
    @{ Id = "bark-small"; Modes = @("tts"); Voice = "default" },
    @{ Id = "canary"; Modes = @("stt") },
    @{ Id = "chatterbox"; Modes = @("tts", "clone"); Voice = "default" },
    @{ Id = "cosyvoice2"; Modes = @("tts", "clone"); Voice = $ReferenceAudio },
    @{ Id = "dia"; Modes = @("tts"); Text = "[S1] Takokit is ready. [S2] The dialogue model is responding."; Voice = "default" },
    @{ Id = "distil-whisper-large-v3"; Modes = @("stt") },
    @{ Id = "f5-tts"; Modes = @("tts", "clone"); Voice = "default" },
    @{ Id = "fish-speech"; Modes = @("tts", "clone"); Voice = $ReferenceAudio },
    @{ Id = "gpt-sovits"; Modes = @("tts", "clone", "train"); Voice = $ReferenceAudio; ReferenceText = $ReferenceText },
    @{ Id = "kokoro"; Modes = @("tts"); Voice = "default" },
    @{ Id = "kyutai-tts-1.6b"; Modes = @("tts"); Voice = "default" },
    @{ Id = "mms-tts-eng"; Modes = @("tts"); Voice = "default" },
    @{ Id = "openvoice"; Modes = @("tts", "clone", "convert"); Voice = $ReferenceAudio },
    @{ Id = "parakeet"; Modes = @("stt") },
    @{ Id = "piper-lessac"; Modes = @("tts"); Voice = "default" },
    @{ Id = "qwen2-5-omni"; Modes = @("tts", "stt"); Voice = "default"; Omni = $true },
    @{ Id = "qwen3-omni"; Modes = @("tts", "stt"); Voice = "default"; Omni = $true; WorkstationOnly = $true },
    @{ Id = "qwen3-tts"; Modes = @("tts"); Voice = "Ryan" },
    @{ Id = "qwen3-tts-0.6b-base"; Modes = @("tts", "clone"); Voice = $ReferenceAudio; ReferenceText = $ReferenceText },
    @{ Id = "qwen3-tts-1.7b-base"; Modes = @("tts", "clone"); Voice = $ReferenceAudio; ReferenceText = $ReferenceText },
    @{ Id = "qwen3-tts-1.7b-custom"; Modes = @("tts"); Voice = "Ryan" },
    @{ Id = "qwen3-tts-1.7b-voice-design"; Modes = @("tts"); Voice = "default"; Instruction = "Warm, clear, confident narration with a natural pace." },
    @{ Id = "rvc"; Modes = @("convert") },
    @{ Id = "sensevoice"; Modes = @("stt") },
    @{ Id = "voxtral"; Modes = @("stt") },
    @{ Id = "wav2vec2-base-960h"; Modes = @("stt") },
    @{ Id = "whisper-base"; Modes = @("stt") },
    @{ Id = "whisper-small"; Modes = @("stt") },
    @{ Id = "whisper-tiny"; Modes = @("stt") },
    @{ Id = "xtts-v2"; Modes = @("tts", "clone"); Voice = $ReferenceAudio },
    @{ Id = "yourtts"; Modes = @("tts", "clone"); Voice = $ReferenceAudio }
)

$SelectedCases = @($Cases | Where-Object { Test-Category $_ })
foreach ($case in $SelectedCases) {
    if ($case.WorkstationOnly -and -not $IncludeWorkstation) {
        $script:TotalSteps++
        continue
    }
    if (-not $SkipPull) { $script:TotalSteps++ }
    $script:TotalSteps++
    if (-not $PlanOnly) { $script:TotalSteps += $case.Modes.Count }
}

[pscustomobject]@{
    started_at = (Get-Date).ToString("o")
    category = $Category
    models = $SelectedCases.Count
    planned_steps = $script:TotalSteps
    takokit_home = $env:TAKOKIT_HOME
    takokit_executable = $Tako
    mode = "direct"
} | ConvertTo-Json | Out-File (Join-Path $script:RunRoot "run.json") -Encoding utf8

Write-Host ""
Write-Host "Takokit all-model smoke run" -ForegroundColor Cyan
Write-Host "Models:   $($SelectedCases.Count)"
Write-Host "Steps:    $($script:TotalSteps)"
Write-Host "Evidence: $($script:RunRoot)"
Write-Host "Storage:  $env:TAKOKIT_HOME"
Write-Host "Mode:     direct; port 5050 and managed daemons are not used" -ForegroundColor Green

$RunComplete = $false
$FatalError = ""
try {
    foreach ($case in $SelectedCases) {
        $model = $case.Id

        if ($case.WorkstationOnly -and -not $IncludeWorkstation) {
            Add-SkippedStep $model "hardware" "blocked-hardware" "Requires workstation-class memory."
            continue
        }

        if (-not $SkipPull) {
            if (-not (Invoke-TakoDirect $model "pull" @("pull", $model))) {
                Add-SkippedStep $model "plan" "skipped-dependency" "Pull failed."
                if (-not $PlanOnly) {
                    foreach ($mode in $case.Modes) { Add-SkippedStep $model $mode "skipped-dependency" "Pull failed." }
                }
                continue
            }
        }

        if (-not (Invoke-TakoDirect $model "plan" @("plan", $model, "--json") -RequireExecutablePlan)) {
            if (-not $PlanOnly) {
                foreach ($mode in $case.Modes) { Add-SkippedStep $model $mode "skipped-dependency" "Plan was not executable." }
            }
            continue
        }
        if ($PlanOnly) { continue }

        foreach ($mode in $case.Modes) {
            switch ($mode) {
                "tts" {
                    $text = if ($case.Text) { $case.Text } else { "Takokit all-model smoke test for $model." }
                    $arguments = @("speak", $text, "--model", $model, "--voice", [string]$case.Voice)
                    if ($case.ReferenceText) { $arguments += @("--reference-text", [string]$case.ReferenceText) }
                    if ($case.Instruction) { $arguments += @("--instruction", [string]$case.Instruction) }
                    Invoke-TakoDirect $model "tts" $arguments | Out-Null
                }
                "stt" {
                    Invoke-TakoDirect $model "stt" @("transcribe", $Audio, "--model", $model) | Out-Null
                }
                "clone" {
                    $profileName = "Smoke $model $stamp"
                    Invoke-TakoDirect $model "clone" @("clone", $ReferenceAudio, "--name", $profileName, "--model", $model, "--consent") | Out-Null
                }
                "convert" {
                    if ($model -eq "rvc" -and -not $RvcTarget) {
                        Add-SkippedStep $model "convert" "blocked-input" "Provide -RvcTarget with a user-owned checkpoint."
                    } else {
                        $target = if ($model -eq "rvc") { $RvcTarget } else { $ReferenceAudio }
                        Invoke-TakoDirect $model "convert" @("convert", $Audio, "--target-voice", $target, "--model", $model, "--consent") | Out-Null
                    }
                }
                "train" {
                    if (-not $TrainingSamples) {
                        Add-SkippedStep $model "train" "blocked-input" "Provide -TrainingSamples containing train.list and wavs/."
                    } else {
                        Invoke-TakoDirect $model "train" @("train", $TrainingSamples, "--name", "smoke-$stamp", "--model", $model, "--epochs", "1", "--consent") | Out-Null
                    }
                }
            }
        }
    }
    $RunComplete = $true
} catch {
    $FatalError = $_.Exception.Message
    Write-Host "Smoke run aborted: $FatalError" -ForegroundColor Red
} finally {
    if ($script:ActiveProcess -and -not $script:ActiveProcess.HasExited) {
        Stop-ProcessTree $script:ActiveProcess.Id
    }
    Write-Progress -Id 1 -Activity "Takokit all-model smoke tests" -Completed
    Save-Reports -Complete $RunComplete -FatalError $FatalError
}

Write-Host ""
@($script:Results | Group-Object status | Sort-Object Name | ForEach-Object {
    [pscustomobject]@{ status = $_.Name; count = $_.Count }
}) | Format-Table -AutoSize
Write-Host "Evidence: $($script:RunRoot)"

if ($FatalError -or $script:Results.status -contains "failed") { exit 1 }
exit 0
