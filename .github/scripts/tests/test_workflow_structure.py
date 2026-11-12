"""Guard workflow entry points and pinned source checkout."""

from pathlib import Path
import re
import unittest

WORKFLOWS = Path(__file__).resolve().parents[2] / "workflows"


class WorkflowStructureTests(unittest.TestCase):
    def test_expected_english_workflows_exist(self):
        self.assertEqual(
            {path.name for path in WORKFLOWS.glob("*.yml")},
            {"ci.yml", "bump-version.yml", "website-build.yml"},
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
        self.assertIn("needs: prepare", jobs["release"])
        self.assertIn("needs: [prepare, publish]", jobs["website_build"])
        self.assertIn("needs: website_build", jobs["website_deploy"])
        self.assertNotIn("gh workflow run", source)
        self.assertNotIn("github.ref_type == 'tag'", source)

    def test_checks_only_run_on_main_pushes(self):
        source = (WORKFLOWS / "ci.yml").read_text()
        triggers = source.split("on:\n", 1)[1].split("permissions:", 1)[0]
        self.assertIn("branches: [main]", triggers)
        for trigger in ("pull_request:", "workflow_call:", "workflow_dispatch:"):
            self.assertNotIn(trigger, triggers)
        release = (WORKFLOWS / "bump-version.yml").read_text()
        self.assertNotIn("./.github/workflows/ci.yml", release)
        self.assertNotIn("\n  checks:\n", release)


    def test_manual_website_build_uses_main_and_publishes(self):
        source = (WORKFLOWS / "website-build.yml").read_text()
        triggers = source.split("on:\n", 1)[1].split("permissions:", 1)[0]
        self.assertEqual(triggers.strip(), "workflow_dispatch:")
        self.assertIn("if: github.ref == 'refs/heads/main'", source)
        self.assertIn("ref: ${{ github.sha }}", source)
        self.assertIn("npm run generate", source)
        self.assertIn("actions/upload-artifact@", source)
        self.assertIn("actions/deploy-pages@", source)
        self.assertIn("needs: build", source)
        self.assertNotIn("contents: write", source)
