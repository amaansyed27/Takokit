[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Audio,
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
. (Join-Path $PSScriptRoot "takokit_testing_helpers.ps1")

function Save-SmokeReports {
    param([bool]$Complete = $false, [string]$FatalError = "")
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

function Add-SmokeResult {
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
    Save-SmokeReports
}

function Write-SmokeProgress {
    param([string]$Extra = "")
    if ($script:TotalSteps -le 0) { return }
    $step = [Math]::Min($script:CompletedSteps + 1, $script:TotalSteps)
    $percent = [Math]::Floor(($script:CompletedSteps / $script:TotalSteps) * 100)
    $status = "[$step/$($script:TotalSteps)] $($script:CurrentModel) - $($script:CurrentPhase)"
    if ($Extra) { $status += " | $Extra" }
    Write-Progress -Id 1 -Activity "Takokit all-model smoke tests" -Status $status -PercentComplete $percent
}

function Complete-SmokeStep {
    param([string]$Status, [long]$DurationMs)
    $script:CompletedSteps++
    $duration = if ($DurationMs -lt 1000) {
        "${DurationMs}ms"
    } else {
        "{0:hh\:mm\:ss}" -f [TimeSpan]::FromMilliseconds($DurationMs)
    }
    $color = if ($Status -eq "passed") { "Green" } elseif ($Status -eq "failed") { "Red" } else { "Yellow" }
    Write-Host "Result: $Status ($duration)" -ForegroundColor $color
    Write-SmokeProgress $Status
    Save-SmokeReports
}

function Invoke-SmokeCommand {
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

    $tick = {
        param($Watch, $StartedAt)
        $latest = Get-TakokitLastUsefulLine @(Get-TakokitTestLogPaths -StorageRoot $env:TAKOKIT_HOME -Since $StartedAt)
        $status = "elapsed $('{0:hh\:mm\:ss}' -f $Watch.Elapsed)"
        if ($latest) { $status += " | $latest" }
        Write-SmokeProgress $status
    }

    $result = Invoke-TakokitDirectProcess `
        -Tako $Tako `
        -Arguments $Arguments `
        -LogPath $log `
        -OnTick $tick `
        -RequireExecutablePlan:$RequireExecutablePlan

    if (-not [string]::IsNullOrWhiteSpace($result.stdout)) {
        $result.stdout.TrimEnd() -split "`r?`n" | ForEach-Object { Write-Host "  $_" }
    }
    if ($result.exit_code -ne 0 -and -not [string]::IsNullOrWhiteSpace($result.stderr)) {
        $result.stderr.TrimEnd() -split "`r?`n" | ForEach-Object { Write-Host "  $_" -ForegroundColor Red }
    }

    if ($result.exit_code -ne 0) {
        Add-SmokeResult $Model $Phase "failed" $result.duration_ms $log $result.detail
        Complete-SmokeStep "failed" $result.duration_ms
        return $false
    }

    Add-SmokeResult $Model $Phase "passed" $result.duration_ms $log "Direct command completed and validated."
    Complete-SmokeStep "passed" $result.duration_ms
    return $true
}

function Add-SkippedSmokeStep {
    param([string]$Model, [string]$Phase, [string]$Status, [string]$Detail)
    $script:CurrentModel = $Model
    $script:CurrentPhase = $Phase
    Write-Host ""
    Write-Host "[$([Math]::Min($script:CompletedSteps + 1, $script:TotalSteps))/$($script:TotalSteps)] $Model :: $Phase" -ForegroundColor Cyan
    Add-SmokeResult $Model $Phase $Status 0 "" $Detail
    Complete-SmokeStep $Status 0
}

function Test-SmokeCategory {
    param([hashtable]$Case)
    if ($Category -eq "all") { return $true }
    if ($Category -eq "omni") { return [bool]$Case.Omni }
    return $Case.Modes -contains $Category
}

$Audio = Resolve-TakokitTestPath $Audio "Audio sample"
if (-not $ReferenceAudio) { $ReferenceAudio = $Audio }
$ReferenceAudio = Resolve-TakokitTestPath $ReferenceAudio "Reference audio"
if ($TrainingSamples) { $TrainingSamples = Resolve-TakokitTestPath $TrainingSamples "Training dataset" }
if ($RvcTarget) { $RvcTarget = Resolve-TakokitTestPath $RvcTarget "RVC target checkpoint or directory" }
$Tako = Resolve-TakokitTestPath $Tako "Takokit executable"
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

$SelectedCases = @($Cases | Where-Object { Test-SmokeCategory $_ })
foreach ($case in $SelectedCases) {
    if ($case.WorkstationOnly -and -not $IncludeWorkstation) {
        $script:TotalSteps++
        continue
    }
    if (-not $SkipPull) { $script:TotalSteps++ }
    $script:TotalSteps++
    if (-not $PlanOnly) { $script:TotalSteps += $case.Modes.Count }
}

Write-Host ""
Write-Host "Takokit all-model smoke run" -ForegroundColor Cyan
Write-Host "Models:   $($SelectedCases.Count)"
Write-Host "Steps:    $($script:TotalSteps)"
Write-Host "Evidence: $($script:RunRoot)"
Write-Host "Storage:  $env:TAKOKIT_HOME"
Write-Host "Mode:     direct; port 5050 is not used" -ForegroundColor Green

$RunComplete = $false
$FatalError = ""
try {
    foreach ($case in $SelectedCases) {
        $model = $case.Id

        if ($case.WorkstationOnly -and -not $IncludeWorkstation) {
            Add-SkippedSmokeStep $model "hardware" "blocked-hardware" "Requires workstation-class memory."
            continue
        }

        if (-not $SkipPull) {
            if (-not (Invoke-SmokeCommand $model "pull" @("pull", $model))) {
                Add-SkippedSmokeStep $model "plan" "skipped-dependency" "Pull failed."
                if (-not $PlanOnly) {
                    foreach ($mode in $case.Modes) {
                        Add-SkippedSmokeStep $model $mode "skipped-dependency" "Pull failed."
                    }
                }
                continue
            }
        }

        if (-not (Invoke-SmokeCommand $model "plan" @("plan", $model, "--json") -RequireExecutablePlan)) {
            if (-not $PlanOnly) {
                foreach ($mode in $case.Modes) {
                    Add-SkippedSmokeStep $model $mode "skipped-dependency" "Plan was not executable."
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
                    Invoke-SmokeCommand $model "tts" $arguments | Out-Null
                }
                "stt" {
                    Invoke-SmokeCommand $model "stt" @("transcribe", $Audio, "--model", $model) | Out-Null
                }
                "clone" {
                    $profileName = "Smoke $model $stamp"
                    Invoke-SmokeCommand $model "clone" @("clone", $ReferenceAudio, "--name", $profileName, "--model", $model, "--consent") | Out-Null
                }
                "convert" {
                    if ($model -eq "rvc" -and -not $RvcTarget) {
                        Add-SkippedSmokeStep $model "convert" "blocked-input" "Provide -RvcTarget with a user-owned checkpoint."
                    } else {
                        $target = if ($model -eq "rvc") { $RvcTarget } else { $ReferenceAudio }
                        Invoke-SmokeCommand $model "convert" @("convert", $Audio, "--target-voice", $target, "--model", $model, "--consent") | Out-Null
                    }
                }
                "train" {
                    if (-not $TrainingSamples) {
                        Add-SkippedSmokeStep $model "train" "blocked-input" "Provide -TrainingSamples containing train.list and wavs/."
                    } else {
                        Invoke-SmokeCommand $model "train" @("train", $TrainingSamples, "--name", "smoke-$stamp", "--model", $model, "--epochs", "1", "--consent") | Out-Null
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
    Stop-TakokitActiveTestProcess
    Write-Progress -Id 1 -Activity "Takokit all-model smoke tests" -Completed
    Save-SmokeReports -Complete $RunComplete -FatalError $FatalError
}

Write-Host ""
@($script:Results | Group-Object status | Sort-Object Name | ForEach-Object {
    [pscustomobject]@{ status = $_.Name; count = $_.Count }
}) | Format-Table -AutoSize
Write-Host "Evidence: $($script:RunRoot)"
if ($FatalError -or $script:Results.status -contains "failed") { exit 1 }
exit 0
