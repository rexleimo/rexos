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
        self.assertIn("scripts.tests.test_resolve_release_tag", ci)
        self.assertIn("scripts.tests.test_provider_health_report", ci)
        self.assertIn("scripts.tests.test_package_release", ci)
        self.assertIn("scripts.tests.test_onboard_metrics_report", ci)

    def test_ci_includes_windows_security_boundaries_fast_slice(self):
        ci = (REPO_ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        self.assertIn("security-boundaries-fast", ci)
        self.assertIn("security boundaries (fast) (windows)", ci)
        self.assertIn("runs-on: windows-latest", ci)
        self.assertIn("cargo test -p rexos-tools --locked tests::web::a2a::", ci)
        self.assertIn(
            "cargo test -p rexos-tools --locked tests::web::fetch::web_fetch_respects_egress_policy_rules",
            ci,
        )
        self.assertIn("cargo test -p rexos-tools --locked tests::browser::policy::url::", ci)

    def test_ci_includes_msrv_compile_guard(self):
        ci = (REPO_ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        self.assertIn("msrv-compile", ci)
        self.assertIn("msrv compile (rust 1.75.0)", ci)
        self.assertIn("Install Rust 1.75.0", ci)
        self.assertIn("toolchain: 1.75.0", ci)
        self.assertIn("cargo check --workspace --locked", ci)

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
        self.assertIn("cargo build --release -p loopforge-cli --locked", workflow)
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

    def test_auto_release_tag_workflow_creates_missing_semver_tag_after_ci(self):
        workflow = (
            REPO_ROOT / ".github/workflows/auto-release-tag.yml"
        ).read_text(encoding="utf-8")
        self.assertIn("workflow_run", workflow)
        self.assertIn('workflows: ["CI"]', workflow)
        self.assertIn("github.event.workflow_run.conclusion == 'success'", workflow)
        self.assertIn("github.event.workflow_run.head_branch == 'main'", workflow)
        self.assertIn("scripts/resolve_release_tag.py", workflow)
        self.assertIn("release check --tag", workflow)
        self.assertIn("git push origin", workflow)

    def test_release_dry_run_workflow_has_packaged_binary_smoke_steps(self):
        workflow = (
            REPO_ROOT / ".github/workflows/release-dry-run.yml"
        ).read_text(encoding="utf-8")
        self.assertIn("Smoke test packaged binary (Unix)", workflow)
        self.assertIn("Smoke test packaged binary (Windows)", workflow)
        self.assertIn("doctor --json", workflow)


if __name__ == "__main__":
    unittest.main()
