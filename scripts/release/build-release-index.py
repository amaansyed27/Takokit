#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", action="append", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    manifests = []
    for raw in args.input:
        path = Path(raw)
        value = json.loads(path.read_text(encoding="utf-8"))
        manifests.append((path, value))
    first = manifests[0][1]
    invariants = ("product", "version", "channel", "commit_sha", "signing_key_id", "test_fixture")
    for _, value in manifests[1:]:
        for key in invariants:
            if value.get(key) != first.get(key):
                raise SystemExit(f"platform manifest drift for {key}")
    platforms = {}
    for path, value in manifests:
        key = f"{value['os']}-{value['architecture']}"
        platforms[key] = {
            "os": value["os"],
            "architecture": value["architecture"],
            "manifest": path.name,
            "signature": path.with_suffix(".sig").name,
        }
    index = {
        "schema_version": 1,
        "product": first["product"],
        "version": first["version"],
        "channel": first["channel"],
        "commit_sha": first["commit_sha"],
        "signing_key_id": first["signing_key_id"],
        "test_fixture": first["test_fixture"],
        "platforms": dict(sorted(platforms.items())),
    }
    output = Path(args.output)
    output.write_text(json.dumps(index, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
