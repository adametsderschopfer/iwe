#!/usr/bin/env python3
"""Validate authored release notes and render the matching GitHub release body."""

import argparse
from datetime import datetime
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
LEDGER = ROOT / "website/app/release-notes/releases.json"
LANGUAGES = {"en", "ru", "es"}
CATEGORIES = {"added": "New", "changed": "Improved", "fixed": "Fixed", "maintenance": "Maintenance"}
VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+")


def validate():
    records = json.loads(LEDGER.read_text(encoding="utf-8"))
    if not isinstance(records, list) or not records:
        raise ValueError("Release notes must contain version records")
    versions = {}
    identifiers = set()
    for record in records:
        version = record["version"]
        if version != "unreleased" and not VERSION.fullmatch(version):
            raise ValueError(f"Invalid release version: {version}")
        if version in versions:
            raise ValueError(f"Duplicate release version: {version}")
        versions[version] = record
        status = record["status"]
        if status not in {"released", "development", "prepared"}:
            raise ValueError(f"Unknown release status: {version}")
        items = record["items"]
        if not isinstance(items, list) or (version != "unreleased" and not items):
            raise ValueError(f"Version needs authored release-note items: {version}")
        if version == "unreleased" and (status != "prepared" or record["tags"] or record["publishedAt"]):
            raise ValueError("Unreleased notes must remain unpublished")
        if status == "released" and (not record["tags"] or not record["publishedAt"]):
            raise ValueError(f"Published releases need verified tags and a publication date: {version}")
        if status != "released" and (record["tags"] or record["publishedAt"]):
            raise ValueError(f"Unpublished versions cannot claim publication: {version}")
        if record["publishedAt"]:
            date = datetime.fromisoformat(record["publishedAt"].replace("Z", "+00:00"))
            if date.tzinfo is None:
                raise ValueError(f"Publication dates need a timezone: {version}")
        for tag in record["tags"]:
            if not re.fullmatch(r"(?:(?:macos|windows|linux)-)?v?" + re.escape(version), tag):
                raise ValueError(f"Release tag does not match version {version}: {tag}")
        if record["source"] is not None and not re.fullmatch(r"[0-9a-f]{40}", record["source"]):
            raise ValueError(f"Version source must be an inspected commit: {version}")
        for item in items:
            identifier = item["id"]
            if not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", identifier) or identifier in identifiers:
                raise ValueError(f"Invalid or duplicate release-note item: {identifier}")
            identifiers.add(identifier)
            if item["category"] not in CATEGORIES:
                raise ValueError(f"Unknown release-note category: {identifier}")
            if set(item["text"]) != LANGUAGES or any(
                not isinstance(value, str) or not value.strip()
                for value in item["text"].values()
            ):
                raise ValueError(f"Every item needs English, Russian, and Spanish text: {identifier}")
            if not isinstance(item["sources"], list) or not item["sources"]:
                raise ValueError(f"Every item needs inspected source references: {identifier}")
            for source in item["sources"]:
                if (
                    not isinstance(source, str)
                    or not re.fullmatch(r"[A-Za-z0-9_./-]+", source)
                    or source.startswith("/")
                    or ".." in source.split("/")
                ):
                    raise ValueError(f"Use commit identifiers or repository-relative source paths: {identifier}")
    if "unreleased" not in versions:
        raise ValueError("Keep an unreleased record for notes collected during commits")
    return versions


def render(record):
    lines = [f"# IWE Code {record['version']}", ""]
    for category, title in CATEGORIES.items():
        items = [item for item in record["items"] if item["category"] == category]
        if items:
            lines.extend([f"## {title}", ""])
            lines.extend(f"- {item['text']['en']}" for item in items)
            lines.append("")
    base = "https://adametsderschopfer.github.io/iwe-code/"
    links = [
        f"[{label}]({base}{prefix}releases/{record['version']}/)"
        for prefix, label in [("", "English"), ("ru/", "Русский"), ("es/", "Español")]
    ]
    lines.extend([" · ".join(links), ""])
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--version")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        versions = validate()
        record = None
        if args.version:
            record = versions.get(args.version)
            if (
                not record
                or args.version == "unreleased"
                or record["status"] not in {"prepared", "released"}
                or not record["items"]
            ):
                raise ValueError(f"Prepare reviewed release notes for version {args.version} before creating its tag")
        if args.output:
            if record is None:
                raise ValueError("--output requires --version")
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(render(record), encoding="utf-8")
        elif not args.check:
            raise ValueError("Choose --check or --version VERSION --output FILE")
        print(f"Release notes validated: {len(versions) - 1} versions")
    except (OSError, ValueError, KeyError, TypeError) as error:
        parser.exit(1, f"Release notes: {error}\n")


if __name__ == "__main__":
    main()
