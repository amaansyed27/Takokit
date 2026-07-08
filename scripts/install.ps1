$ErrorActionPreference = "Stop"

Write-Host "Takokit release downloads are not published yet."
Write-Host "This installer is scaffolded for future release distribution and will not download binaries today."
Write-Host ""

$os = if ($IsWindows -or $env:OS -eq "Windows_NT") {
    "windows"
} elseif ($IsMacOS) {
    "macos"
} elseif ($IsLinux) {
    "linux"
} else {
    [System.Runtime.InteropServices.RuntimeInformation]::OSDescription.ToLowerInvariant()
}

$archName = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant()
$arch = switch ($archName) {
    "x64" { "x64" }
    "arm64" { "arm64" }
    default { $archName }
}

$artifact = "takokit-$os-$arch.zip"
$releaseBaseUrl = "https://github.com/amaansyed27/Takokit/releases/latest/download"
$futureUrl = "$releaseBaseUrl/$artifact"

Write-Host "Detected target: $os-$arch"
Write-Host "Future artifact: $artifact"
Write-Host "Future URL: $futureUrl"
Write-Host ""
Write-Host "Future installer flow:"
Write-Host "  1. Download $artifact from GitHub Releases."
Write-Host "  2. Verify a published SHA256 checksum."
Write-Host "  3. Install takokit.exe into a user-local bin directory on PATH."
Write-Host "  4. Run: takokit doctor"
Write-Host ""
Write-Host "No installation was performed."
