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
            if ($line.Length -gt 150) {
                $line = $line.Substring(0, 147) + "..."
            }
            return $line
        }
    }

    return ""
}

function Get-ReportedPercent {
    param([string[]]$Paths)

    $matches = foreach ($path in $Paths) {
        if (-not $path -or -not (Test-Path -LiteralPath $path -PathType Leaf)) {
            continue
        }

        Get-Content -LiteralPath $path -Tail 40 -ErrorAction SilentlyContinue |
            ForEach-Object {
                foreach ($match in [regex]::Matches("$_", '(?<!\d)(\d{1,3}(?:\.\d+)?)\s*%')) {
                    [double]$match.Groups[1].Value
                }
            }
    }

    if (@($matches).Count -eq 0) {
        return $null
    }

    return [Math]::Max(0, [Math]::Min(100, [double]@($matches)[-1]))
}

function New-PulseBar {
    param(
        [long]$ElapsedMilliseconds,
        [int]$Width = 28
    )

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

function Get-TakokitLiveLogPaths {
    param([datetime]$Since)

    if (-not $env:TAKOKIT_HOME) {
        return @()
    }

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

function Write-ModelProgress {
    param(
        [string]$Model,
        [string]$Phase,
        [int]$ModelNumber,
        [int]$ModelCount,
        [System.Diagnostics.Stopwatch]$Watch,
        [string[]]$OutputPaths,
        [datetime]$StartedAt
    )

    $reportedPercent = Get-ReportedPercent $OutputPaths
    $completedBefore = $ModelNumber - 1
    $phaseFraction = if ($null -ne $reportedPercent) { $reportedPercent / 100.0 } else { 0.0 }
    $overallPercent = [Math]::Min(
        99,
        [Math]::Floor((($completedBefore + $phaseFraction) / $ModelCount) * 100)
    )

    Write-Progress `
        -Id 1 `
        -Activity "Takokit all-model prefetch" `
        -Status "[$ModelNumber/$ModelCount] $Model - $Phase" `
        -PercentComplete $overallPercent

    $liveLogs = @(Get-TakokitLiveLogPaths -Since $StartedAt)
    $latest = Get-LastUsefulLine @($OutputPaths + $liveLogs)
    $elapsed = "{0:hh\:mm\:ss}" -f $Watch.Elapsed

    if ($null -ne $reportedPercent) {
        $phasePercent = [Math]::Floor($reportedPercent)
        $status = "$phasePercent% | elapsed $elapsed"
        if ($latest) {
            $status += " | $latest"
        }

        Write-Progress `
            -Id 2 `
            -ParentId 1 `
            -Activity "$Phase $Model" `
            -Status $status `
            -PercentComplete $phasePercent
    } else {
        $pulse = New-PulseBar -ElapsedMilliseconds $Watch.ElapsedMilliseconds
        $status = "$pulse elapsed $elapsed"
        if ($latest) {
            $status += " | $latest"
        }

        Write-Progress `
            -Id 2 `
            -ParentId 1 `
            -Activity "$Phase $Model" `
            -Status $status `
            -PercentComplete -1
    }
}

function Invoke-TakoCaptured {
    param(
        [string]$Executable,
        [string[]]$Arguments,
        [string]$LogPath,
        [string]$Model,
        [string]$Phase,
        [int]$ModelNumber,
        [int]$ModelCount
    )

    $started = Get-Date
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $stdoutLog = "$LogPath.stdout.tmp"
    $stderrLog = "$LogPath.stderr.tmp"
    $process = $null
    $exitCode = -1

    Remove-Item -LiteralPath $stdoutLog, $stderrLog -Force -ErrorAction SilentlyContinue

    try {
        $process = Start-Process `
            -FilePath $Executable `
            -ArgumentList $Arguments `
            -WorkingDirectory (Get-Location).Path `
            -RedirectStandardOutput $stdoutLog `
            -RedirectStandardError $stderrLog `
            -NoNewWindow `
            -PassThru

        while (-not $process.HasExited) {
            Write-ModelProgress `
                -Model $Model `
                -Phase $Phase `
                -ModelNumber $ModelNumber `
                -ModelCount $ModelCount `
                -Watch $watch `
                -OutputPaths @($stderrLog, $stdoutLog) `
                -StartedAt $started
            Start-Sleep -Milliseconds 350
            $process.Refresh()
        }

        $process.WaitForExit()
        $process.Refresh()
        $exitCode = [int]$process.ExitCode
    } catch {
        $_.Exception.Message | Set-Content -LiteralPath $stderrLog -Encoding utf8
        $exitCode = -1
    } finally {
        $watch.Stop()
        Write-Progress -Id 2 -ParentId 1 -Activity "$Phase $Model" -Completed

        if ($process) {
            try {
                if (-not $process.HasExited) {
                    $process.Kill()
                }
            } catch {}
            $process.Dispose()
        }
    }

    "=== stdout ===" | Set-Content -LiteralPath $LogPath -Encoding utf8
    if (Test-Path -LiteralPath $stdoutLog) {
        Get-Content -LiteralPath $stdoutLog | Add-Content -LiteralPath $LogPath -Encoding utf8
    }
    "=== stderr ===" | Add-Content -LiteralPath $LogPath -Encoding utf8
    if (Test-Path -LiteralPath $stderrLog) {
        Get-Content -LiteralPath $stderrLog | Add-Content -LiteralPath $LogPath -Encoding utf8
    }
    "[exit code: $exitCode]" | Add-Content -LiteralPath $LogPath -Encoding utf8

    $detail = Get-LastUsefulLine @($stderrLog, $stdoutLog, $LogPath)
    Remove-Item -LiteralPath $stdoutLog, $stderrLog -Force -ErrorAction SilentlyContinue

    return [pscustomobject]@{
        exit_code   = [int]$exitCode
        duration_ms = [long]$watch.ElapsedMilliseconds
        started_at  = $started.ToString("o")
        detail      = $detail
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
Write-Host "Live progress and complete logs remain visible during long installs." -ForegroundColor DarkGray

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

$env:TAKOKIT_HOME = $StorageRoot
$RunComplete = $false

try {
    for ($index = 0; $index -lt $Models.Count; $index++) {
        $entry = $Models[$index]
        $model = $entry.Id
        $number = $index + 1
        Write-Host ""
        Write-Host "[$number/$($Models.Count)] $model" -ForegroundColor Cyan

        Write-Progress `
            -Id 1 `
            -Activity "Takokit all-model prefetch" `
            -Status "[$number/$($Models.Count)] $model" `
            -PercentComplete ([Math]::Floor(($index / $Models.Count) * 100))

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
        $pull = Invoke-TakoCaptured `
            -Executable $Tako `
            -Arguments @("pull", $model) `
            -LogPath $pullLog `
            -Model $model `
            -Phase "Pulling" `
            -ModelNumber $number `
            -ModelCount $Models.Count

        if ($pull.exit_code -ne 0) {
            Write-Host "  failed: pull exited with $($pull.exit_code)." -ForegroundColor Red
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
        $plan = Invoke-TakoCaptured `
            -Executable $Tako `
            -Arguments @("plan", $model, "--json") `
            -LogPath $planLog `
            -Model $model `
            -Phase "Verifying readiness" `
            -ModelNumber $number `
            -ModelCount $Models.Count
        $duration = $pull.duration_ms + $plan.duration_ms

        if ($plan.exit_code -eq 0) {
            $elapsed = [TimeSpan]::FromMilliseconds($duration)
            Write-Host ("  passed in {0:hh\:mm\:ss}" -f $elapsed) -ForegroundColor Green
            $status = "passed"
            $detail = "Pull and readiness plan succeeded."
        } else {
            Write-Host "  failed: readiness plan exited with $($plan.exit_code)." -ForegroundColor Red
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

    Write-Progress -Id 1 -Activity "Takokit all-model prefetch" -Status "Preparing installed model list" -PercentComplete 99
    Write-Host ""
    Write-Host "Installed model list" -ForegroundColor Cyan
    $installedLog = Join-Path $RunRoot "installed-models.log"
    $installed = Invoke-TakoCaptured `
        -Executable $Tako `
        -Arguments @("list") `
        -LogPath $installedLog `
        -Model "catalog" `
        -Phase "Listing installed models" `
        -ModelNumber $Models.Count `
        -ModelCount $Models.Count
    if ($installed.exit_code -ne 0) {
        Write-Warning "tako list failed; see $installedLog"
    }

    $RunComplete = $true
} finally {
    Write-Progress -Id 2 -ParentId 1 -Activity "Takokit model operation" -Completed
    Write-Progress -Id 1 -Activity "Takokit all-model prefetch" -Completed
    $env:TAKOKIT_HOME = $PreviousTakokitHome
    $ProgressPreference = $PreviousProgressPreference
    if ($ProgressViewChanged) {
        $PSStyle.Progress.View = $PreviousProgressView
    }
    Save-Results $Results $RunRoot -Complete $RunComplete
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
