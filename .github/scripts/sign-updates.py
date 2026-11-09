#!/usr/bin/env python3
"""Sign the selected platform's update manifest without exposing the private key."""

import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile


def main():
    match = re.fullmatch(r"(?:(macos|windows|linux)-)?v?([0-9]+\.[0-9]+\.[0-9]+)", os.environ["RELEASE_TAG"])
    if not match:
        raise ValueError("Update manifests require a stable version")
    family = match.group(1) or "all"
    version = match.group(2)
    if os.environ.get("RELEASE_PLATFORM", family) != family:
        raise ValueError("Release platform does not match its tag")
    assets = []
    for platform in ("macos-arm64", "macos-x64", "windows-x64", "windows-x86", "linux-x64", "linux-x86"):
        if family != "all" and not platform.startswith(family + "-"):
            continue
        name = f"iwe-{version}-{platform}-update.tar.gz"
        path = Path("dist") / name
        if not 0 < path.stat().st_size <= 512 * 1024 * 1024:
            raise ValueError("Invalid update package size")
        with path.open("rb") as source:
            digest = hashlib.file_digest(source, "sha256").hexdigest()
        assets.append({"name": name, "size": path.stat().st_size, "sha256": digest})
    manifest = Path("dist/updates.json")
    manifest.write_text(json.dumps({"version": version, "assets": assets}, separators=(",", ":")) + "\n", encoding="utf-8")
    with tempfile.TemporaryDirectory() as directory:
        key = Path(directory) / "private.pem"
        key.touch(mode=0o600)
        key.write_text(os.environ["IWE_UPDATE_SIGNING_KEY"], encoding="ascii")
        public_der = subprocess.check_output(["openssl", "pkey", "-in", str(key), "-pubout", "-outform", "DER"])
        if public_der[-32:] != Path("assets/update-public-key.bin").read_bytes():
            raise ValueError("Signing key does not match the embedded verification key")
        subprocess.run(["openssl", "pkeyutl", "-sign", "-rawin", "-inkey", str(key), "-in", str(manifest), "-out", "dist/updates.sig"], check=True)
    print(f"Signed {len(assets)} update packages for {family}")


if __name__ == "__main__":
    main()
