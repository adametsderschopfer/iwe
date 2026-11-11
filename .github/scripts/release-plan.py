#!/usr/bin/env python3
"""Plan cross-platform CI and validate release tags."""

import json
import os
from pathlib import Path
import re
import subprocess
import sys


if os.environ["REF_TYPE"] == "tag":
    match = re.fullmatch(r"v([0-9]+\.[0-9]+\.[0-9]+)", os.environ["REF_NAME"])
    if not match:
        raise ValueError("Use a stable release tag in the form vMAJOR.MINOR.PATCH")

notes_command = [sys.executable, str(Path(__file__).with_name("release-notes.py")), "--check"]
if os.environ["REF_TYPE"] == "tag":
    notes_command.extend(["--version", match.group(1)])
subprocess.run(notes_command, check=True)

targets = [
    {"os": "macos-15", "target": "aarch64-apple-darwin", "platform": "macos-arm64"},
    {"os": "macos-15-intel", "target": "x86_64-apple-darwin", "platform": "macos-x64"},
    {"os": "windows-2022", "target": "x86_64-pc-windows-msvc", "platform": "windows-x64"},
    {"os": "windows-2022", "target": "i686-pc-windows-msvc", "platform": "windows-x86"},
    {"os": "ubuntu-22.04", "target": "x86_64-unknown-linux-gnu", "platform": "linux-x64"},
    {"os": "ubuntu-22.04", "target": "i686-unknown-linux-gnu", "platform": "linux-x86"},
]
checks = ["macos-latest", "windows-latest", "ubuntu-24.04"]
with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
    output.write("checks=" + json.dumps({"os": checks}, separators=(",", ":")) + "\n")
    output.write("targets=" + json.dumps({"include": targets}, separators=(",", ":")) + "\n")
