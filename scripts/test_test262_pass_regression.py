#!/usr/bin/env python3

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("test262_pass_regression.py")


class Test262PassRegressionTests(unittest.TestCase):
    def run_script(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), *args],
            check=False,
            capture_output=True,
            text=True,
        )

    def test_prepares_retries_for_baseline_failures_and_new_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            baseline = directory / "baseline.txt"
            initial_output = directory / "initial.txt"
            retry_list = directory / "retries.txt"
            github_output = directory / "github-output.txt"
            baseline.write_text("test/stable.js\ntest/flaky.js\n", encoding="utf-8")
            initial_output.write_text(
                "PASS test/stable.js\n"
                "FAIL test/flaky.js: transient error\n"
                "PASS test/new.js\n"
                "FAIL test/other.js: unsupported\n",
                encoding="utf-8",
            )

            result = self.run_script(
                "--runner-output",
                str(initial_output),
                "--baseline",
                str(baseline),
                "--prepare-retries",
                "--retry-list",
                str(retry_list),
                "--github-output",
                str(github_output),
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                retry_list.read_text(encoding="utf-8"),
                "test262/test/flaky.js\ntest262/test/new.js\n",
            )
            self.assertEqual(github_output.read_text(encoding="utf-8"), "retry_count=2\n")

    def test_only_confirms_new_passes_after_retry_and_recovers_baseline_failures(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            baseline = directory / "baseline.txt"
            initial_output = directory / "initial.txt"
            retry_output = directory / "retry.txt"
            github_output = directory / "github-output.txt"
            baseline.write_text("test/stable.js\ntest/flaky.js\n", encoding="utf-8")
            initial_output.write_text(
                "PASS test/stable.js\n"
                "FAIL test/flaky.js: transient error\n"
                "PASS test/new.js\n"
                "PASS test/unstable-new.js\n",
                encoding="utf-8",
            )
            retry_output.write_text(
                "PASS test/flaky.js\n"
                "PASS test/new.js\n"
                "FAIL test/unstable-new.js: transient error\n",
                encoding="utf-8",
            )

            result = self.run_script(
                "--runner-output",
                str(initial_output),
                "--baseline",
                str(baseline),
                "--retry-output",
                str(retry_output),
                "--github-output",
                str(github_output),
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                baseline.read_text(encoding="utf-8"),
                "test/flaky.js\ntest/new.js\ntest/stable.js\n",
            )
            self.assertEqual(github_output.read_text(encoding="utf-8"), "new_pass_count=1\n")
            self.assertIn("test/unstable-new.js", result.stdout)

    def test_persistent_baseline_failure_fails_without_updating_baseline(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            baseline = directory / "baseline.txt"
            initial_output = directory / "initial.txt"
            retry_output = directory / "retry.txt"
            github_output = directory / "github-output.txt"
            original_baseline = "test/stable.js\ntest/flaky.js\n"
            baseline.write_text(original_baseline, encoding="utf-8")
            initial_output.write_text(
                "PASS test/stable.js\nFAIL test/flaky.js: first failure\n",
                encoding="utf-8",
            )
            retry_output.write_text(
                "FAIL test/flaky.js: retry failure\n",
                encoding="utf-8",
            )

            result = self.run_script(
                "--runner-output",
                str(initial_output),
                "--baseline",
                str(baseline),
                "--retry-output",
                str(retry_output),
                "--github-output",
                str(github_output),
            )

            self.assertEqual(result.returncode, 1)
            self.assertEqual(baseline.read_text(encoding="utf-8"), original_baseline)
            self.assertEqual(github_output.read_text(encoding="utf-8"), "new_pass_count=0\n")
            self.assertIn("test/flaky.js", result.stderr)


if __name__ == "__main__":
    unittest.main()
