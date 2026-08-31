#!/usr/bin/env python3
import argparse
import gzip
import os
import tarfile
from pathlib import Path


def add_tree(archive, source: Path, archive_root: str, epoch: int):
    entries = [source, *sorted(source.rglob("*"), key=lambda path: path.as_posix())]
    for path in entries:
        relative = path.relative_to(source)
        name = Path(archive_root) / relative if archive_root else relative
        if not str(name) or str(name) == ".":
            continue
        info = archive.gettarinfo(str(path), arcname=name.as_posix())
        info.uid = info.gid = 0
        info.uname = info.gname = ""
        info.mtime = epoch
        if info.isfile():
            with path.open("rb") as handle:
                archive.addfile(info, handle)
        else:
            archive.addfile(info)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--root", default="")
    parser.add_argument("--epoch", required=True, type=int)
    args = parser.parse_args()
    source = Path(args.source).resolve()
    output = Path(args.output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=args.epoch) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as archive:
                add_tree(archive, source, args.root, args.epoch)


if __name__ == "__main__":
    main()
