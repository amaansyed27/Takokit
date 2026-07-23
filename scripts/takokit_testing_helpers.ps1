$script:TakokitActiveProcess = $null

function Resolve-TakokitTestPath {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Label
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        throw "$Label does not exist: $Path"
    }
    return (Resolve-Path -LiteralPath $Path).Path
}

function ConvertTo-TakokitProcessArgument {
    param([AllowEmptyString()][string]$Value)

    if ($Value.Length -eq 0) { return '""' }
    if ($Value -notmatch '[\s"]') { return $Value }

    $escaped = $Value -replace '(\\*)"', '$1$1\"'
    $escaped = $escaped -replace '(\\+)$', '$1$1'
    return '"' + $escaped + '"'
}

function Stop-TakokitProcessTree {
    param([Parameter(Mandatory = $true)][int]$ProcessId)

    if (-not (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)) { return }

    if ($env:OS -eq "Windows_NT") {
        & taskkill.exe /PID $ProcessId /T /F 2>&1 | Out-Null
    } else {
        Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    }
}

function Stop-TakokitActiveTestProcess {
    if ($script:TakokitActiveProcess -and -not $script:TakokitActiveProcess.HasExited) {
        Stop-TakokitProcessTree -ProcessId $script:TakokitActiveProcess.Id
    }
    $script:TakokitActiveProcess = $null
}

function Get-TakokitLastUsefulLine {
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

function Get-TakokitTestLogPaths {
    param(
        [Parameter(Mandatory = $true)][string]$StorageRoot,
        [Parameter(Mandatory = $true)][datetime]$Since
    )

    $patterns = @(
        (Join-Path $StorageRoot "logs\*.log"),
        (Join-Path $StorageRoot "runners\*\logs\*.log"),
        (Join-Path $StorageRoot "runners\python-managed\adapters\*\*.log")
    )
    $files = @()
    foreach ($pattern in $patterns) {
        $files += @(
            Get-ChildItem -Path $pattern -File -ErrorAction SilentlyContinue |
                Where-Object { $_.LastWriteTime -ge $Since.AddSeconds(-2) }
        )
    }

    return @(
        $files |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 6 -ExpandProperty FullName
    )
}

function Test-TakokitFailureText {
    param([string]$Text)

    if ([string]::IsNullOrWhiteSpace($Text)) { return $false }
    return $Text -match '(?im)^\s*Failed after\b' -or
        $Text -match '(?im)^\s*Error:\s+' -or
        $Text -match '(?im)^\s*thread .* panicked'
}

function Invoke-TakokitDirectProcess {
    param(
        [Parameter(Mandatory = $true)][string]$Tako,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$LogPath,
        [scriptblock]$OnTick,
        [switch]$RequireExecutablePlan
    )

    $allArguments = @("--direct") + $Arguments
    $argumentLine = (@(
        $allArguments | ForEach-Object { ConvertTo-TakokitProcessArgument "$_" }
    )) -join ' '

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
    $processStarted = $false

    try {
        if (-not $process.Start()) { throw "Failed to start Takokit process." }
        $processStarted = $true
        $script:TakokitActiveProcess = $process
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()

        while (-not $process.WaitForExit(500)) {
            if ($OnTick) { & $OnTick $watch $startedAt }
        }

        $process.WaitForExit()
        $stdoutTask.Wait()
        $stderrTask.Wait()
        $process.Refresh()
        $stdout = "$($stdoutTask.Result)"
        $stderr = "$($stderrTask.Result)"
        $reportedExitCode = [int]$process.ExitCode
    } catch {
        $stderr = $_.Exception.Message
        $reportedExitCode = -1
    } finally {
        $watch.Stop()
        if ($processStarted -and -not $process.HasExited) {
            Stop-TakokitProcessTree -ProcessId $process.Id
        }
        $script:TakokitActiveProcess = $null
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

    $failed = $reportedExitCode -ne 0 -or (Test-TakokitFailureText $combined)
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

    $detail = if ($failed) {
        Get-TakokitLastUsefulLine @($LogPath)
    } elseif (-not [string]::IsNullOrWhiteSpace($stdout)) {
        ($stdout -split "`r?`n" | Where-Object { $_.Trim() } | Select-Object -Last 1)
    } else {
        "completed"
    }

    return [pscustomobject]@{
        exit_code = $effectiveExitCode
        reported_exit_code = $reportedExitCode
        duration_ms = [long]$watch.ElapsedMilliseconds
        stdout = $stdout
        stderr = $stderr
        json = $json
        detail = "$detail"
    }
}
