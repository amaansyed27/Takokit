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
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    & $Tako @Arguments *>&1 | Tee-Object -FilePath $log
    $exitCode = $LASTEXITCODE
    $watch.Stop()
    if ($exitCode -eq 0) {
        Add-Result $Model $Phase "passed" $watch.ElapsedMilliseconds $log "exit code 0"
        return $true
    }
    Add-Result $Model $Phase "failed" $watch.ElapsedMilliseconds $log "exit code $exitCode"
    return $false
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

foreach ($case in $Cases) {
    if (-not (Test-Category $case)) { continue }
    $model = $case.Id

    if ($case.WorkstationOnly -and -not $IncludeWorkstation) {
        Add-Result $model "hardware" "blocked-hardware" 0 "" "Requires workstation-class memory; rerun with -IncludeWorkstation on suitable hardware."
        continue
    }

    if (-not $SkipPull) {
        if (-not (Invoke-Tako $model "pull" @("pull", $model))) { continue }
    }
    if (-not (Invoke-Tako $model "plan" @("plan", $model, "--json"))) { continue }
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
                    Add-Result $model "convert" "blocked-input" 0 "" "Provide -RvcTarget pointing to a user-owned .pth checkpoint or directory."
                } else {
                    $target = if ($model -eq "rvc") { $RvcTarget } else { $ReferenceAudio }
                    Invoke-Tako $model "convert" @("convert", $Audio, "--target-voice", $target, "--model", $model, "--consent") | Out-Null
                }
            }
            "train" {
                if (-not $TrainingSamples) {
                    Add-Result $model "train" "blocked-input" 0 "" "Provide -TrainingSamples containing train.list and wavs/."
                } else {
                    Invoke-Tako $model "train" @("train", $TrainingSamples, "--name", "smoke-$stamp", "--model", $model, "--epochs", "1", "--consent") | Out-Null
                }
            }
        }
    }
}

$Results | ConvertTo-Json -Depth 5 | Out-File (Join-Path $RunRoot "results.json") -Encoding utf8
$Results | Export-Csv (Join-Path $RunRoot "results.csv") -NoTypeInformation

$summary = $Results | Group-Object status | Sort-Object Name | ForEach-Object {
    [pscustomobject]@{ status = $_.Name; count = $_.Count }
}
$summary | Format-Table -AutoSize
$summary | ConvertTo-Json | Out-File (Join-Path $RunRoot "summary.json") -Encoding utf8
Write-Host "Evidence: $RunRoot"

if ($Results.status -contains "failed") {
    exit 1
}
