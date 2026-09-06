# Continuous Test262 baseline check

`.github/workflows/test262.yml` runs on pull requests (targeting `master`) and
can also be started with **Run workflow**. The workflow checks out a shallow
Test262 copy, builds the release runner, and executes the supported built-ins,
language, and Annex B suites with the existing explicit exclusions.

The runner is built with the nightly Rust toolchain because the interpreter's
dependency uses an unstable language feature.

The workflow compares the run output against `passed_test262.txt` via
`scripts/test262_pass_regression.py`:

- If any test that is already listed in `passed_test262.txt` no longer reports
  `PASS`, the check fails and logs the missing tests.
- If all listed tests still pass and there are no additional passing tests, the
  check passes with no repository changes.
- If all listed tests still pass and new passing tests are found, the check
  passes and commits an updated `passed_test262.txt` baseline.

Runner output is uploaded as an artifact so failures can be inspected from CI.
