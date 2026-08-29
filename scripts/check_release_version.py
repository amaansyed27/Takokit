#!/usr/bin/env python3
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPECTED = "0.1.0"

checks = []


def cargo_version(path: Path):
    text = path.read_text(encoding="utf-8")
    match = re.search(r'(?m)^version\s*=\s*"([^"]+)"', text)
    if not match:
        raise RuntimeError(f"no version found in {path}")
    return match.group(1)


checks.append(("workspace Cargo version", cargo_version(ROOT / "Cargo.toml")))
checks.append(
    (
        "GUI package version",
        json.loads((ROOT / "apps/gui/package.json").read_text(encoding="utf-8"))["version"],
    )
)
checks.append(
    (
        "GUI lockfile version",
        json.loads((ROOT / "apps/gui/package-lock.json").read_text(encoding="utf-8"))[
            "version"
        ],
    )
)
checks.append(
    (
        "GUI lockfile root package version",
        json.loads((ROOT / "apps/gui/package-lock.json").read_text(encoding="utf-8"))[
            "packages"
        ][""]["version"],
    )
)
checks.append(
    (
        "site package version",
        json.loads((ROOT / "site/package.json").read_text(encoding="utf-8"))["version"],
    )
)
checks.append(
    (
        "site lockfile version",
        json.loads((ROOT / "site/package-lock.json").read_text(encoding="utf-8"))["version"],
    )
)
checks.append(
    (
        "site lockfile root package version",
        json.loads((ROOT / "site/package-lock.json").read_text(encoding="utf-8"))["packages"][""]["version"],
    )
)

installer = ROOT / "packaging/windows/Takokit.iss"
if installer.exists():
    text = installer.read_text(encoding="utf-8")
    match = re.search(r'(?m)^#define\s+MyAppVersion\s+"([^"]+)"', text)
    checks.append(("installer version", match.group(1) if match else "<missing>"))

release_ps1 = ROOT / "scripts/release/build-windows.ps1"
if release_ps1.exists():
    text = release_ps1.read_text(encoding="utf-8")
    match = re.search(r'\[string\]\$Version\s*=\s*"([^"]+)"', text)
    checks.append(("release script default", match.group(1) if match else "<missing>"))

for relative, label, pattern in [
    (
        "scripts/release/test-windows-distribution.ps1",
        "distribution acceptance version",
        r"(?m)^\$Version\s*=\s*'([^']+)'",
    ),
    (
        "scripts/release/test-windows-product-contract.ps1",
        "product contract version",
        r"(?m)^\$Version\s*=\s*'([^']+)'",
    ),
]:
    text = (ROOT / relative).read_text(encoding="utf-8")
    match = re.search(pattern, text)
    checks.append((label, match.group(1) if match else "<missing>"))

release_notes = ROOT / f"docs/release/windows-v{EXPECTED}-notes.md"
checks.append(("release notes filename", EXPECTED if release_notes.exists() else "<missing>"))

workflow = (ROOT / ".github/workflows/slice4-windows.yml").read_text(encoding="utf-8")
artifact_match = re.search(r"name:\s+takokit-v([0-9.]+)-windows-x86_64-", workflow)
checks.append(("Windows CI artifact version", artifact_match.group(1) if artifact_match else "<missing>"))

failed = False
for label, version in checks:
    if version != EXPECTED:
        print(f"ERROR: {label} is {version!r}; expected {EXPECTED!r}")
        failed = True
    else:
        print(f"ok: {label} = {version}")

if failed:
    sys.exit(1)
