#!/usr/bin/env python3
"""Verify that native GUI artifact staging cannot scoop up profile contents."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("evidence", Path(__file__).with_name("e2e-stage-evidence.py"))
evidence = importlib.util.module_from_spec(spec)
spec.loader.exec_module(evidence)


class EvidenceBoundaryTests(unittest.TestCase):
    def test_upload_only_root_evidence_and_no_profile_database(self):
        with tempfile.TemporaryDirectory(prefix="mpd-e2e-stage-") as directory:
            root = Path(directory)
            source, target = root / "source", root / "upload"
            source.mkdir()
            (source / "run.json").write_text("{}")
            (source / "state.sqlite").write_bytes(b"private database")
            profile = source / "profile"
            profile.mkdir()
            (profile / "cookies.json").write_text("private profile")
            self.assertTrue(evidence.stage(source, target))
            self.assertEqual({item.name for item in target.iterdir()}, {"run.json", "upload-summary.json"})

    def test_sensitive_filename_and_oversize_fail_closed(self):
        with tempfile.TemporaryDirectory(prefix="mpd-e2e-stage-") as directory:
            root = Path(directory)
            source, target = root / "source", root / "upload"
            source.mkdir()
            (source / "fake-token.json").write_text("do not collect")
            with (source / "huge.log").open("wb") as output:
                output.truncate(evidence.MAX_FILE_BYTES + 1)
            self.assertFalse(evidence.stage(source, target))
            result = json.loads((target / "upload-summary.json").read_text())
            self.assertEqual(len(result["rejected"]), 2)
            self.assertEqual(result["files"], [])

    def test_existing_destination_is_not_overwritten(self):
        with tempfile.TemporaryDirectory(prefix="mpd-e2e-stage-") as directory:
            root = Path(directory)
            source, target = root / "source", root / "upload"
            source.mkdir()
            target.mkdir()
            (source / "run.log").write_text("new")
            (target / "run.log").write_text("existing")
            with self.assertRaises(ValueError):
                evidence.stage(source, target)
            self.assertEqual((target / "run.log").read_text(), "existing")

    def test_missing_output_produces_failure_summary(self):
        with tempfile.TemporaryDirectory(prefix="mpd-e2e-stage-") as directory:
            root = Path(directory)
            self.assertFalse(evidence.stage(root / "missing", root / "upload"))
            result = json.loads((root / "upload" / "upload-summary.json").read_text())
            self.assertTrue(result["missing_evidence"])


if __name__ == "__main__":
    unittest.main()
