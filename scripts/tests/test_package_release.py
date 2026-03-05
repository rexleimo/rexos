from __future__ import annotations

import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts" / "package_release.py"


class PackageReleaseTests(unittest.TestCase):
    def test_package_release_outputs_loopforge_archive_without_compat_binary(self):
        with tempfile.TemporaryDirectory() as tmp:
            workdir = Path(tmp)
            build_dir = workdir / "build"
            build_dir.mkdir(parents=True, exist_ok=True)

            primary_bin = build_dir / "loopforge"
            primary_bin.write_text("#!/bin/sh\necho loopforge\n", encoding="utf-8")
            primary_bin.chmod(0o755)

            cmd = [
                sys.executable,
                str(SCRIPT),
                "--version",
                "v0.1.0",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--bin",
                "build/loopforge",
                "--out-dir",
                "dist",
            ]
            subprocess.run(cmd, cwd=workdir, check=True)

            base = "loopforge-v0.1.0-x86_64-unknown-linux-gnu"
            archive = workdir / "dist" / f"{base}.tar.gz"
            self.assertTrue(archive.exists(), f"missing archive: {archive}")

            with tarfile.open(archive, "r:gz") as tf:
                names = set(tf.getnames())

            self.assertIn(f"{base}/loopforge", names)
            self.assertNotIn(f"{base}/rexos", names)

    def test_package_release_rejects_compat_bin_argument(self):
        with tempfile.TemporaryDirectory() as tmp:
            workdir = Path(tmp)
            build_dir = workdir / "build"
            build_dir.mkdir(parents=True, exist_ok=True)

            primary_bin = build_dir / "loopforge"
            compat_bin = build_dir / "rexos"
            primary_bin.write_text("#!/bin/sh\necho loopforge\n", encoding="utf-8")
            compat_bin.write_text("#!/bin/sh\necho rexos\n", encoding="utf-8")
            primary_bin.chmod(0o755)
            compat_bin.chmod(0o755)

            cmd = [
                sys.executable,
                str(SCRIPT),
                "--version",
                "v0.1.0",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--bin",
                "build/loopforge",
                "--compat-bin",
                "build/rexos",
                "--out-dir",
                "dist",
            ]
            result = subprocess.run(
                cmd,
                cwd=workdir,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("unrecognized arguments: --compat-bin", result.stderr)


if __name__ == "__main__":
    unittest.main()
