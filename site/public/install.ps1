$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repository = "https://github.com/amaansyed27/Takokit.git"
$installRoot = if ($env:TAKOKIT_INSTALL_DIR) {
    $env:TAKOKIT_INSTALL_DIR
} else {
    Join-Path $env:LOCALAPPDATA "Takokit\bin"
}

foreach ($commandName in @("git", "cargo", "npm")) {
    if (-not (Get-Command $commandName -ErrorAction SilentlyContinue)) {
        throw "Missing required command: $commandName. Install Git, Rust stable, Node.js LTS, and npm, then run this command again."
    }
}

$workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("takokit-install-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $workDir | Out-Null

try {
    Write-Host "Downloading Takokit source..."
    git clone --depth 1 $repository (Join-Path $workDir "Takokit")

    Push-Location (Join-Path $workDir "Takokit\apps\gui")
    try {
        npm ci
        npm run build
    } finally {
        Pop-Location
    }

    Push-Location (Join-Path $workDir "Takokit")
    try {
        cargo build --release --locked
    } finally {
        Pop-Location
    }

    $binary = Join-Path $workDir "Takokit\target\release\tako.exe"
    if (-not (Test-Path $binary)) {
        $binary = Join-Path $workDir "Takokit\target\release\takokit.exe"
    }
    if (-not (Test-Path $binary)) {
        throw "The Takokit binary was not produced by the build."
    }

    New-Item -ItemType Directory -Force -Path $installRoot | Out-Null
    $installedBinary = Join-Path $installRoot "tako.exe"
    Copy-Item -Force $binary $installedBinary

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $pathEntries = @($userPath -split ";" | Where-Object { $_ })
    if ($pathEntries -notcontains $installRoot) {
        $updatedPath = (@($pathEntries) + $installRoot) -join ";"
        [Environment]::SetEnvironmentVariable("Path", $updatedPath, "User")
    }
    if (($env:Path -split ";") -notcontains $installRoot) {
        $env:Path = "$installRoot;$env:Path"
    }

    Write-Host "Takokit installed at $installedBinary"
    & $installedBinary version
} finally {
    Remove-Item -Recurse -Force $workDir -ErrorAction SilentlyContinue
}
