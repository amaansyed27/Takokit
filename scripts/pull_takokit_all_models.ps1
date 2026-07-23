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

function Save-Results {
    param(
        [System.Collections.Generic.List[object]]$Results,
        [string]$RunRoot,
        [bool]$Complete = $false
    )

    @($Results) | ConvertTo-Json -Depth 4 |
        Out-File (Join-Path $RunRoot "results.json") -Encoding utf8
    @($Results) | Export-Csv (Join-Path $RunRoot "results.csv") -NoTypeInformation

    @($Results | Group-Object status | Sort-Object Name | ForEach-Object {
        [pscustomobject]@{
            status = $_.Name
            count  = $_.Count
        }
    }) | ConvertTo-Json |
        Out-File (Join-Path $RunRoot "summary.json") -Encoding utf8

    [pscustomobject]@{
        state      = if ($Complete) { "complete" } else { "in-progress" }
        updated_at = (Get-Date).ToString("o")
        completed  = $Results.Count
    } | ConvertTo-Json |
        Out-File (Join-Path $RunRoot "progress.json") -Encoding utf8
}

function Invoke-TakoCaptured {
    param(
        [string]$Executable,
        [string[]]$Arguments,
        [string]$LogPath
    )

    $started = Get-Date
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $captured = @(& $Executable @Arguments 2>&1)
        $exitCode = $LASTEXITCODE
    } catch {
        $captured = @($_.Exception.Message)
        $exitCode = -1
    } finally {
        $watch.Stop()
    }

    $captured | Set-Content -LiteralPath $LogPath -Encoding utf8
    foreach ($line in $captured) {
        Write-Host "  $line"
    }

    return [pscustomobject]@{
        exit_code   = [int]$exitCode
        duration_ms = [long]$watch.ElapsedMilliseconds
        started_at  = $started.ToString("o")
        detail      = if ($captured.Count -gt 0) { "$($captured[-1])" } else { "" }
    }
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
Write-Host "Already verified models are reused, so this script is safe to rerun." -ForegroundColor DarkGray

$PreviousTakokitHome = $env:TAKOKIT_HOME
$env:TAKOKIT_HOME = $StorageRoot

try {
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
        $pull = Invoke-TakoCaptured $Tako @("pull", $model) $pullLog

        if ($pull.exit_code -ne 0) {
            Write-Host "  failed: pull exited with $($pull.exit_code)." -ForegroundColor Red
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
        $plan = Invoke-TakoCaptured $Tako @("plan", $model, "--json") $planLog
        $duration = $pull.duration_ms + $plan.duration_ms

        if ($plan.exit_code -eq 0) {
            Write-Host "  passed" -ForegroundColor Green
            $status = "passed"
            $detail = "Pull and readiness plan succeeded."
        } else {
            Write-Host "  failed: readiness plan exited with $($plan.exit_code)." -ForegroundColor Red
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

    Write-Host ""
    Write-Host "Installed model list" -ForegroundColor Cyan
    $installedLog = Join-Path $RunRoot "installed-models.log"
    $installed = Invoke-TakoCaptured $Tako @("list") $installedLog
    if ($installed.exit_code -ne 0) {
        Write-Warning "tako list failed; see $installedLog"
    }
} finally {
    $env:TAKOKIT_HOME = $PreviousTakokitHome
    Save-Results $Results $RunRoot -Complete $true
}

Write-Host ""
@($Results | Group-Object status | Sort-Object Name | ForEach-Object {
    [pscustomobject]@{ status = $_.Name; count = $_.Count }
}) | Format-Table -AutoSize
Write-Host "Evidence: $RunRoot"
Write-Host "Next: .\scripts\run_takokit_all_smokes.ps1 -SkipPull" -ForegroundColor Cyan

if ($Results.status -contains "failed") {
    exit 1
}
exit 0
