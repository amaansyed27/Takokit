#!/usr/bin/env python3
"""Deterministic documentation drift checks for release-facing Takokit facts."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def require(text: str, needle: str, where: str) -> None:
    if needle not in text:
        raise SystemExit(f"documentation drift: {where} is missing {needle!r}")


def forbid(text: str, needle: str, where: str) -> None:
    if needle.lower() in text.lower():
        raise SystemExit(f"documentation drift: {where} still contains stale {needle!r}")


def check_markdown_links() -> None:
    pattern = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
    for path in [ROOT / "README.md", ROOT / "CONTRIBUTING.md", *(ROOT / "docs").glob("*.md")]:
        text = path.read_text(encoding="utf-8")
        for target in pattern.findall(text):
            target = target.strip().split("#", 1)[0]
            if not target or target.startswith(("http://", "https://", "mailto:")):
                continue
            if target.startswith("<") and target.endswith(">"):
                target = target[1:-1]
            resolved = (path.parent / target).resolve()
            try:
                resolved.relative_to(ROOT.resolve())
            except ValueError as exc:
                raise SystemExit(f"documentation link escapes repository: {path}: {target}") from exc
            if not resolved.exists():
                raise SystemExit(f"broken documentation link: {path.relative_to(ROOT)} -> {target}")


def check_cli_contract() -> None:
    args = read("apps/cli/src/args.rs")
    interfaces = read("site/src/docs/pages/interfaces.js")
    mapping = {
        "Start": "tako start", "Stop": "tako stop", "Serve": "tako serve", "Server": "tako server",
        "Gui": "tako gui", "Doctor": "tako doctor", "Version": "tako version", "Status": "tako status",
        "Storage": "tako storage", "Update": "tako update", "Reset": "tako reset", "Licenses": "tako licenses",
        "Capabilities": "tako capabilities", "Models": "tako models", "Runners": "tako runners",
        "CustomModel": "tako custom-model", "Voice": "tako voice", "Library": "tako library",
        "Speak": "tako speak", "Pull": "tako pull", "Show": "tako show", "Plan": "tako plan",
        "Rm": "tako rm", "List": "tako list", "Run": "tako run", "Ps": "tako ps",
        "Runner": "tako runner", "Adapter": "tako adapter", "Sessions": "tako sessions",
        "Quickstart": "tako quickstart", "Deps": "tako deps", "Samples": "tako samples",
        "Test": "tako test", "Transcribe": "tako transcribe", "Clone": "tako clone",
        "Convert": "tako convert", "Train": "tako train",
    }
    for variant, command in mapping.items():
        if not re.search(rf"^\s*{re.escape(variant)}(?:\s*\{{|\s*\(|,)", args, re.MULTILINE):
            raise SystemExit(f"CLI drift checker expected missing source variant: {variant}")
        require(interfaces, command, "public CLI reference")

    for global_flag in ["--direct", "--output", "--workspace", "--session"]:
        require(interfaces, global_flag, "public CLI reference")

    rvc = read("apps/cli/src/args/rvc.rs")
    voice_docs = read("site/src/docs/pages/voice-workflows.js")
    for variant in [
        "Create", "Samples", "Inspect", "Presets", "Preflight", "Prepare", "Train", "Status",
        "Logs", "Cancel", "Recover", "Checkpoints", "Indexes", "Activate", "Test", "Import",
        "Export", "Verify", "ImportPackage", "Remove",
    ]:
        if variant not in rvc:
            raise SystemExit(f"RVC source command disappeared: {variant}")
    for command in [
        "tako voice rvc create", "tako voice rvc preflight", "tako voice rvc train",
        "tako voice rvc export", "tako voice rvc verify",
    ]:
        require(voice_docs, command, "Advanced RVC docs")


def check_api_contract() -> None:
    inventory = read("docs/api-route-inventory.md")
    developer = read("site/src/docs/pages/developers.js")
    for route in [
        "GET /v1/models", "GET /v1/models/{model}", "POST /v1/audio/speech",
        "POST /v1/audio/transcriptions",
    ]:
        require(inventory, route, "API route inventory")
    require(inventory, "/api/v1", "API route inventory")
    require(inventory, "audio-only subset", "API route inventory")
    require(developer, "OpenAI-compatible audio", "public API docs")
    for unsupported in ["Chat completions", "Responses", "Embeddings", "Images"]:
        require(developer, unsupported, "public API compatibility matrix")
    # Negative warnings such as “do not describe Takokit as a general OpenAI-compatible
    # server” are intentional. Only positive support claims are forbidden here.
    if re.search(r"\b(?:supports?|provides?)\s+(?:full|general)\s+OpenAI", developer, re.I):
        raise SystemExit("public docs overclaim OpenAI compatibility")


def check_release_and_platform_copy() -> None:
    files = {
        "README.md": read("README.md"),
        "getting-started": read("site/src/docs/pages/getting-started.js"),
        "release-docs": read("site/src/docs/pages/release.js"),
        "platform-ui": read("site/src/lib/platform.js"),
        "download-page": read("site/src/pages/DownloadPage.jsx"),
    }
    combined = "\n".join(files.values())
    for required in [
        "v0.3.0", "Windows x86_64", "Linux x86_64", "Apple Silicon",
        "install.ps1", "install.sh", "1.0.0",
    ]:
        require(combined, required, "release-facing documentation")
    require(files["release-docs"], "every 0.x.x release is Beta", "beta policy")
    for stale in [
        "Linux and macOS packages are coming later", "Packaging is not available yet",
        "One Windows runtime", "apps/desktop", "planned separately under Issue #68",
    ]:
        for where, text in files.items():
            forbid(text, stale, where)


def main() -> None:
    check_markdown_links()
    check_cli_contract()
    check_api_contract()
    check_release_and_platform_copy()
    print("Takokit documentation contracts passed")


if __name__ == "__main__":
    main()
