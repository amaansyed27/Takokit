#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
path = root / "apps/cli/src/tui/input/forms.rs"
source = path.read_text(encoding="utf-8")
marker = "#[cfg(test)]\nmod tests {\n"
if source.count(marker) != 1:
    raise RuntimeError("expected exactly one inline forms test module")
before, body = source.split(marker, 1)
body = body.rstrip()
if not body.endswith("}"):
    raise RuntimeError("forms test module is missing its closing brace")
body = body[:-1].rstrip() + "\n"
tests = root / "apps/cli/src/tui/input/forms/tests.rs"
tests.parent.mkdir(parents=True, exist_ok=True)
tests.write_text(body, encoding="utf-8")
path.write_text(before.rstrip() + "\n\n#[cfg(test)]\nmod tests;\n", encoding="utf-8")
