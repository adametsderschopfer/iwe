#!/usr/bin/env python3
"""Publish one Latest summary after all platform releases finish successfully."""

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib

FAMILIES = {
    "macos": ("arm64", "x64"),
    "windows": ("x64", "x86"),
    "linux": ("x64", "x86"),
}
ROOT = Path(__file__).resolve().parents[2]


def version_number(value):
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", value):
        raise ValueError("Use a stable numeric version")
    return tuple(map(int, value.split(".")))


def archive_names(version, family):
    return {f"iwe-{version}-{family}-{arch}-update.tar.gz" for arch in FAMILIES[family]}


def validate_summary_source(comparison, package_version, version):
    # GitHub restricts creating tags at commits with older workflow definitions.
    # A summary can target current release metadata only when application inputs match.
    if package_version != version or comparison["status"] not in {"ahead", "identical"}:
        raise ValueError("Summary metadata must retain the published application version and ancestry")
    files = comparison["files"]
    if len(files) >= 300:
        raise ValueError("The source comparison exceeds GitHub's complete file listing limit")
    for file in files:
        for path in [file["filename"], file.get("previous_filename", file["filename"])]:
            if path not in {".gitignore", "AGENTS.md", "README.md"} and not path.startswith(("docs/", "website/", ".github/scripts/", ".github/workflows/")):
                raise ValueError(f"Application inputs changed after platform publication: {path}")


def validate_platform(release, version, family):
    tag = f"{family}-v{version}"
    if release["tag_name"] != tag or release["draft"] or release["prerelease"]:
        raise ValueError(f"Platform release is not published: {tag}")
    names = {asset["name"] for asset in release["assets"] if asset["state"] == "uploaded" and asset["size"] > 0}
    required = archive_names(version, family) | {"updates.json", "updates.sig"}
    for arch in FAMILIES[family]:
        prefix = f"iwe-code-{version}-{family}-{arch}"
        required.update(
            {prefix + ".zip", prefix + "-setup.exe"} if family == "windows"
            else {prefix + (".dmg" if family == "macos" else ".tar.gz")}
        )
    if not required <= names:
        raise ValueError(f"Missing platform packages: {tag}: {sorted(required - names)}")


def validate_archives(directory, version, family):
    manifest = json.loads((directory / "updates.json").read_text())
    if manifest["version"] != version or len(manifest["assets"]) != 2:
        raise ValueError("Platform manifest has the wrong version or package count")
    if {asset["name"] for asset in manifest["assets"]} != archive_names(version, family):
        raise ValueError("Platform manifest has unexpected packages")
    for asset in manifest["assets"]:
        path = directory / asset["name"]
        with path.open("rb") as source:
            digest = hashlib.file_digest(source, "sha256").hexdigest()
        if not 0 < path.stat().st_size == asset["size"] <= 512 * 1024 * 1024 or digest != asset["sha256"]:
            raise ValueError(f"Update archive does not match its signed manifest: {path.name}")


def api(path):
    result = subprocess.run(["gh", "api", path], capture_output=True, text=True)
    if result.returncode:
        if "HTTP 404" in result.stderr:
            return None
        raise RuntimeError(f"GitHub metadata request failed: {path}")
    return json.loads(result.stdout)


def release_body(version, releases, records, previous):
    lines = [f"# IWE Code {version}", "", "## Downloads", ""]
    for family, release in releases.items():
        label = {"macos": "macOS", "windows": "Windows", "linux": "Linux"}[family]
        lines.append(f"- [{label} {version}]({release['html_url']})")
    lines.extend(["", "## Changes", ""])
    for record in sorted(records.values(), key=lambda record: version_number(record["version"]) if record["version"] != "unreleased" else (0, 0, 0)):
        if record["version"] == "unreleased":
            continue
        if previous < version_number(record["version"]) <= version_number(version):
            lines.extend([f"### {record['version']}", ""])
            lines.extend(f"- {item['text']['en']}" for item in record["items"])
            lines.append("")
    base = "https://adametsderschopfer.github.io/iwe-code/"
    lines.extend([
        " · ".join(f"[{label}]({base}{prefix}releases/{version}/)" for prefix, label in [("", "English"), ("ru/", "Русский"), ("es/", "Español")]),
        "", "Update archives below support automatic updates. Use the platform links above for installation packages.", "",
    ])
    return "\n".join(lines)


def main():
    version = os.environ.get("SUMMARY_VERSION") or os.environ["RELEASE_TAG"].split("-v", 1)[1]
    number = version_number(version)
    repo = os.environ["GH_REPO"]
    releases = {}
    commits = set()
    for family in FAMILIES:
        tag = f"{family}-v{version}"
        release = api(f"repos/{repo}/releases/tags/{tag}")
        if release is None or release["draft"]:
            print(f"Summary waits for {tag}; Latest remains unchanged.")
            return
        validate_platform(release, version, family)
        releases[family] = release
        commits.add(api(f"repos/{repo}/commits/{tag}")["sha"])
    if len(commits) != 1:
        raise ValueError("All platform releases must use the same commit")
    platform_commit = next(iter(commits))
    latest = api(f"repos/{repo}/releases/latest")
    if latest and re.fullmatch(r"v?[0-9]+\.[0-9]+\.[0-9]+", latest["tag_name"]):
        if version_number(latest["tag_name"].removeprefix("v")) > number:
            print("A newer shared release exists; Latest remains unchanged.")
            return
    tag = f"v{version}"
    existing = api(f"repos/{repo}/releases/tags/{tag}")
    if existing:
        if existing["draft"] or existing["prerelease"]:
            raise ValueError("Inspect the existing draft before retrying summary publication")
        names = {asset["name"] for asset in existing["assets"] if asset["state"] == "uploaded" and asset["size"] > 0}
        required = set.union(*(archive_names(version, family) for family in FAMILIES)) | {"updates.json", "updates.sig"}
        if not required <= names:
            raise ValueError("Existing shared release lacks automatic update archives")
        subprocess.run(["gh", "release", "edit", tag, "--latest"], check=True)
        print(f"Shared release already published: {existing['html_url']}")
        return
    previous = (0, 0, 0)
    for page in range(1, 4):
        batch = api(f"repos/{repo}/releases?per_page=100&page={page}")
        for release in batch:
            if not release["draft"] and not release["prerelease"] and re.fullmatch(r"v?[0-9]+\.[0-9]+\.[0-9]+", release["tag_name"]):
                candidate = version_number(release["tag_name"].removeprefix("v"))
                if previous < candidate < number:
                    previous = candidate
        if len(batch) < 100:
            break
    spec = importlib.util.spec_from_file_location("release_notes", ROOT / ".github/scripts/release-notes.py")
    notes = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(notes)
    records = notes.validate()
    if version not in records:
        raise ValueError("Prepare authored notes before publishing a summary")
    target = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    comparison = api(f"repos/{repo}/compare/{platform_commit}...{target}")
    package_version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    validate_summary_source(comparison, package_version, version)
    with tempfile.TemporaryDirectory() as temporary:
        staging = Path(temporary)
        dist = staging / "dist"
        dist.mkdir()
        public = staging / "public.der"
        public.write_bytes(bytes.fromhex("302a300506032b6570032100") + (ROOT / "assets/update-public-key.bin").read_bytes())
        for family in FAMILIES:
            directory = staging / family
            names = sorted(archive_names(version, family) | {"updates.json", "updates.sig"})
            command = ["gh", "release", "download", f"{family}-v{version}", "--dir", str(directory)]
            for name in names:
                command.extend(["--pattern", name])
            subprocess.run(command, check=True)
            subprocess.run(["openssl", "pkeyutl", "-verify", "-pubin", "-keyform", "DER", "-inkey", str(public), "-rawin", "-in", str(directory / "updates.json"), "-sigfile", str(directory / "updates.sig")], check=True)
            validate_archives(directory, version, family)
            for name in archive_names(version, family):
                shutil.copyfile(directory / name, dist / name)
        (staging / "assets").mkdir()
        shutil.copyfile(ROOT / "assets/update-public-key.bin", staging / "assets/update-public-key.bin")
        subprocess.run(["python3", str(ROOT / ".github/scripts/sign-updates.py")], cwd=staging, env={**os.environ, "RELEASE_TAG": tag, "RELEASE_PLATFORM": "all"}, check=True)
        body = staging / "notes.md"
        body.write_text(release_body(version, releases, records, previous), encoding="utf-8")
        assets = sorted(str(path) for path in dist.iterdir())
        subprocess.run(["gh", "release", "create", tag, "--target", target, "--title", f"IWE Code {version}", "--latest", "--notes-file", str(body), *assets], check=True)
        print(f"Published shared Latest: https://github.com/{repo}/releases/tag/{tag}")


if __name__ == "__main__":
    main()
