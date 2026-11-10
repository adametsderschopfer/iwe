"""Exercise summary prerequisites and package integrity without publishing releases."""

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("publish_summary", Path(__file__).resolve().parents[1] / "publish-summary.py")
summary = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(summary)


class SummaryTests(unittest.TestCase):
    def test_unpublished_platform_never_changes_latest(self):
        with patch.dict(os.environ, {"SUMMARY_VERSION": "0.1.14", "GH_REPO": "owner/repo"}):
            with patch.object(summary, "api", return_value=None), patch.object(summary.subprocess, "run") as run:
                summary.main()
                run.assert_not_called()

    def test_all_installers_must_exist_before_summary_publication(self):
        names = summary.archive_names("0.1.14", "windows") | {"updates.json", "updates.sig"}
        for arch in ["x64", "x86"]:
            names.update({f"iwe-code-0.1.14-windows-{arch}.zip", f"iwe-code-0.1.14-windows-{arch}-setup.exe"})
        release = {"tag_name": "windows-v0.1.14", "draft": False, "prerelease": False, "assets": [
            {"name": name, "size": 42, "state": "uploaded"} for name in names
        ]}
        summary.validate_platform(release, "0.1.14", "windows")
        for missing in names:
            with self.subTest(missing=missing), self.assertRaises(ValueError):
                summary.validate_platform({**release, "assets": [asset for asset in release["assets"] if asset["name"] != missing]}, "0.1.14", "windows")
        for field in ["draft", "prerelease"]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                summary.validate_platform({**release, field: True}, "0.1.14", "windows")

    def test_archive_bytes_and_platform_must_match_manifest(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            data = b"update package fixture"
            assets = []
            for name in sorted(summary.archive_names("0.1.14", "linux")):
                (directory / name).write_bytes(data)
                assets.append({"name": name, "size": len(data), "sha256": hashlib.sha256(data).hexdigest()})
            manifest = directory / "updates.json"
            manifest.write_text(json.dumps({"version": "0.1.14", "assets": assets}))
            summary.validate_archives(directory, "0.1.14", "linux")
            with self.assertRaises(ValueError):
                summary.validate_archives(directory, "0.1.14", "windows")
            with self.assertRaises(ValueError):
                summary.validate_archives(directory, "0.1.15", "linux")
            (directory / assets[0]["name"]).write_bytes(b"damaged")
            with self.assertRaises(ValueError):
                summary.validate_archives(directory, "0.1.14", "linux")

    def test_body_collects_changes_since_previous_shared_version(self):
        records = {version: {"version": version, "items": [{"text": {"en": message}}]} for version, message in [
            ("0.1.11", "Old change"), ("0.1.13", "Workspace improvements"), ("0.1.14", "Platform fix"), ("unreleased", "Future change")
        ]}
        releases = {family: {"html_url": f"https://example.com/{family}"} for family in summary.FAMILIES}
        body = summary.release_body("0.1.14", releases, records, (0, 1, 11))
        for value in ["Workspace improvements", "Platform fix", "https://example.com/windows", "https://example.com/linux", "https://example.com/macos"]:
            self.assertIn(value, body)
        for value in ["Old change", "Future change"]:
            self.assertNotIn(value, body)

    def test_version_rejects_untrusted_tag_components(self):
        for value in ["../0.1.14", "0.1.14-rc1", "1.2", "$(anything)", "v0.1.14"]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                summary.version_number(value)

    def test_summary_source_permits_metadata_but_rejects_changed_application_inputs(self):
        comparison = {"status": "ahead", "files": [{"filename": ".github/workflows/release-summary.yml"}, {"filename": "docs/ci.md"}]}
        summary.validate_summary_source(comparison, "0.1.14", "0.1.14")
        with self.assertRaises(ValueError):
            summary.validate_summary_source(comparison, "0.1.15", "0.1.14")
        for path in ["src/i18n/mod.rs", "Cargo.lock", "assets/update-public-key.bin", "packaging/windows/setup.iss"]:
            with self.subTest(path=path), self.assertRaises(ValueError):
                summary.validate_summary_source({"status": "ahead", "files": [{"filename": path}]}, "0.1.14", "0.1.14")
        with self.assertRaises(ValueError):
            summary.validate_summary_source({"status": "diverged", "files": []}, "0.1.14", "0.1.14")
        with self.assertRaises(ValueError):
            summary.validate_summary_source({"status": "ahead", "files": [{"filename": "docs/ci.md", "previous_filename": "src/main.rs"}]}, "0.1.14", "0.1.14")


if __name__ == "__main__":
    unittest.main()
