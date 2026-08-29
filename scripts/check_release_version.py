#!/usr/bin/env python3
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPECTED = "0.0.1"

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

failed = False
for label, version in checks:
    if version != EXPECTED:
        print(f"ERROR: {label} is {version!r}; expected {EXPECTED!r}")
        failed = True
    else:
        print(f"ok: {label} = {version}")

# Guard against accidental public product versions in release-facing metadata only.
for path in [ROOT / "apps/gui/package.json", ROOT / "apps/gui/package-lock.json"]:
    text = path.read_text(encoding="utf-8")
    if '"0.1.0"' in text or '"0.2.0"' in text:
        print(f"ERROR: stale public product version found in {path.relative_to(ROOT)}")
        failed = True

if failed:
    sys.exit(1)
