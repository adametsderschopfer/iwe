#!/usr/bin/env python3
"""Prepare version metadata from reviewed unreleased notes."""

import argparse
import json
from pathlib import Path
import re
import tomllib


def prepare(root, kind):
    manifest = root / "Cargo.toml"
    lock = root / "Cargo.lock"
    ledger = root / "website/app/release-notes/releases.json"
    manifest_text = manifest.read_text()
    package = tomllib.loads(manifest_text)["package"]
    parts = [int(part) for part in package["version"].split(".")]
    index = {"major": 0, "minor": 1, "patch": 2}[kind]
    parts[index] += 1
    parts[index + 1:] = [0] * (2 - index)
    version = ".".join(map(str, parts))
    records = json.loads(ledger.read_text())
    unreleased = next(record for record in records if record["version"] == "unreleased")
    if not unreleased["items"]:
        raise ValueError("Add reviewed unreleased notes before requesting a version")
    if any(record["version"] == version for record in records):
        raise ValueError(f"Version {version} already exists")
    old = re.escape(package["version"])
    manifest_text, count = re.subn(
        rf'(?m)^(version = "){old}(")$',
        lambda match: match[1] + version + match[2], manifest_text, count=1,
    )
    if count != 1:
        raise ValueError("Cannot locate package version")
    lock_text, count = re.subn(
        rf'(\[\[package\]\]\nname = "{re.escape(package["name"])}"\nversion = "){old}(")',
        lambda match: match[1] + version + match[2], lock.read_text(),
    )
    if count != 1:
        raise ValueError("Cannot locate unique package version in Cargo.lock")
    records.insert(records.index(unreleased), {
        **unreleased, "version": version, "source": None,
    })
    unreleased["items"] = []
    manifest.write_text(manifest_text)
    lock.write_text(lock_text)
    ledger.write_text(json.dumps(records, ensure_ascii=False, indent=2) + "\n")
    return version


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("kind", choices=["patch", "minor", "major"])
    args = parser.parse_args()
    print(prepare(Path(__file__).resolve().parents[2], args.kind))
