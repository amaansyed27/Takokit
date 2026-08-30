#!/usr/bin/env python3
"""Fail when the Slice 5 public namespaces, OpenAPI, or bundled clients drift."""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
router = (ROOT / "crates/takokit-server/src/router.rs").read_text(encoding="utf-8")
openai = (ROOT / "crates/takokit-server/src/handlers/openai.rs").read_text(encoding="utf-8")
required_router = ['"/v1/models"', '"/v1/audio/speech"', '"/v1/audio/transcriptions"', '.nest("/api/v1", native_router())', '"/openapi.json"']
required_openapi = ['"/v1/models"', '"/v1/audio/speech"', '"/v1/audio/transcriptions"', '"/api/v1/models"', '"/api/v1/audio/speech"', '"/api/v1/audio/transcriptions"']
errors = [f"router missing {item}" for item in required_router if item not in router]
errors += [f"OpenAPI missing {item}" for item in required_openapi if item not in openai]
for path in (ROOT / "apps/gui/src").rglob("*"):
    if path.suffix not in {".ts", ".tsx"}:
        continue
    text = path.read_text(encoding="utf-8")
    for route in ("/v1/models", "/v1/audio/speech", "/v1/audio/transcriptions"):
        if route in text and f"/api{route}" not in text:
            errors.append(f"native GUI client uses conflicting route {route}: {path.relative_to(ROOT)}")
if errors:
    print("\n".join(f"ERROR: {error}" for error in errors))
    sys.exit(1)
print("ok: router, OpenAPI, and bundled native client namespace contract")
