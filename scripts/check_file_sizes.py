#!/usr/bin/env python3
"""Backward-compatible entrypoint for the repository source-size audit.

Kept so older release-test instructions continue to work.
"""

from __future__ import annotations

import runpy
from pathlib import Path


if __name__ == "__main__":
    runpy.run_path(str(Path(__file__).with_name("audit_file_sizes.py")), run_name="__main__")
