[CmdletBinding()]
param(
    [string]$StorageRoot = (Join-Path $env:TEMP "takokit-all-model-smoke"),
    [string]$Tako = ".\target\release\tako.exe",
    [string]$EvidenceRoot = "$HOME\takokit-test-evidence",
    [switch]$IncludeWorkstation
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false
. (Join-Path $PSScriptRoot "takokit_testing_helpers.ps1")

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

function Test-ReadyRecord {
    param([string]$Model)
    $record = Join-Path $StorageRoot "manifests\installed-models\$Model.toml"
    $manifest = Join-Path $StorageRoot "manifests\models\$Model.toml"
    if (-not (Test-Path -LiteralPath $record -PathType Leaf)) { return $false }
    if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) { return $false }
    return (Get-Content -LiteralPath $record -Raw) -match '(?im)^\s*status\s*=\s*"ready"\s*$'
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

$Tako = Resolve-TakokitTestPath $Tako "Takokit executable"
$StorageRoot = [System.IO.Path]::GetFullPath([Environment]::ExpandEnvironmentVariables($StorageRoot))
$EvidenceRoot = [System.IO.Path]::GetFullPath([Environment]::ExpandEnvironmentVariables($EvidenceRoot))
New-Item -ItemType Directory -Force $StorageRoot, $EvidenceRoot | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$RunRoot = Join-Path $EvidenceRoot "pull-all-models-$stamp"
New-Item -ItemType Directory -Force $RunRoot | Out-Null
$Results = [System.Collections.Generic.List[object]]::new()
$FatalError = ""
$RunComplete = $false

Write-Host ""
Write-Host "Takokit sequential all-model prefetch" -ForegroundColor Cyan
Write-Host "Models:   $($Models.Count)"
Write-Host "Storage:  $StorageRoot"
Write-Host "Evidence: $RunRoot"
Write-Host "Mode:     direct; port 5050 is not used" -ForegroundColor Green
Write-Host "Interrupted downloads remain cached and resume or revalidate on rerun." -ForegroundColor DarkGray

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
        $tick = {
            param($Watch, $StartedAt)
            $overall = [Math]::Min(99, [Math]::Floor((($number - 1) / $Models.Count) * 100))
            Write-Progress -Id 1 -Activity "Takokit all-model prefetch" -Status "[$number/$($Models.Count)] $model" -PercentComplete $overall
            $latest = Get-TakokitLastUsefulLine @(Get-TakokitTestLogPaths -StorageRoot $StorageRoot -Since $StartedAt)
            $status = "$(New-PulseBar -ElapsedMilliseconds $Watch.ElapsedMilliseconds) elapsed $(('{0:hh\:mm\:ss}' -f $Watch.Elapsed))"
            if ($latest) { $status += " | $latest" }
            Write-Progress -Id 2 -ParentId 1 -Activity "Pulling $model" -Status $status -PercentComplete -1
        }

        Write-Host "  Pulling directly..."
        Write-Host "  Log: $pullLog" -ForegroundColor DarkGray
        $pull = Invoke-TakokitDirectProcess -Tako $Tako -Arguments @("pull", $model) -LogPath $pullLog -OnTick $tick
        Write-Progress -Id 2 -ParentId 1 -Activity "Pulling $model" -Completed

        if ($pull.exit_code -ne 0) {
            Write-Host "  failed: $($pull.detail)" -ForegroundColor Red
            $Results.Add([pscustomobject]@{
                model = $model; status = "failed"; duration_ms = $pull.duration_ms
                pull_log = $pullLog; plan_log = ""; detail = $pull.detail
            }) | Out-Null
            Save-Results $Results $RunRoot
            continue
        }

        Write-Host "  Verifying executable plan..."
        $plan = Invoke-TakokitDirectProcess -Tako $Tako -Arguments @("plan", $model, "--json") -LogPath $planLog -RequireExecutablePlan
        $duration = $pull.duration_ms + $plan.duration_ms

        if ($plan.exit_code -ne 0 -or -not (Test-ReadyRecord $model)) {
            $detail = if ($plan.exit_code -ne 0) { $plan.detail } else { "ready install record is missing" }
            Write-Host "  failed: $detail" -ForegroundColor Red
            $status = "failed"
        } else {
            Write-Host ("  passed in {0:hh\:mm\:ss}" -f [TimeSpan]::FromMilliseconds($duration)) -ForegroundColor Green
            $detail = "Direct pull, ready record and executable plan verified."
            $status = "passed"
        }

        $Results.Add([pscustomobject]@{
            model = $model; status = $status; duration_ms = $duration
            pull_log = $pullLog; plan_log = $planLog; detail = $detail
        }) | Out-Null
        Save-Results $Results $RunRoot
    }

    $readyRecords = @(
        Get-ChildItem -LiteralPath (Join-Path $StorageRoot "manifests\installed-models") -Filter "*.toml" -File -ErrorAction SilentlyContinue |
            Where-Object { (Get-Content -LiteralPath $_.FullName -Raw) -match '(?im)^\s*status\s*=\s*"ready"\s*$' } |
            Select-Object -ExpandProperty BaseName
    )
    $missingPassed = @(
        $Results |
            Where-Object status -eq "passed" |
            Select-Object -ExpandProperty model |
            Where-Object { $readyRecords -notcontains $_ }
    )
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
    Stop-TakokitActiveTestProcess
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
