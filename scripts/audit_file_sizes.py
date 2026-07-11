#!/usr/bin/env python3
"""Audit tracked source files for modularity risks.

The script intentionally ignores generated dependency lockfiles and binary assets.
It fails when a human-maintained source/config/documentation file exceeds the hard
limit and reports near-limit files so they can be split before becoming god files.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

SOURCE_SUFFIXES = {
    ".rs",
    ".ts",
    ".tsx",
    ".js",
    ".jsx",
    ".css",
    ".scss",
    ".py",
    ".sh",
    ".ps1",
    ".toml",
    ".yml",
    ".yaml",
    ".md",
}

EXCLUDED_NAMES = {
    "Cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
}

EXCLUDED_PARTS = {
    "target",
    "node_modules",
    "dist",
    "build",
    ".git",
    "vendor",
    "generated",
}


@dataclass(frozen=True)
class FileSize:
    path: Path
    lines: int


def tracked_files(root: Path) -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return [root / item.decode() for item in result.stdout.split(b"\0") if item]


def should_audit(path: Path, root: Path) -> bool:
    relative = path.relative_to(root)
    if path.name in EXCLUDED_NAMES:
        return False
    if any(part in EXCLUDED_PARTS for part in relative.parts):
        return False
    return path.suffix.lower() in SOURCE_SUFFIXES


def line_count(path: Path) -> int | None:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return sum(1 for _ in handle)
    except (UnicodeDecodeError, OSError):
        return None


def audit(root: Path) -> list[FileSize]:
    rows: list[FileSize] = []
    for path in tracked_files(root):
        if not path.is_file() or not should_audit(path, root):
            continue
        lines = line_count(path)
        if lines is not None:
            rows.append(FileSize(path.relative_to(root), lines))
    return sorted(rows, key=lambda row: (-row.lines, str(row.path)))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--limit", type=int, default=500)
    parser.add_argument("--warn", type=int, default=400)
    args = parser.parse_args()

    if args.warn >= args.limit:
        parser.error("--warn must be lower than --limit")

    root = Path(__file__).resolve().parents[1]
    rows = audit(root)
    oversized = [row for row in rows if row.lines > args.limit]
    warnings = [row for row in rows if args.warn <= row.lines <= args.limit]

    print(f"Audited {len(rows)} tracked source/config/documentation files.")
    print(f"Hard limit: {args.limit} lines; warning threshold: {args.warn} lines.")

    if oversized:
        print("\nFiles over the hard limit:")
        for row in oversized:
            print(f"  {row.lines:>5}  {row.path}")
    else:
        print("\nNo files exceed the hard limit.")

    if warnings:
        print("\nFiles approaching the limit:")
        for row in warnings:
            print(f"  {row.lines:>5}  {row.path}")

    print("\nLargest audited files:")
    for row in rows[:15]:
        print(f"  {row.lines:>5}  {row.path}")

    return 1 if oversized else 0


if __name__ == "__main__":
    sys.exit(main())
