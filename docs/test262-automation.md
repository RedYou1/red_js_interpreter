# Continuous Test262 triage

`.github/workflows/test262.yml` runs the supported Test262 built-ins, language,
and Annex B suites every
Monday and can also be started with **Run workflow**. The workflow checks out a
shallow Test262 copy, builds the release runner, and passes only the explicitly
unsupported `BigInt` and `Symbol` features as skips. Runner output and the
machine-readable triage summary are retained as artifacts.

The runner is built with the nightly Rust toolchain because the interpreter's
dependency uses an unstable language feature.

The coordinator is `scripts/test262_triage.py`. It parses only `FAIL` records,
groups them by directory, feature metadata, and failure category, and compares
the current sweep with cached state. Actionable groups are created or updated
as issues labeled `test262-failure`; at most eight open task issues are created
at once. Skips, unsupported results, infrastructure failures, and duplicate
failures are not delegated. A group absent from a later sweep is closed with a
comment after the rerun.

Each issue contains the coordinator and fixer-agent checklists. A coordinator
must verify supported scope before delegating work, split unrelated cases into
focused tasks, and require human approval before changing the supported scope
or merging fixes. A fixer agent must reproduce the case, identify the parser,
execution, built-in, or runner subsystem, add a regression test, run the
relevant Rust and Test262 tests, and open a focused linked pull request. After
merge, the coordinator should rerun the affected subset before closing the
issue.

The cache is keyed by workflow run ID with a restore prefix, so the previous
state is available without committing generated files to the repository.
Artifacts and issues are the durable record for each run; supported-test
failures are surfaced as warnings, while runner infrastructure errors fail the
workflow.
