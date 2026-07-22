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

function Resolve-ExistingPath {
    param([string]$Path, [string]$Label)
    if (-not $Path) {
        throw "$Label was not provided."
    }
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "$Label does not exist: $Path"
    }
    return (Resolve-Path -LiteralPath $Path).Path
}

function Save-Reports {
    param([bool]$Complete = $false, [string]$FatalError = "")
    if (-not $script:RunRoot -or -not $script:Results) { return }

    $resultsJson = Join-Path $script:RunRoot "results.json"
    $resultsCsv = Join-Path $script:RunRoot "results.csv"
    $summaryJson = Join-Path $script:RunRoot "summary.json"
    $failuresCsv = Join-Path $script:RunRoot "failures.csv"

    @($script:Results) | ConvertTo-Json -Depth 5 | Out-File $resultsJson -Encoding utf8
    @($script:Results) | Export-Csv $resultsCsv -NoTypeInformation

    $summary = @($script:Results | Group-Object status | Sort-Object Name | ForEach-Object {
        [pscustomobject]@{ status = $_.Name; count = $_.Count }
    })
    $summary | ConvertTo-Json | Out-File $summaryJson -Encoding utf8

    $failures = @($script:Results | Where-Object { $_.status -eq "failed" })
    if ($failures.Count -gt 0) {
        $failures | Export-Csv $failuresCsv -NoTypeInformation
    }

    [pscustomobject]@{
        state           = if ($Complete) { "complete" } else { "in-progress" }
        completed_steps = $script:CompletedSteps
        total_steps     = $script:TotalSteps
        current_model   = $script:CurrentModel
        current_phase   = $script:CurrentPhase
        started_at      = $script:RunStartedAt.ToString("o")
        updated_at      = (Get-Date).ToString("o")
        fatal_error     = $FatalError
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
        model       = $Model
        phase       = $Phase
        status      = $Status
        duration_ms = $DurationMs
        log         = $Log
        detail      = $Detail
    }) | Out-Null
    Save-Reports
}

function ConvertTo-ProcessArgument {
    param([AllowEmptyString()][string]$Value)
    if ($Value.Length -eq 0) { return '""' }
    if ($Value -notmatch '[\s"]') { return $Value }

    $escaped = $Value -replace '(\\*)"', '$1$1\"'
    $escaped = $escaped -replace '(\\+)$', '$1$1'
    return '"' + $escaped + '"'
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

function Start-Step {
    param([string]$Model, [string]$Phase, [string]$Log = "")
    $script:CurrentModel = $Model
    $script:CurrentPhase = $Phase
    $step = [Math]::Min($script:CompletedSteps + 1, $script:TotalSteps)
    Write-Host ""
    Write-Host "[$step/$($script:TotalSteps)] $Model :: $Phase" -ForegroundColor Cyan
    if ($Log) { Write-Host "Log: $Log" -ForegroundColor DarkGray }
    Write-StepProgress "starting"
    Save-Reports
}

function Complete-Step {
    param([string]$Status, [long]$DurationMs = 0)
    $script:CompletedSteps++
    $elapsed = [TimeSpan]::FromMilliseconds($DurationMs)
    $duration = if ($DurationMs -lt 1000) {
        "${DurationMs}ms"
    } elseif ($elapsed.TotalHours -ge 1) {
        "{0:h\h\ mm\m\ ss\s}" -f $elapsed
    } else {
        "{0:m\m\ ss\s}" -f $elapsed
    }
    $color = switch ($Status) {
        "passed" { "Green" }
        "failed" { "Red" }
        default { "Yellow" }
    }
    Write-Host "Result: $Status ($duration)" -ForegroundColor $color
    Write-StepProgress "$Status"
    Save-Reports
}

function Get-LastUsefulLine {
    param([string[]]$Paths)
    foreach ($path in $Paths) {
        if (-not (Test-Path -LiteralPath $path)) { continue }
        $line = Get-Content -LiteralPath $path -Tail 20 -ErrorAction SilentlyContinue |
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

function Invoke-Tako {
    param(
        [string]$Model,
        [string]$Phase,
        [string[]]$Arguments
    )
    $safeModel = $Model -replace '[^a-zA-Z0-9._-]', '_'
    $safePhase = $Phase -replace '[^a-zA-Z0-9._-]', '_'
    $log = Join-Path $RunRoot "$safeModel-$safePhase.log"
    $stdoutLog = Join-Path $RunRoot "$safeModel-$safePhase.stdout.tmp"
    $stderrLog = Join-Path $RunRoot "$safeModel-$safePhase.stderr.tmp"
    Start-Step $Model $Phase $log
    $watch = [System.Diagnostics.Stopwatch]::StartNew()

    try {
        $argumentLine = (@($Arguments | ForEach-Object { ConvertTo-ProcessArgument "$_" })) -join ' '
        $process = Start-Process -FilePath $Tako `
            -ArgumentList $argumentLine `
            -NoNewWindow `
            -PassThru `
            -RedirectStandardOutput $stdoutLog `
            -RedirectStandardError $stderrLog

        while (-not $process.HasExited) {
            $tail = Get-LastUsefulLine @($stderrLog, $stdoutLog)
            $status = "elapsed {0:hh\:mm\:ss}" -f $watch.Elapsed
            if ($tail) { $status += " | $tail" }
            Write-StepProgress $status
            Start-Sleep -Milliseconds 750
            $process.Refresh()
        }
        $process.WaitForExit()
        $exitCode = $process.ExitCode
    } catch {
        $exitCode = -1
        $_.Exception.Message | Out-File $stderrLog -Append -Encoding utf8
    } finally {
        $watch.Stop()
    }

    $captured = @()
    if (Test-Path -LiteralPath $stdoutLog) { $captured += Get-Content -LiteralPath $stdoutLog }
    if (Test-Path -LiteralPath $stderrLog) { $captured += Get-Content -LiteralPath $stderrLog }
    $captured | Out-File -LiteralPath $log -Encoding utf8
    Remove-Item -LiteralPath $stdoutLog, $stderrLog -Force -ErrorAction SilentlyContinue

    if ($captured.Count -gt 0) {
        $captured | ForEach-Object { Write-Host "  $_" }
    }

    if ($exitCode -eq 0) {
        Add-Result $Model $Phase "passed" $watch.ElapsedMilliseconds $log "exit code 0"
        Complete-Step "passed" $watch.ElapsedMilliseconds
        return $true
    }

    $detail = Get-LastUsefulLine @($log)
    if (-not $detail) { $detail = "exit code $exitCode" }
    else { $detail = "exit code $exitCode`: $detail" }
    Add-Result $Model $Phase "failed" $watch.ElapsedMilliseconds $log $detail
    Complete-Step "failed" $watch.ElapsedMilliseconds
    return $false
}

function Add-SkippedStep {
    param([string]$Model, [string]$Phase, [string]$Status, [string]$Detail)
    Start-Step $Model $Phase
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

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$RunRoot = Join-Path $EvidenceRoot "all-models-$stamp"
New-Item -ItemType Directory -Force $RunRoot | Out-Null
$Results = [System.Collections.Generic.List[object]]::new()
$RunStartedAt = Get-Date
$CompletedSteps = 0
$CurrentModel = "initializing"
$CurrentPhase = "environment"
$TotalSteps = 0

try {
    git rev-parse HEAD | Out-File (Join-Path $RunRoot "commit.txt")
} catch {
    "unknown" | Out-File (Join-Path $RunRoot "commit.txt")
}
try { nvidia-smi | Out-File (Join-Path $RunRoot "nvidia-smi.txt") } catch {}

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
        $TotalSteps++
        continue
    }
    if (-not $SkipPull) { $TotalSteps++ }
    $TotalSteps++
    if (-not $PlanOnly) { $TotalSteps += $case.Modes.Count }
}

[pscustomobject]@{
    started_at        = $RunStartedAt.ToString("o")
    category          = $Category
    models            = $SelectedCases.Count
    planned_steps     = $TotalSteps
    audio             = $Audio
    reference_audio   = $ReferenceAudio
    training_samples  = $TrainingSamples
    rvc_target        = $RvcTarget
    takokit_home      = $env:TAKOKIT_HOME
    takokit_executable = $Tako
} | ConvertTo-Json | Out-File (Join-Path $RunRoot "run.json") -Encoding utf8

Write-Host ""
Write-Host "Takokit all-model smoke run" -ForegroundColor Cyan
Write-Host "Models:   $($SelectedCases.Count)"
Write-Host "Steps:    $TotalSteps"
Write-Host "Evidence: $RunRoot"
Write-Host "Storage:  $env:TAKOKIT_HOME"

$RunComplete = $false
$FatalError = ""
try {
    foreach ($case in $SelectedCases) {
        $model = $case.Id

        if ($case.WorkstationOnly -and -not $IncludeWorkstation) {
            Add-SkippedStep $model "hardware" "blocked-hardware" "Requires workstation-class memory; rerun with -IncludeWorkstation on suitable hardware."
            continue
        }

        if (-not $SkipPull) {
            if (-not (Invoke-Tako $model "pull" @("pull", $model))) {
                Add-SkippedStep $model "plan" "skipped-dependency" "Pull failed."
                if (-not $PlanOnly) {
                    foreach ($mode in $case.Modes) {
                        Add-SkippedStep $model $mode "skipped-dependency" "Pull failed."
                    }
                }
                continue
            }
        }
        if (-not (Invoke-Tako $model "plan" @("plan", $model, "--json"))) {
            if (-not $PlanOnly) {
                foreach ($mode in $case.Modes) {
                    Add-SkippedStep $model $mode "skipped-dependency" "Plan failed."
                }
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
                    Invoke-Tako $model "tts" $arguments | Out-Null
                }
                "stt" {
                    Invoke-Tako $model "stt" @("transcribe", $Audio, "--model", $model) | Out-Null
                }
                "clone" {
                    $profileName = "Smoke $model $stamp"
                    Invoke-Tako $model "clone" @("clone", $ReferenceAudio, "--name", $profileName, "--model", $model, "--consent") | Out-Null
                }
                "convert" {
                    if ($model -eq "rvc" -and -not $RvcTarget) {
                        Add-SkippedStep $model "convert" "blocked-input" "Provide -RvcTarget pointing to a user-owned .pth checkpoint or directory."
                    } else {
                        $target = if ($model -eq "rvc") { $RvcTarget } else { $ReferenceAudio }
                        Invoke-Tako $model "convert" @("convert", $Audio, "--target-voice", $target, "--model", $model, "--consent") | Out-Null
                    }
                }
                "train" {
                    if (-not $TrainingSamples) {
                        Add-SkippedStep $model "train" "blocked-input" "Provide -TrainingSamples containing train.list and wavs/."
                    } else {
                        Invoke-Tako $model "train" @("train", $TrainingSamples, "--name", "smoke-$stamp", "--model", $model, "--epochs", "1", "--consent") | Out-Null
                    }
                }
            }
        }
    }
    $RunComplete = $true
} catch {
    $FatalError = $_.Exception.Message
    throw
} finally {
    Save-Reports -Complete $RunComplete -FatalError $FatalError
    Write-Progress -Id 1 -Activity "Takokit all-model smoke tests" -Completed
}

$summary = @($Results | Group-Object status | Sort-Object Name | ForEach-Object {
    [pscustomobject]@{ status = $_.Name; count = $_.Count }
})
$summary | Format-Table -AutoSize
Write-Host "Evidence: $RunRoot"
Write-Host "Failures: $(Join-Path $RunRoot 'failures.csv')"

if ($Results.status -contains "failed") {
    exit 1
}
