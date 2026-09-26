#!/usr/bin/env python3
"""Copy only bounded top-level GUI evidence; never recurse into test profiles."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

MAX_FILES = 160
MAX_FILE_BYTES = 8 * 1024 * 1024
MAX_VIDEO_BYTES = 32 * 1024 * 1024
MAX_TOTAL_BYTES = 256 * 1024 * 1024
TYPE_LIMITS = {"report": (24, 24 * 1024 * 1024), "log": (16, 24 * 1024 * 1024),
               "priority_png": (32, 48 * 1024 * 1024), "video": (24, 192 * 1024 * 1024),
               "checkpoint": (64, 64 * 1024 * 1024)}
SENSITIVE = ("cookie", "credential", "profile", "token", "vault", "secret", "password")
VIDEO_NAME = re.compile(r"video-[A-Za-z0-9_-]+\.(?:mp4|webm)\Z")
SAFE_NAME = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,99}\Z")


def kind(path):
    name = path.name
    if path.suffix in {".json", ".xml"} and len(name) <= 100:
        return "report"
    if path.suffix == ".log" and len(name) <= 100:
        return "log"
    if VIDEO_NAME.fullmatch(name):
        return "video"
    if path.suffix == ".png" and len(name) <= 100:
        return "checkpoint" if name.startswith("checkpoint-") else "priority_png"
    return None


def video_valid(path, probe=None):
    """Require a decodable video stream with positive duration and frames."""
    probe = probe or shutil.which("ffprobe")
    if not probe:
        return False, "ffprobe unavailable"
    try:
        result = subprocess.run([str(probe), "-v", "error", "-count_frames", "-show_entries",
                                 "stream=codec_type,codec_name,width,height,nb_read_frames,duration:format=duration",
                                 "-of", "json", str(path)], capture_output=True, text=True, timeout=30)
        data = json.loads(result.stdout) if result.returncode == 0 else {}
        if result.stderr.strip():
            return False, "decode errors reported"
        streams = data.get("streams", [])
        if any(stream.get("codec_type") == "audio" for stream in streams):
            return False, "audio stream present"
        stream = next((stream for stream in streams if stream.get("codec_type") == "video"), {})
        duration = float(stream.get("duration") or data.get("format", {}).get("duration") or 0)
        frames = int(stream.get("nb_read_frames") or 0)
        if stream.get("codec_type") != "video" or duration <= 0 or frames <= 0:
            return False, "no decodable frames"
        if stream.get("codec_name") != "h264":
            return False, "video codec is not H.264"
        width, height = int(stream.get("width") or 0), int(stream.get("height") or 0)
        if width <= 0 or height <= 0 or width > 1280 or height > 800:
            return False, "video dimensions exceed evidence bounds"
        return True, "valid"
    except (OSError, ValueError, TypeError, subprocess.TimeoutExpired):
        return False, "probe failed"


def stage(source, destination):
    if source.is_symlink() or destination.is_symlink():
        raise ValueError("Evidence roots must not be symlinks")
    destination.mkdir(parents=True, exist_ok=True)
    copied, rejected, total = [], [], 0
    counts = {key: 0 for key in TYPE_LIMITS}
    bytes_by_type = {key: 0 for key in TYPE_LIMITS}
    candidates = []
    if source.exists():
        for path in source.iterdir():
            category = kind(path)
            if category is None:
                continue
            if any(term in path.name.lower() for term in SENSITIVE):
                rejected.append({"name": "[sensitive filename]", "reason": "sensitive filename"})
                continue
            if path.is_symlink():
                rejected.append({"name": "[symlink]", "reason": "symlink"})
                continue
            if not path.is_file():
                continue
            if not SAFE_NAME.fullmatch(path.name):
                rejected.append({"name": "[unsafe filename]", "reason": "unsafe filename"})
                continue
            candidates.append((category, path))
    priority = {"report": 0, "log": 1, "priority_png": 2, "video": 3, "checkpoint": 4}
    for category, path in sorted(candidates, key=lambda item: (priority[item[0]], item[1].name)):
        size = path.stat().st_size
        count_limit, byte_limit = TYPE_LIMITS[category]
        per_file = MAX_VIDEO_BYTES if category == "video" else MAX_FILE_BYTES
        if (len(copied) >= MAX_FILES or counts[category] >= count_limit or size > per_file
                or bytes_by_type[category] + size > byte_limit or total + size > MAX_TOTAL_BYTES):
            rejected.append({"name": path.name, "reason": "evidence size ceiling"})
            continue
        if category in {"checkpoint", "priority_png"}:
            with path.open("rb") as image:
                if image.read(8) != b"\x89PNG\r\n\x1a\n":
                    rejected.append({"name": path.name, "reason": "invalid PNG signature"})
                    continue
        target = destination / path.name
        if target.exists() or target.is_symlink():
            raise ValueError("Refusing to overwrite previously staged evidence")
        digest = hashlib.sha256()
        with path.open("rb") as source_file, target.open("xb") as target_file:
            remaining = size
            while remaining and (chunk := source_file.read(min(1024 * 1024, remaining))):
                digest.update(chunk)
                target_file.write(chunk)
                remaining -= len(chunk)
        if remaining:
            target.unlink()
            rejected.append({"name": path.name, "reason": "evidence changed during copy"})
            continue
        if category == "video":
            valid, reason = video_valid(target)
            if not valid:
                target.unlink()
                rejected.append({"name": path.name, "reason": reason})
                continue
        copied.append({"name": path.name, "bytes": size, "sha256": digest.hexdigest(), "type": category})
        counts[category] += 1
        bytes_by_type[category] += size
        total += size
    advisory_video = {"ffprobe unavailable", "probe failed", "no decodable frames",
                      "decode errors reported", "audio stream present", "video codec is not H.264",
                      "video dimensions exceed evidence bounds", "evidence size ceiling"}
    critical = [item for item in rejected if not (
        (item["name"].endswith((".mp4", ".webm")) and item["reason"] in advisory_video)
        or (item["reason"] == "evidence size ceiling" and item["name"].startswith("checkpoint-")
            and item["name"].endswith(".png")))]
    summary = {
        "schema_version": 2,
        "commit": os.environ.get("GITHUB_SHA", "local"),
        "job_status": os.environ.get("MPD_E2E_JOB_STATUS", "unknown"),
        "files": copied,
        "bytes": total,
        "bytes_by_type": bytes_by_type,
        "rejected": rejected,
        "critical_rejections": critical,
        "missing_evidence": not copied,
    }
    (destination / "upload-summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    (destination / "SHA256SUMS.txt").write_text(
        "".join(f'{item["sha256"]}  {item["name"]}\n' for item in copied), encoding="utf-8")
    return bool(copied) and not critical


def main():
    source = Path(os.environ["MPD_E2E_OUTPUT_DIR"])
    destination = Path(os.environ.get("RUNNER_TEMP", tempfile.gettempdir())) / "mpd-e2e-upload"
    return 0 if stage(source, destination) else 1


if __name__ == "__main__":
    sys.exit(main())
