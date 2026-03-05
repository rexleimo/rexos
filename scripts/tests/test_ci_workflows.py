from pathlib import Path
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]


class CiWorkflowTests(unittest.TestCase):
    def test_ci_runs_versioning_script_tests(self):
        ci = (REPO_ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        self.assertIn("scripts-tests", ci)
        self.assertIn("python3 -m unittest", ci)
        self.assertIn("scripts.tests.test_verify_version_changelog", ci)
        self.assertIn("scripts.tests.test_verify_release_consistency", ci)
        self.assertIn("scripts.tests.test_provider_health_report", ci)
        self.assertIn("scripts.tests.test_package_release", ci)

    def test_provider_nightly_workflow_generates_health_artifacts(self):
        workflow = (
            REPO_ROOT / ".github/workflows/provider-nightly.yml"
        ).read_text(encoding="utf-8")
        self.assertIn("schedule:", workflow)
        self.assertIn("provider_health_report.py", workflow)
        self.assertIn("upload-artifact@v4", workflow)

    def test_release_dry_run_workflow_builds_but_does_not_publish(self):
        workflow = (
            REPO_ROOT / ".github/workflows/release-dry-run.yml"
        ).read_text(encoding="utf-8")
        self.assertIn("pull_request", workflow)
        self.assertIn("workflow_dispatch", workflow)
        self.assertIn("cargo build --release -p rexos-cli --locked", workflow)
        self.assertIn("python scripts/package_release.py", workflow)
        self.assertIn("target/release/loopforge", workflow)
        self.assertNotIn("target/release/rexos", workflow)
        self.assertNotIn("--compat-bin", workflow)
        self.assertIn("loopforge-${version}", workflow)
        self.assertIn("actions/upload-artifact@v4", workflow)
        self.assertNotIn("softprops/action-gh-release", workflow)

    def test_release_workflow_has_packaged_binary_smoke_steps(self):
        workflow = (REPO_ROOT / ".github/workflows/release.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("Smoke test packaged binary (Unix)", workflow)
        self.assertIn("Smoke test packaged binary (Windows)", workflow)
        self.assertIn("doctor --json", workflow)
        self.assertIn("target/release/loopforge", workflow)
        self.assertNotIn("target/release/rexos", workflow)
        self.assertNotIn("--compat-bin", workflow)
        self.assertIn("loopforge-${version}", workflow)
        self.assertIn("runner.os != 'Windows'", workflow)
        self.assertIn("runner.os == 'Windows'", workflow)

    def test_release_dry_run_workflow_has_packaged_binary_smoke_steps(self):
        workflow = (
            REPO_ROOT / ".github/workflows/release-dry-run.yml"
        ).read_text(encoding="utf-8")
        self.assertIn("Smoke test packaged binary (Unix)", workflow)
        self.assertIn("Smoke test packaged binary (Windows)", workflow)
        self.assertIn("doctor --json", workflow)


if __name__ == "__main__":
    unittest.main()
