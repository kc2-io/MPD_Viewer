#!/usr/bin/env python3
"""Write a fixed-label per-platform Actions job summary after artifact upload."""
import json
import os
from pathlib import Path
import re
import sys

PLATFORMS = {"Windows-x64", "macOS-arm64", "macOS-x64", "Linux-x64"}
STATUSES = {"success", "failure", "cancelled", "skipped", "timed_out", "neutral", "action_required"}


def render(directory, environment):
    platform = environment.get("MPD_E2E_PLATFORM", "unknown")
    if platform not in PLATFORMS:
        platform = "unknown"
    status = environment.get("MPD_E2E_JOB_STATUS", "unknown")
    if status not in STATUSES:
        status = "unknown"
    sha = environment.get("GITHUB_SHA", "")
    if not re.fullmatch(r"[0-9a-f]{40}", sha):
        sha = "unknown"
    try:
        data = json.loads((directory / "upload-summary.json").read_text(encoding="utf-8"))
        files = data.get("files", []) if isinstance(data, dict) else []
        if not isinstance(files, list):
            files = []
    except (OSError, ValueError):
        files = []
    video_count = sum(item.get("type") == "video" for item in files if isinstance(item, dict))
    screenshot_count = sum(item.get("type") in {"checkpoint", "priority_png"}
                           for item in files if isinstance(item, dict))
    outcome = environment.get("MPD_E2E_UPLOAD_OUTCOME", "unknown")
    url = environment.get("MPD_E2E_UPLOAD_URL", "")
    digest = environment.get("MPD_E2E_UPLOAD_DIGEST", "")
    if outcome == "success" and re.fullmatch(r"https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/actions/runs/\d+/artifacts/\d+", url):
        artifact = f"[Download artifact]({url})"
    else:
        artifact = "Unavailable (upload did not succeed)"
    if not re.fullmatch(r"sha256:[0-9a-f]{64}", digest):
        digest = "unavailable"
    try:
        validation = json.loads((directory / "visual-validation.json").read_text(encoding="utf-8"))
        visual_status = "valid" if isinstance(validation, dict) and validation.get("valid") is True else "advisory warning"
    except (OSError, ValueError):
        visual_status = "not available"
    revision_label = "merge commit tested" if environment.get("GITHUB_EVENT_NAME") == "pull_request" else "commit tested"
    return (f"### Desktop E2E visual evidence: {platform}\n\n"
            f"Job: {status}; {revision_label}: `{sha}`.\n\n"
            f"Video segments: {video_count}; screenshots: {screenshot_count}; "
            f"visual validation: {visual_status}.\n\n"
            f"{artifact}; artifact digest: `{digest}`. Retained for seven days.\n\n"
            "Controlled fixtures only; this does not establish live Twitch behavior.\n")


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: e2e-job-summary.py STAGED_DIR")
    content = render(Path(sys.argv[1]), os.environ)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as output:
            output.write(content)
    else:
        print(content)


if __name__ == "__main__":
    main()
