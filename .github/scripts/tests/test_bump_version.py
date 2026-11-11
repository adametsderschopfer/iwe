"""Verify version preparation without modifying the repository."""

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "bump-version.py"
SPEC = importlib.util.spec_from_file_location("bump_version", SCRIPT)
BUMP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUMP)


class BumpVersionTests(unittest.TestCase):
    def fixture(self, root, items):
        (root / "Cargo.toml").write_text('[package]\nname = "iwe"\nversion = "1.2.3"\n')
        (root / "Cargo.lock").write_text(
            '[[package]]\nname = "iwe"\nversion = "1.2.3"\n'
            '[[package]]\nname = "other"\nversion = "1.2.3"\n'
        )
        ledger = root / "website/app/release-notes/releases.json"
        ledger.parent.mkdir(parents=True)
        ledger.write_text(json.dumps([{
            "version": "unreleased", "status": "prepared", "source": None,
            "tags": [], "publishedAt": None, "items": items,
        }]))
        return ledger

    def test_version_choices_move_notes_and_preserve_dependency(self):
        for kind, expected in [("patch", "1.2.4"), ("minor", "1.3.0"), ("major", "2.0.0")]:
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                ledger = self.fixture(root, [{"id": "reviewed-change"}])
                self.assertEqual(BUMP.prepare(root, kind), expected)
                records = json.loads(ledger.read_text())
                self.assertEqual(records[0]["items"], [{"id": "reviewed-change"}])
                self.assertEqual(records[1]["items"], [])
                self.assertIn(f'version = "{expected}"', (root / "Cargo.toml").read_text())
                self.assertIn('name = "other"\nversion = "1.2.3"', (root / "Cargo.lock").read_text())

    def test_empty_notes_leave_files_unchanged(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root, [])
            original = (root / "Cargo.toml").read_text()
            with self.assertRaisesRegex(ValueError, "reviewed unreleased"):
                BUMP.prepare(root, "patch")
            self.assertEqual((root / "Cargo.toml").read_text(), original)

    def test_missing_lock_package_leaves_manifest_unchanged(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root, [{"id": "reviewed-change"}])
            (root / "Cargo.lock").write_text("")
            original = (root / "Cargo.toml").read_text()
            with self.assertRaisesRegex(ValueError, "Cargo.lock"):
                BUMP.prepare(root, "patch")
            self.assertEqual((root / "Cargo.toml").read_text(), original)
