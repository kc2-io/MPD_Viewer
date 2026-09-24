#!/usr/bin/env python3
"""Copy only bounded top-level GUI evidence; never recurse into test profiles."""
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile

ALLOWED_SUFFIXES = {".json", ".xml", ".log", ".png"}
MAX_FILES = 128
MAX_FILE_BYTES = 8 * 1024 * 1024
MAX_TOTAL_BYTES = 64 * 1024 * 1024


def stage(source, destination):
    if source.is_symlink() or destination.is_symlink():
        raise ValueError("Evidence roots must not be symlinks")
    destination.mkdir(parents=True, exist_ok=True)
    copied, rejected, total = [], [], 0
    if source.exists():
        for path in sorted(source.iterdir()):
            if path.is_symlink():
                rejected.append({"name": path.name, "reason": "symlink"})
                continue
            # Profiles/SQLite/credentials are not eligible even if nested filenames
            # resemble logs. Only four evidence formats at the output root qualify.
            if not path.is_file() or path.suffix.lower() not in ALLOWED_SUFFIXES:
                continue
            if any(term in path.name.lower() for term in ("cookie", "credential", "profile", "token", "vault")):
                rejected.append({"name": path.name, "reason": "sensitive filename"})
                continue
            size = path.stat().st_size
            if len(copied) >= MAX_FILES or size > MAX_FILE_BYTES or total + size > MAX_TOTAL_BYTES:
                rejected.append({"name": path.name, "reason": "evidence size ceiling"})
                continue
            target = destination / path.name
            if target.exists() or target.is_symlink():
                raise ValueError("Refusing to overwrite previously staged evidence")
            shutil.copyfile(path, target)
            copied.append({"name": path.name, "bytes": size})
            total += size
    summary = {
        "commit": os.environ.get("GITHUB_SHA", "local"),
        "job_status": os.environ.get("MPD_E2E_JOB_STATUS", "unknown"),
        "files": copied,
        "bytes": total,
        "rejected": rejected,
        "missing_evidence": not copied,
    }
    (destination / "upload-summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    return bool(copied) and not rejected


def main():
    source = Path(os.environ["MPD_E2E_OUTPUT_DIR"])
    destination = Path(os.environ.get("RUNNER_TEMP", tempfile.gettempdir())) / "mpd-e2e-upload"
    return 0 if stage(source, destination) else 1


if __name__ == "__main__":
    sys.exit(main())
