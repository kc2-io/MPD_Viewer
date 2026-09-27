#!/usr/bin/env python3
"""Classify a checked-out revision as full CI or non-build-only CI."""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import subprocess
import sys


SHA = re.compile(r"[0-9a-f]{40}\Z")
SAFE_EXACT = {
    ".github/CODEOWNERS",
    ".github/pull_request_template.md",
    "LICENSE",
}
SAFE_PREFIXES = (
    ".github/ISSUE_TEMPLATE/",
    "docs/",
)


def safe_path(path: str) -> bool:
    return (
        path.lower().endswith(".md")
        or path in SAFE_EXACT
        or path.startswith(SAFE_PREFIXES)
    )


def classify_paths(paths: list[str]) -> tuple[bool, str]:
    """Return (full, reason), defaulting ambiguous input to the full suite."""
    if not paths:
        return True, "empty-diff"
    for path in paths:
        if not path or path.startswith("/") or "\0" in path or not safe_path(path):
            return True, "build-impacting"
    return False, "non-build-only"


def git(*args: str, cwd: Path) -> bytes:
    return subprocess.run(
        ["git", *args],
        cwd=cwd,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    ).stdout


def classify_revisions(base: str, head: str, cwd: Path) -> tuple[bool, str]:
    if not SHA.fullmatch(base) or not SHA.fullmatch(head):
        raise ValueError("base and head must be full lowercase commit SHAs")
    if base == "0" * 40 or head == "0" * 40:
        raise ValueError("all-zero commit SHA is not classifiable")

    checkout = git("rev-parse", "--verify", "HEAD^{commit}", cwd=cwd).decode("ascii").strip()
    if checkout != head:
        raise ValueError("classified head is not the checked-out commit")
    git("cat-file", "-e", f"{base}^{{commit}}", cwd=cwd)
    git("cat-file", "-e", f"{head}^{{commit}}", cwd=cwd)
    git("merge-base", "--is-ancestor", base, head, cwd=cwd)
    raw = git("diff", "--name-only", "--no-renames", "-z", base, head, "--", cwd=cwd)
    try:
        paths = [item.decode("utf-8") for item in raw.split(b"\0") if item]
    except UnicodeDecodeError as error:
        raise ValueError("changed path is not valid UTF-8") from error
    return classify_paths(paths)


def emit(full: bool, reason: str, base: str, head: str) -> None:
    allowed_reasons = {"build-impacting", "classification-error", "empty-diff", "forced-full", "non-build-only"}

    def revision(value: str) -> str:
        if SHA.fullmatch(value) and value != "0" * 40:
            return value
        if value in {"not-applicable", "unavailable"}:
            return value
        return "invalid"

    print(f"full={'true' if full else 'false'}")
    print(f"reason={reason if reason in allowed_reasons else 'classification-error'}")
    print(f"base={revision(base)}")
    print(f"head={revision(head)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base")
    parser.add_argument("--head")
    parser.add_argument("--full", action="store_true", help="force full CI for schedule/manual runs")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]

    if args.full:
        try:
            head = git("rev-parse", "--verify", "HEAD^{commit}", cwd=root).decode("ascii").strip()
        except (OSError, UnicodeDecodeError, subprocess.CalledProcessError):
            head = "unavailable"
        emit(True, "forced-full", "not-applicable", head)
        return 0

    base = args.base or "missing"
    head = args.head or "missing"
    try:
        full, reason = classify_revisions(base, head, root)
    except (OSError, UnicodeDecodeError, ValueError, subprocess.CalledProcessError) as error:
        print(f"CI impact classification selected full execution: {error}", file=sys.stderr)
        emit(True, "classification-error", base, head)
        return 0
    emit(full, reason, base, head)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
