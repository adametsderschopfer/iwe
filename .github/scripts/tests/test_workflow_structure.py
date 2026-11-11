"""Guard the two-workflow release entry points and pinned source checkout."""

from pathlib import Path
import re
import unittest

WORKFLOWS = Path(__file__).resolve().parents[2] / "workflows"


class WorkflowStructureTests(unittest.TestCase):
    def test_only_two_english_workflows_exist(self):
        self.assertEqual(
            {path.name for path in WORKFLOWS.glob("*.yml")},
            {"ci.yml", "bump-version.yml"},
        )
        for path in WORKFLOWS.glob("*.yml"):
            self.assertNotRegex(path.read_text(), r"[А-Яа-яЁё]")

    def test_release_jobs_use_prepared_commit(self):
        source = (WORKFLOWS / "bump-version.yml").read_text()
        jobs = dict(re.findall(
            r"^  ([a-z_]+):\n(.*?)(?=^  [a-z_]+:\n|\Z)",
            source.split("jobs:\n", 1)[1], re.MULTILINE | re.DOTALL,
        ))
        for name in ("release", "plan", "build", "windows_package", "publish", "website_build"):
            with self.subTest(job=name):
                self.assertIn("ref: ${{ needs.prepare.outputs.sha }}", jobs[name])
        self.assertIn("needs: [prepare, checks]", jobs["release"])
        self.assertIn("needs: [prepare, publish]", jobs["website_build"])
        self.assertIn("needs: website_build", jobs["website_deploy"])
        self.assertNotIn("gh workflow run", source)
        self.assertNotIn("github.ref_type == 'tag'", source)
