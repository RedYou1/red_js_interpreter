#!/usr/bin/env python3
"""Summarize Test262 failures and coordinate focused GitHub issues."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from pathlib import Path, PurePosixPath
from urllib import error, parse, request


FAILURE_RE = re.compile(r"^FAIL (?P<path>.+?): (?P<reason>.+)$")
FEATURES_RE = re.compile(r"^\s*features:\s*(?P<features>.*)$")
INFRASTRUCTURE_MARKERS = (
    "could not load test262 harness",
    "failed to spawn",
    "runner error",
    "runner infrastructure",
)
LABEL = "test262-failure"
DEFAULT_MAX_OPEN_TASKS = 8


def parse_features(test262_dir: Path, test_path: str) -> tuple[str, ...]:
    """Read the small frontmatter subset needed for grouping failures."""
    source = test262_dir / test_path
    try:
        text = source.read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        return ()

    frontmatter = re.search(r"/\*---(?P<body>.*?)---\*/", text, re.DOTALL)
    if not frontmatter:
        return ()
    for line in frontmatter.group("body").splitlines():
        match = FEATURES_RE.match(line)
        if not match:
            continue
        value = match.group("features").strip().strip("[]")
        return tuple(
            sorted(
                item.strip().strip("'\"")
                for item in value.split(",")
                if item.strip()
            )
        )
    return ()


def classify_reason(reason: str) -> str:
    lowered = reason.lower()
    if "syntaxerror" in lowered or "parse" in lowered:
        return "parser"
    if any(marker in lowered for marker in INFRASTRUCTURE_MARKERS):
        return "infrastructure"
    if "not implemented" in lowered or "unsupported" in lowered:
        return "unsupported"
    if any(name in lowered for name in ("typeerror", "rangeerror", "referenceerror")):
        return "runtime-error"
    return "runtime"


def parse_failures(output: str, test262_dir: Path) -> list[dict[str, object]]:
    failures = []
    for line in output.splitlines():
        match = FAILURE_RE.match(line.strip())
        if not match:
            continue
        path = PurePosixPath(match.group("path")).as_posix()
        reason = match.group("reason").strip()
        features = parse_features(test262_dir, path)
        error_type = classify_reason(reason)
        key = hashlib.sha256(f"{path}\n{reason}".encode()).hexdigest()[:16]
        failures.append(
            {
                "key": key,
                "path": path,
                "reason": reason,
                "features": list(features),
                "directory": str(PurePosixPath(path).parent),
                "error_type": error_type,
            }
        )
    return failures


def group_failures(failures: list[dict[str, object]]) -> dict[str, dict[str, object]]:
    groups: dict[str, dict[str, object]] = {}
    for failure in failures:
        features = ",".join(failure["features"]) or "none"
        group_name = "|".join(
            (str(failure["directory"]), features, str(failure["error_type"]))
        )
        group_id = hashlib.sha256(group_name.encode()).hexdigest()[:12]
        group = groups.setdefault(
            group_id,
            {
                "name": group_name,
                "directory": failure["directory"],
                "features": failure["features"],
                "error_type": failure["error_type"],
                "failures": [],
            },
        )
        group["failures"].append(failure)
    return groups


class GitHub:
    def __init__(self, token: str, repository: str) -> None:
        self.token = token
        self.base_url = f"https://api.github.com/repos/{repository}"

    def request(self, method: str, endpoint: str, payload: object | None = None) -> object:
        body = None if payload is None else json.dumps(payload).encode()
        http_request = request.Request(
            self.base_url + endpoint,
            data=body,
            method=method,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": "Bearer " + self.token,
                "X-GitHub-Api-Version": "2022-11-28",
                "Content-Type": "application/json",
            },
        )
        with request.urlopen(http_request, timeout=30) as response:
            return json.loads(response.read())

    def ensure_label(self) -> None:
        try:
            self.request("POST", "/labels", {"name": LABEL, "color": "5319e7"})
        except error.HTTPError as exc:
            if exc.code != 422:
                raise

    def open_issue_count(self) -> int:
        query = parse.urlencode({"state": "open", "labels": LABEL, "per_page": 100})
        issues = self.request("GET", f"/issues?{query}")
        return len(issues)


def issue_body(group: dict[str, object], run_url: str, new_count: int) -> str:
    failures = group["failures"]
    rows = "\n".join(
        f"- `{failure['path']}` — {failure['reason']}" for failure in failures
    )
    return f"""<!-- test262-failure-group: {group['name']} -->
## Test262 conformance failures

The scheduled Test262 coordinator found **{len(failures)}** actionable failure(s)
in `{group['directory']}`. This report contains {new_count} failure(s) not seen
in the preceding run.

- Feature metadata: `{', '.join(group['features']) or 'none'}`
- Failure category: `{group['error_type']}`
- Run: {run_url or 'local run'}

### Coordinator checklist

- [ ] Confirm each case is within the interpreter's intentionally supported
      scope; do not delegate skipped, unsupported, or infrastructure failures.
- [ ] Split this issue into focused fixer-agent tasks when cases need different
      parser, execution, built-in, or runner changes.
- [ ] Require human approval before broadening the supported-test scope or
      merging a fix.

### Failing cases

{rows}

### Fixer-agent requirements

Reproduce the case locally, identify the affected subsystem, add or update a
regression test, run the relevant Rust tests and Test262 case, and open a
focused pull request linked to this issue. The coordinator should monitor CI
and review results and rerun this affected subset after the pull request lands.
"""


def sync_issues(
    groups: dict[str, dict[str, object]],
    state: dict[str, object],
    github: GitHub | None,
    run_url: str,
    max_open_tasks: int,
) -> dict[str, object]:
    previous_failures = set(state.get("failures", {}))
    previous_groups = state.get("groups", {})
    current_failures = {
        failure["key"]: failure for group in groups.values() for failure in group["failures"]
    }
    updated_groups: dict[str, object] = {}
    open_count = github.open_issue_count() if github else 0
    if github:
        github.ensure_label()

    for group_id, group in groups.items():
        if group["error_type"] in {"unsupported", "infrastructure"}:
            continue
        old_group = previous_groups.get(group_id, {})
        new_count = sum(
            failure["key"] not in previous_failures for failure in group["failures"]
        )
        issue_number = old_group.get("issue") if isinstance(old_group, dict) else None
        if github and (new_count or not issue_number):
            if not issue_number and open_count >= max_open_tasks:
                continue
            body = issue_body(group, run_url, new_count)
            if issue_number:
                github.request(
                    "PATCH",
                    f"/issues/{issue_number}",
                    {"title": f"Test262 failures: {group['directory']}", "body": body, "state": "open"},
                )
            else:
                issue = github.request(
                    "POST",
                    "/issues",
                    {
                        "title": f"Test262 failures: {group['directory']}",
                        "body": body,
                        "labels": [LABEL],
                    },
                )
                issue_number = issue["number"]
                open_count += 1
        updated_groups[group_id] = {
            "issue": issue_number,
            "failures": [failure["key"] for failure in group["failures"]],
        }

    if github:
        for group_id, old_group in previous_groups.items():
            if group_id in updated_groups or not isinstance(old_group, dict):
                continue
            issue_number = old_group.get("issue")
            if issue_number:
                github.request(
                    "POST",
                    f"/issues/{issue_number}/comments",
                    {"body": "The latest Test262 sweep no longer reproduces this group; closing as fixed. Reopen if a subsequent run reproduces it."},
                )
                github.request("PATCH", f"/issues/{issue_number}", {"state": "closed"})

    return {"failures": current_failures, "groups": updated_groups}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--test262-dir", required=True, type=Path)
    parser.add_argument("--state-file", required=True, type=Path)
    parser.add_argument("--summary", required=True, type=Path)
    args = parser.parse_args()

    output = args.output.read_text(encoding="utf-8", errors="replace")
    failures = parse_failures(output, args.test262_dir)
    groups = group_failures(failures)
    try:
        state = json.loads(args.state_file.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError):
        state = {}

    token = os.environ.get("GITHUB_TOKEN")
    repository = os.environ.get("GITHUB_REPOSITORY")
    github = GitHub(token, repository) if token and repository else None
    synced = sync_issues(
        groups,
        state,
        github,
        os.environ.get("GITHUB_SERVER_URL", "https://github.com")
        + "/"
        + (repository or "")
        + "/actions/runs/"
        + os.environ.get("GITHUB_RUN_ID", ""),
        int(os.environ.get("TEST262_MAX_OPEN_TASKS", DEFAULT_MAX_OPEN_TASKS)),
    )
    args.state_file.parent.mkdir(parents=True, exist_ok=True)
    args.state_file.write_text(json.dumps(synced, indent=2, sort_keys=True) + "\n")
    args.summary.parent.mkdir(parents=True, exist_ok=True)
    args.summary.write_text(
        json.dumps(
            {
                "failure_count": len(failures),
                "group_count": len(groups),
                "new_failure_count": len(
                    set(synced["failures"]) - set(state.get("failures", {}))
                ),
                "groups": groups,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n"
    )
    print(f"Test262 triage: {len(failures)} failures in {len(groups)} groups")
    return 0


if __name__ == "__main__":
    sys.exit(main())
