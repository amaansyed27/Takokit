# Shared managed-Python storage regression

This focused Windows pass verifies that Takokit stores heavy Python dependencies once per
Python ABI rather than once per adapter. It uses `mms-tts-eng` and `openvoice` for Python
3.11, then `rvc` for Python 3.10.

The adapter installer resolves the complete dependency graph first. It then removes only
adapter distributions whose normalized package name and version exactly match the
Takokit-owned ABI base. Genuine adapter version conflicts remain local. The UV cache is
never used as a runtime target and may be cleared without breaking installed adapters.

## Pull the test matrix

From the repository root in PowerShell:

```powershell
$env:TAKOKIT_HOME = "$HOME\.takokit"
$Tako = (Resolve-Path .\target\release\tako.exe).Path

& $Tako pull mms-tts-eng
& $Tako pull openvoice
& $Tako pull rvc
```

`mms-tts-eng` installs the `hf_audio` adapter. The first two adapters must inherit the
same Python 3.11 base. RVC must inherit a separate Python 3.10 base because compiled
Python extensions cannot be shared safely across ABIs.

## Verify dependency origins

```powershell
$Adapters = @("hf_audio", "openvoice", "rvc")
$TorchOrigins = @{}

foreach ($Adapter in $Adapters) {
  $AdapterRoot = Join-Path $env:TAKOKIT_HOME `
    "runners\python-managed\adapters\$Adapter"
  $Python = Join-Path $AdapterRoot "venv\Scripts\python.exe"
  $LocalTorch = Join-Path $AdapterRoot "venv\Lib\site-packages\torch"

  if (Test-Path -LiteralPath $LocalTorch) {
    throw "$Adapter kept an avoidable local Torch copy: $LocalTorch"
  }

  $TorchOrigins[$Adapter] = & $Python -I -c `
    "import torch; print(torch.__file__)"
  $ManagedPython = Join-Path $env:TAKOKIT_HOME "tools\python"
  if (-not $TorchOrigins[$Adapter].StartsWith(
      $ManagedPython,
      [StringComparison]::OrdinalIgnoreCase
  )) {
    throw "$Adapter did not inherit Torch from a Takokit-owned ABI base"
  }
}

if ($TorchOrigins["hf_audio"] -ne $TorchOrigins["openvoice"]) {
  throw "Python 3.11 adapters did not share the same Torch installation"
}
if ($TorchOrigins["rvc"] -eq $TorchOrigins["openvoice"]) {
  throw "Python 3.10 and 3.11 unexpectedly shared one binary Torch installation"
}

$TorchOrigins
```

Expected: `hf_audio` and `openvoice` print the same path below
`$env:TAKOKIT_HOME\tools\python`; RVC prints a different managed-Python path. None of the
three adapter `site-packages` directories contains its own `torch` directory.

## Inspect inherited packages

```powershell
Get-ChildItem `
  "$env:TAKOKIT_HOME\runners\python-managed\adapters" `
  -Filter ".takokit-inherited-packages.txt" `
  -File -Recurse |
  ForEach-Object {
    [PSCustomObject]@{
      Adapter = Split-Path $_.DirectoryName -Leaf
      SharedPackages = (Get-Content $_.FullName) -join ", "
    }
  } |
  Format-Table -AutoSize
```

Each successful heavy adapter should list exact duplicates that were removed from its
thin overlay and inherited from the ABI base. A package omitted from this file either is
adapter-specific or has a genuinely different version.
