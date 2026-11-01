#!/usr/bin/env python3
"""Fail on passed-Test262 regressions and refresh the baseline on new passes."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

PASS_RE = re.compile(r"^PASS (?P<path>.+)$")
FAIL_RE = re.compile(r"^FAIL (?P<path>.+?): .+$")


def read_baseline(path: Path) -> set[str]:
    return {
        line.strip()
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    }


def read_passes(path: Path) -> set[str]:
    passes: set[str] = set()
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = PASS_RE.match(line.strip())
        if match:
            passes.add(match.group("path"))
    return passes


def read_failures(path: Path) -> set[str]:
    failures: set[str] = set()
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = FAIL_RE.match(line.strip())
        if match:
            failures.add(match.group("path"))
    return failures


def retry_candidates(
    baseline: set[str], initial_passes: set[str], initial_failures: set[str]
) -> set[str]:
    baseline_failures = (baseline - initial_passes) & initial_failures
    new_passes = initial_passes - baseline
    return baseline_failures | new_passes


def write_output(path: str | None, key: str, value: str) -> None:
    if not path:
        return
    with Path(path).open("a", encoding="utf-8") as handle:
        handle.write(f"{key}={value}\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runner-output", type=Path, required=True)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--github-output")
    parser.add_argument("--retry-output", type=Path)
    parser.add_argument("--prepare-retries", action="store_true")
    parser.add_argument("--retry-list", type=Path)
    args = parser.parse_args()

    baseline = read_baseline(args.baseline)
    initial_passes = read_passes(args.runner_output)
    initial_failures = read_failures(args.runner_output)

    if args.prepare_retries:
        if args.retry_list is None:
            parser.error("--prepare-retries requires --retry-list")
        retries = sorted(retry_candidates(baseline, initial_passes, initial_failures))
        args.retry_list.parent.mkdir(parents=True, exist_ok=True)
        args.retry_list.write_text(
            "".join(f"test262/{test}\n" for test in retries), encoding="utf-8"
        )
        write_output(args.github_output, "retry_count", str(len(retries)))
        print(f"Prepared {len(retries)} Test262 retries.")
        return 0
    if args.retry_list is not None:
        parser.error("--retry-list requires --prepare-retries")

    retry_passes = read_passes(args.retry_output) if args.retry_output else set()
    retry_passes &= retry_candidates(baseline, initial_passes, initial_failures)
    current_passes = initial_passes | retry_passes

    missing = sorted(baseline - current_passes)
    if missing:
        print(
            f"Regression detected: {len(missing)} previously passing Test262 tests still fail after retry.",
            file=sys.stderr,
        )
        for test in missing:
            print(f"- {test}", file=sys.stderr)
        write_output(args.github_output, "new_pass_count", "0")
        return 1

    new_passes = sorted((initial_passes - baseline) & retry_passes)
    unconfirmed_passes = sorted((initial_passes - baseline) - retry_passes)
    write_output(args.github_output, "new_pass_count", str(len(new_passes)))

    if unconfirmed_passes:
        print(
            f"Not adding {len(unconfirmed_passes)} new passing tests because they did not pass on retry."
        )
        for test in unconfirmed_passes:
            print(f"! {test}")

    if not new_passes:
        print("No Test262 pass regressions detected. No confirmed new passing tests.")
        return 0

    updated = sorted(baseline | set(new_passes))
    args.baseline.write_text("\n".join(updated) + "\n", encoding="utf-8")
    print(f"No regressions detected. Added {len(new_passes)} confirmed passing tests to baseline.")
    for test in new_passes:
        print(f"+ {test}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
