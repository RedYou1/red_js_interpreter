#!/usr/bin/env python3
"""Fail on passed-Test262 regressions and refresh the baseline on new passes."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

PASS_RE = re.compile(r"^PASS (?P<path>.+)$")


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
    args = parser.parse_args()

    baseline = read_baseline(args.baseline)
    current_passes = read_passes(args.runner_output)

    missing = sorted(baseline - current_passes)
    if missing:
        print(
            f"Regression detected: {len(missing)} previously passing Test262 tests no longer pass.",
            file=sys.stderr,
        )
        for test in missing:
            print(f"- {test}", file=sys.stderr)
        write_output(args.github_output, "new_pass_count", "0")
        return 1

    new_passes = sorted(current_passes - baseline)
    write_output(args.github_output, "new_pass_count", str(len(new_passes)))

    if not new_passes:
        print("No Test262 pass regressions detected. No new passing tests.")
        return 0

    updated = sorted(current_passes)
    args.baseline.write_text("\n".join(updated) + "\n", encoding="utf-8")
    print(f"No regressions detected. Added {len(new_passes)} new passing tests to baseline.")
    for test in new_passes:
        print(f"+ {test}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
