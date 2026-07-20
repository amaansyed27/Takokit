#!/usr/bin/env python3
"""Compatibility entrypoint for the Takokit source-size audit.

The canonical implementation lives in ``audit_file_sizes.py``.  Keeping this
small wrapper means older test notes and external automation continue to work
without duplicating the audit logic.
"""

from __future__ import annotations

import runpy
from pathlib import Path


def main() -> None:
    audit_script = Path(__file__).with_name("audit_file_sizes.py")
    if not audit_script.is_file():
        raise FileNotFoundError(f"Takokit file-size audit is missing: {audit_script}")
    runpy.run_path(str(audit_script), run_name="__main__")


if __name__ == "__main__":
    main()
