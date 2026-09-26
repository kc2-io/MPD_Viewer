#!/usr/bin/env python3
"""Focused boundary tests for visual evidence staging and validation."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1] / "scripts"


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / filename)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


stage = module("stage", "e2e-stage-evidence.py")
validate = module("validate", "e2e-validate-evidence.py")
runtime = module("runtime", "e2e-runtime-manifest.py")


class EvidenceTests(unittest.TestCase):
    def test_interrupt_path_bounds_settle_and_finalizes_recorder_first(self):
        root = Path(__file__).resolve().parents[1]
        runner = (root / "tests/e2e/run.mjs").read_text(encoding="utf-8")
        support = (root / "tests/e2e/support.mjs").read_text(encoding="utf-8")
        self.assertIn("new Promise(resolve => setTimeout(resolve, 2000))", runner)
        self.assertLess(runner.index("await recorder.stop"), runner.index("await app.abortOwned"))
        self.assertIn("async abortOwned()", support)
        self.assertIn("await terminateOwned(this.child)", support)

    def test_reports_survive_checkpoint_quota_and_hashes_match(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, target = root / "source", root / "target"
            source.mkdir()
            (source / "manifest.json").write_text("{}")
            (source / "results.xml").write_text("<testsuites/>")
            for index in range(70):
                (source / f"checkpoint-{index:03d}.png").write_bytes(b"\x89PNG\r\n\x1a\n")
            for index in range(10):
                (source / f"demo-milestone-{index:02d}.png").write_bytes(b"\x89PNG\r\n\x1a\n")
            for index in range(8):
                (source / f"failure-{index:02d}.png").write_bytes(b"\x89PNG\r\n\x1a\n")
            (source / ".DS_Store").write_bytes(b"ignored non-evidence")
            self.assertTrue(stage.stage(source, target))
            result = json.loads((target / "upload-summary.json").read_text())
            self.assertIn("manifest.json", {item["name"] for item in result["files"]})
            self.assertIn("results.xml", {item["name"] for item in result["files"]})
            self.assertEqual(result["bytes_by_type"]["checkpoint"], 64 * 8)
            self.assertEqual(result["bytes_by_type"]["priority_png"], 18 * 8)
            self.assertEqual(len([item for item in result["rejected"] if item["reason"] == "evidence size ceiling"]), 6)
            self.assertEqual(result["critical_rejections"], [])
            self.assertTrue(all((target / f"failure-{index:02d}.png").is_file() for index in range(8)))
            self.assertIn("manifest.json", (target / "SHA256SUMS.txt").read_text())

    def test_rejects_link_sensitive_name_invalid_png_and_bad_video(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, target = root / "source", root / "target"
            source.mkdir()
            (source / "manifest.json").write_text("{}")
            (source / "secret-token.json").write_text("private")
            (source / "checkpoint-001.png").write_bytes(b"not a png")
            (source / "video-001.mp4").write_bytes(b"broken")
            (source / "checkpoint-002.png").symlink_to(source / "checkpoint-001.png")
            with patch.object(stage, "video_valid", return_value=(False, "no decodable frames")):
                self.assertFalse(stage.stage(source, target))
            reasons = {item["reason"] for item in json.loads((target / "upload-summary.json").read_text())["rejected"]}
            self.assertEqual(reasons, {"sensitive filename", "invalid PNG signature", "no decodable frames", "symlink"})

    def test_invalid_video_is_advisory_during_staging(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, target = root / "source", root / "target"
            source.mkdir()
            (source / "manifest.json").write_text("{}")
            (source / "video-000.mp4").write_bytes(b"broken")
            with patch.object(stage, "video_valid", return_value=(False, "no decodable frames")):
                self.assertTrue(stage.stage(source, target))
            result = json.loads((target / "upload-summary.json").read_text())
            self.assertEqual(result["critical_rejections"], [])
            self.assertFalse((target / "video-000.mp4").exists())

    def test_validator_reports_absence_and_accepts_complete_manifest(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.assertIn("upload summary missing or invalid", validate.assess(root))
            source, target = root / "source", root / "target"
            source.mkdir()
            manifest = {"visualEvidence": {"schema": 1, "recorder": {"audio": False,
                        "stopMode": "graceful", "segments": [{"name": "video-001.mp4", "validated": True}]}}}
            (source / "manifest.json").write_text(json.dumps(manifest))
            (source / "runner-manifest.json").write_text("{}")
            (source / "results.xml").write_text("<testsuites/>")
            (source / "checkpoint-001.png").write_bytes(b"\x89PNG\r\n\x1a\n")
            (source / "video-001.mp4").write_bytes(b"synthetic")
            with patch.object(stage, "video_valid", return_value=(True, "valid")):
                self.assertTrue(stage.stage(source, target))
            with patch.object(validate.STAGE, "video_valid", return_value=(True, "valid")):
                self.assertEqual(validate.assess(target), [])

    def test_malformed_upload_summary_is_reported_not_crashed(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "upload-summary.json").write_text("[]")
            self.assertEqual(validate.assess(root), ["upload summary missing or invalid"])

    def test_tool_identity_tolerates_empty_or_failed_version_probe(self):
        with tempfile.TemporaryDirectory() as temporary:
            executable = Path(temporary) / "ffmpeg"
            executable.write_bytes(b"fixture executable")
            with patch.object(runtime.shutil, "which", return_value=str(executable)), \
                    patch.object(runtime, "command", return_value={"error": "TimeoutExpired"}):
                identity = runtime.tool_identity("ffmpeg")
            self.assertTrue(identity["available"])
            self.assertEqual(identity["version"], "")
            self.assertEqual(identity["probe_error"], "TimeoutExpired")


if __name__ == "__main__":
    unittest.main()
