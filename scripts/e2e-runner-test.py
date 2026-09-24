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


class ReleaseGraphBoundaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        boundary_spec = importlib.util.spec_from_file_location("boundary", Path(__file__).with_name("e2e-release-boundary.py"))
        cls.boundary = importlib.util.module_from_spec(boundary_spec)
        boundary_spec.loader.exec_module(cls.boundary)

    def test_rejects_transitive_fixture_even_without_driver(self):
        graph = "mpd-tabber v0.1.0|custom-protocol\nmpd-twitch v0.1.0|e2e-tests\n"
        result = self.boundary.assess_graph(graph)
        self.assertEqual(result["violations"], ["Test fixture feature is active: mpd-twitch/e2e-tests"])

    def test_rejects_driver_even_without_named_feature(self):
        graph = "mpd-tabber v0.1.0|custom-protocol\ntauri-plugin-wdio-webdriver v1.4.0|default (*)\n"
        result = self.boundary.assess_graph(graph)
        self.assertEqual(result["violations"], ["Automation dependency is reachable: tauri-plugin-wdio-webdriver"])

    def test_empty_or_unexpected_graph_cannot_pass(self):
        for graph in ("", "cargo resolution failed", "unrelated v1.0.0|default\n"):
            with self.subTest(graph=graph), self.assertRaises(ValueError):
                self.boundary.assess_graph(graph)


class NormalProbeBoundaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        probe_spec = importlib.util.spec_from_file_location("probe", Path(__file__).with_name("e2e-no-driver-probe.py"))
        cls.probe = importlib.util.module_from_spec(probe_spec)
        probe_spec.loader.exec_module(cls.probe)

    def test_local_or_self_hosted_execution_is_rejected_before_profile_access(self):
        for environment in ({}, {"GITHUB_ACTIONS": "true", "MPD_E2E_RUNNER_ENVIRONMENT": "self-hosted"},
                            {"GITHUB_ACTIONS": "false", "MPD_E2E_RUNNER_ENVIRONMENT": "github-hosted"}):
            with self.subTest(environment=environment), self.assertRaises(ValueError):
                self.probe.require_hosted(environment)

    def test_cleanup_root_requires_matching_owner_marker(self):
        with tempfile.TemporaryDirectory(prefix="mpd-probe-boundary-") as directory:
            parent = Path(directory)
            root = parent / "mpd-normal-e2e-test"
            root.mkdir()
            (root / self.probe.MARKER).write_text("b" * 32)
            environment = {"GITHUB_ACTIONS": "true", "MPD_E2E_RUNNER_ENVIRONMENT": "github-hosted",
                           "RUNNER_TEMP": str(parent), "MPD_E2E_PRODUCTION_ROOT": str(root),
                           "MPD_E2E_PRODUCTION_RUN_ID": "a" * 32}
            with self.assertRaises(ValueError):
                self.probe.owned_root(environment)

    def test_cleanup_root_cannot_escape_runner_temp(self):
        with tempfile.TemporaryDirectory(prefix="mpd-probe-boundary-") as directory:
            parent = Path(directory)
            runner = parent / "runner"
            runner.mkdir()
            outside = parent / "mpd-normal-e2e-outside"
            outside.mkdir()
            (outside / self.probe.MARKER).write_text("a" * 32)
            environment = {"GITHUB_ACTIONS": "true", "MPD_E2E_RUNNER_ENVIRONMENT": "github-hosted",
                           "RUNNER_TEMP": str(runner), "MPD_E2E_PRODUCTION_ROOT": str(outside),
                           "MPD_E2E_PRODUCTION_RUN_ID": "a" * 32}
            with self.assertRaises(ValueError):
                self.probe.owned_root(environment)


if __name__ == "__main__":
    unittest.main()
