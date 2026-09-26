#!/usr/bin/env python3
"""Inspect staged visual evidence; advisory mode preserves the GUI result."""
import argparse
import importlib.util
import json
from pathlib import Path
import sys

STAGE_SPEC = importlib.util.spec_from_file_location("e2e_stage", Path(__file__).with_name("e2e-stage-evidence.py"))
STAGE = importlib.util.module_from_spec(STAGE_SPEC)
STAGE_SPEC.loader.exec_module(STAGE)


def assess(directory, details=None):
    problems = []
    try:
        upload = json.loads((directory / "upload-summary.json").read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return ["upload summary missing or invalid"]
    if not isinstance(upload, dict):
        return ["upload summary missing or invalid"]
    files = upload.get("files", [])
    if not isinstance(files, list):
        return ["upload file list missing or invalid"]
    names = {entry.get("name") for entry in files if isinstance(entry, dict)}
    for name in ("manifest.json", "runner-manifest.json", "results.xml"):
        if name not in names or not (directory / name).is_file():
            problems.append(f"required report missing: {name}")
    video_names = [name for name in names if isinstance(name, str) and name.startswith("video-")]
    if not video_names:
        problems.append("validated video missing")
    video_results = {}
    for name in sorted(video_names):
        if not STAGE.VIDEO_NAME.fullmatch(name) or not (directory / name).is_file() or (directory / name).is_symlink():
            video_results[name] = {"validated": False, "reason": "invalid staged video path"}
        else:
            good, reason = STAGE.video_valid(directory / name)
            video_results[name] = {"validated": good, "reason": reason}
        if not video_results[name]["validated"]:
            problems.append(f"video segment invalid: {name}")
    if details is not None:
        details.update(video_results)
    if not any(isinstance(name, str) and name.endswith(".png") for name in names):
        problems.append("checkpoint screenshot missing")
    critical = upload.get("critical_rejections", upload.get("rejected", []))
    if not isinstance(critical, list):
        problems.append("critical rejection list invalid")
    elif critical:
        problems.append(f"{len(critical)} critical evidence file(s) rejected")
    try:
        manifest = json.loads((directory / "manifest.json").read_text(encoding="utf-8"))
        if not isinstance(manifest, dict):
            raise TypeError("manifest must be an object")
        visual = manifest.get("visualEvidence", {})
        if not isinstance(visual, dict):
            raise TypeError("visual evidence must be an object")
        if visual.get("schema") != 1:
            problems.append("visual evidence manifest missing or incompatible")
        recorder = visual.get("recorder", {})
        if not isinstance(recorder, dict):
            raise TypeError("recorder evidence must be an object")
        if recorder.get("audio") is not False:
            problems.append("silent recorder evidence missing")
        if recorder.get("stopMode") not in {"graceful", "escalated"}:
            problems.append("recorder did not finalize")
        segments = recorder.get("segments", [])
        if not isinstance(segments, list) or not segments or not all(
                isinstance(segment, dict) and segment.get("name") in video_results
                and video_results[segment["name"]]["validated"] for segment in segments):
            problems.append("validated segment manifest missing or inconsistent")
    except (OSError, ValueError, AttributeError, TypeError):
        problems.append("visual evidence manifest invalid")
    return problems


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    parser.add_argument("--mode", choices=("advisory", "required"), default="advisory")
    args = parser.parse_args()
    segments = {}
    problems = assess(args.directory, segments)
    report = {"schema_version": 1, "mode": args.mode, "valid": not problems,
              "problems": problems, "segments": segments}
    args.directory.mkdir(parents=True, exist_ok=True)
    (args.directory / "visual-validation.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print("Visual evidence: " + ("valid" if not problems else "invalid: " + "; ".join(problems)))
    return 1 if problems and args.mode == "required" else 0


if __name__ == "__main__":
    sys.exit(main())
