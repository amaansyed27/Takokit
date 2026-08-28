#!/usr/bin/env python3
"""Generate release-bundled third-party dependency notices.

This intentionally lists application/runtime build dependencies only. Model-specific
licenses remain in the Takokit registry and pull/acceptance flow and are not copied
into the application distribution.
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def cargo_metadata() -> dict:
    return json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--format-version", "1", "--locked"],
            cwd=ROOT,
            text=True,
            encoding="utf-8",
            errors="strict",
        )
    )


def rust_dependencies(metadata: dict) -> list[tuple[str, str, str, str]]:
    workspace = set(metadata.get("workspace_members", []))
    rows = []
    for package in metadata.get("packages", []):
        if package.get("id") in workspace:
            continue
        rows.append(
            (
                package.get("name", "unknown"),
                package.get("version", "unknown"),
                package.get("license") or "UNKNOWN",
                package.get("repository") or package.get("homepage") or "",
            )
        )
    return rows


def npm_dependencies(lockfile: Path) -> list[tuple[str, str, str, str]]:
    data = json.loads(lockfile.read_text(encoding="utf-8"))
    rows = []
    for package_path, package in data.get("packages", {}).items():
        if not package_path.startswith("node_modules/"):
            continue
        name = package_path.removeprefix("node_modules/")
        rows.append(
            (
                name,
                str(package.get("version", "unknown")),
                str(package.get("license") or "UNKNOWN"),
                str(package.get("resolved") or ""),
            )
        )
    return rows


def dedupe(rows: list[tuple[str, str, str, str]]) -> list[tuple[str, str, str, str]]:
    unique: dict[tuple[str, str], tuple[str, str, str, str]] = {}
    for row in rows:
        unique.setdefault((row[0].lower(), row[1]), row)
    return sorted(unique.values(), key=lambda row: (row[0].lower(), row[1]))


def markdown_table(rows: list[tuple[str, str, str, str]]) -> list[str]:
    lines = [
        "| Package | Version | Declared license | Source |",
        "| --- | --- | --- | --- |",
    ]
    for name, version, license_id, source in dedupe(rows):
        safe_source = source.replace("|", "%7C")
        lines.append(f"| `{name}` | `{version}` | `{license_id}` | {safe_source} |")
    return lines


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    rust = rust_dependencies(cargo_metadata())
    npm = npm_dependencies(ROOT / "apps" / "gui" / "package-lock.json")

    lines = [
        "# Takokit third-party notices",
        "",
        "This file records third-party packages used to build or ship the Takokit Windows application.",
        "It is generated from the locked Rust and GUI dependency metadata for the exact release source tree.",
        "",
        "Model weights are downloaded separately and are governed by the model-specific license metadata in the Takokit registry and pull flow; those model licenses are intentionally not duplicated here.",
        "",
        "## Rust dependencies",
        "",
        *markdown_table(rust),
        "",
        "## GUI dependencies",
        "",
        *markdown_table(npm),
        "",
        "`UNKNOWN` means the package metadata did not declare an SPDX/license string. The upstream package remains subject to its own distributed license files and terms.",
        "",
    ]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("\n".join(lines), encoding="utf-8")


if __name__ == "__main__":
    main()
