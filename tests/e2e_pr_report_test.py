#!/usr/bin/env python3
"""Synthetic API fixtures for the reporter's trust and write boundaries."""
import importlib.util
from pathlib import Path
import re
import unittest

spec = importlib.util.spec_from_file_location("report", Path(__file__).resolve().parents[1] / "scripts/e2e-pr-report.py")
report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report)
REPO = "kc2-io/MPD_Viewer"
SHA = "a" * 40


def run():
    return {"id": 123, "run_attempt": 2, "run_number": 20, "event": "pull_request",
            "repository": {"id": 5}, "head_repository": {"id": 7, "owner": {"login": "forker"}},
            "head_branch": "feature", "head_sha": SHA, "name": "Desktop E2E",
            "path": report.PATH, "workflow_id": 99, "status": "completed", "conclusion": "failure",
            "pull_requests": [], "created_at": "2026-09-25T00:00:00Z",
            "updated_at": "2026-09-25T00:10:00Z"}


def pr():
    return {"number": 4, "state": "open", "head": {"sha": SHA, "repo": {"id": 7}},
            "base": {"sha": "b" * 40, "repo": {"id": 5}},
            "created_at": "2026-09-24T00:00:00Z", "closed_at": None}


class FakeApi:
    repo = REPO

    def __init__(self):
        self.mutations = []
        self.comments = []
        self.pull_requests = [pr()]
        self.runs = [run()]
        self.live = run()
        self.current_pr = pr()
        self.jobs_by_attempt = {
            1: [{"name": "GUI (Linux-x64)", "conclusion": "success"}],
            2: [{"name": "GUI (Linux-x64)", "conclusion": "failure"},
                {"name": "evil [click](https://attacker.test)", "conclusion": "success"}],
        }
        self.artifacts = [
            {"id": 88, "name": "Desktop-E2E-Linux-x64-123-attempt2", "expired": False,
             "digest": "sha256:" + "a" * 64, "expires_at": "2026-10-02T00:00:00Z"},
            {"id": 89, "name": "evil [click](https://attacker.test)", "expired": False},
        ]

    def get(self, path, allow_missing=False):
        if path == f"/repos/{REPO}":
            return {"id": 5}
        if "/actions/workflows/" in path:
            return {"id": 99, "path": report.PATH}
        if "/contents/" in path:
            return {"sha": "same"}
        if "/pulls/" in path:
            return self.current_pr
        if "/actions/runs/123" in path:
            return self.live
        raise AssertionError(path)

    def pages(self, path, key=None):
        if "/pulls?" in path:
            return self.pull_requests
        if "/runs?" in path:
            return self.runs
        if "/jobs" in path:
            match = re.search(r"/attempts/(\d+)/jobs", path)
            return self.jobs_by_attempt.get(int(match.group(1)), [])
        if "/artifacts" in path:
            return self.artifacts
        if "/comments" in path:
            return self.comments
        raise AssertionError(path)

    def request(self, method, path, body=None):
        self.mutations.append((method, path, body))


class ReporterTests(unittest.TestCase):
    def test_fork_fallback_and_metadata_only_dry_run(self):
        api = FakeApi()
        content = report.report(api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": run()})
        self.assertIn("[Download](https://github.com/kc2-io/MPD_Viewer/actions/runs/123/artifacts/88)", content)
        self.assertNotIn("attacker.test", content)
        self.assertEqual(api.mutations, [])

    def test_ambiguous_fork_fails_closed(self):
        api = FakeApi()
        api.pull_requests.append(pr())
        with self.assertRaisesRegex(ValueError, "exactly one"):
            report.report(api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": run()}, True)
        self.assertEqual(api.mutations, [])

    def test_closed_old_pr_does_not_rebind_run_to_new_pr(self):
        api = FakeApi()
        old = {**pr(), "state": "closed", "closed_at": "2026-09-25T01:00:00Z"}
        new = {**pr(), "number": 5, "created_at": "2026-09-25T00:01:00Z"}
        api.pull_requests = [new, old]
        result = report.report(api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": run()}, True)
        self.assertIn("moved or closed", result)
        self.assertEqual(api.mutations, [])

    def test_overlapping_historical_prs_fail_closed(self):
        api = FakeApi()
        second = {**pr(), "number": 5, "created_at": "2026-09-24T12:00:00Z"}
        api.pull_requests.append(second)
        with self.assertRaisesRegex(ValueError, "active when the run began"):
            report.report(api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": run()}, True)
        self.assertEqual(api.mutations, [])

    def test_missing_historical_timestamp_fails_closed(self):
        api = FakeApi()
        api.pull_requests[0] = {**pr(), "created_at": None}
        with self.assertRaisesRegex(ValueError, "pull request creation timestamp"):
            report.report(api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": run()}, True)
        self.assertEqual(api.mutations, [])

    def test_supplied_association_created_after_run_fails_closed(self):
        api = FakeApi()
        associated = run()
        associated["pull_requests"] = [{"number": 5}]
        api.current_pr = {**pr(), "number": 5, "created_at": "2026-09-25T00:01:00Z"}
        with self.assertRaisesRegex(ValueError, "not active when the run began"):
            report.report(api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": associated}, True)
        self.assertEqual(api.mutations, [])

    def test_stale_attempt_never_writes(self):
        api = FakeApi()
        newer = run()
        newer["run_attempt"] = 3
        api.runs.append(newer)
        self.assertIn("Skipped stale", report.report(api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": run()}, True))
        self.assertEqual(api.mutations, [])

    def test_copied_marker_is_not_owned(self):
        api = FakeApi()
        api.comments = [{"id": 1, "body": report.MARKER + "\ncopy", "user": {"login": "someone"},
                         "performed_via_github_app": {"slug": "github-actions"}}]
        report.report(api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": run()}, True)
        self.assertEqual(api.mutations[0][0], "POST")

    def test_fixed_actions_bot_comment_with_null_app_is_owned(self):
        comment = {"id": 1, "body": report.MARKER + "\nprior",
                   "user": {"login": "github-actions[bot]", "type": "Bot",
                            "id": report.GITHUB_ACTIONS_BOT_ID},
                   "performed_via_github_app": None}
        self.assertTrue(report.owned_comment(comment))
        self.assertFalse(report.owned_comment({**comment, "user": None}))

    def test_owned_comment_is_updated_not_duplicated(self):
        api = FakeApi()
        api.comments = [{"id": 12, "body": report.MARKER + "\nprior",
                         "user": {"login": "github-actions[bot]", "type": "Bot",
                                  "id": report.GITHUB_ACTIONS_BOT_ID},
                         "performed_via_github_app": None}]
        report.report(api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": run()}, True)
        self.assertEqual(api.mutations[0][0:2], ("PATCH", f"/repos/{REPO}/issues/comments/12"))

    def test_wrong_workflow_id_fails_closed(self):
        api = FakeApi()
        bad = run()
        bad["workflow_id"] = 100
        with self.assertRaisesRegex(ValueError, "workflow identity"):
            report.report(api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": bad}, True)
        self.assertEqual(api.mutations, [])

    def test_delayed_requested_event_uses_live_completed_state(self):
        api = FakeApi()
        old_event = run()
        old_event["status"] = "requested"
        old_event["conclusion"] = None
        content = report.report(api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": old_event})
        self.assertIn("**failure**", content)
        self.assertNotIn("**requested**", content)

    def test_rerun_failed_jobs_keeps_prior_success_artifact(self):
        api = FakeApi()
        api.jobs_by_attempt[2] = [{"name": "GUI (Windows-x64)", "conclusion": "failure"}]
        api.artifacts = [
            {"id": 77, "name": "Desktop-E2E-Linux-x64-123-attempt1", "expired": False},
            {"id": 78, "name": "Desktop-E2E-Windows-x64-123-attempt2", "expired": False},
        ]
        content = report.report(api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": run()})
        self.assertIn("actions/runs/123/artifacts/77", content)
        self.assertIn("| Linux-x64 | success |", content)
        self.assertIn("actions/runs/123/artifacts/78", content)
        self.assertIn("| Windows-x64 | failure |", content)

    def test_head_changes_before_write_skips_comment(self):
        api = FakeApi()
        first = pr()
        api.pull_requests = [first]
        api.current_pr = {**first, "head": {"sha": "c" * 40, "repo": {"id": 7}}}
        self.assertIn("Skipped PR whose head", report.report(
            api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": run()}, True))
        self.assertEqual(api.mutations, [])

    def test_base_changes_before_write_skips_comment(self):
        api = FakeApi()
        first = pr()
        api.pull_requests = [first]
        api.current_pr = {**first, "base": {"sha": "c" * 40, "repo": {"id": 5}}}
        self.assertIn("head, base or newest run", report.report(
            api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": run()}, True))
        self.assertEqual(api.mutations, [])

    def test_stale_event_head_is_skipped_without_error(self):
        api = FakeApi()
        api.pull_requests[0] = {**pr(), "head": {"sha": "c" * 40, "repo": {"id": 7}}}
        self.assertIn("moved or closed", report.report(
            api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": run()}))

    def test_null_nested_run_fields_fail_closed(self):
        api = FakeApi()
        broken = run()
        broken["head_repository"] = None
        with self.assertRaisesRegex(ValueError, "head repository"):
            report.report(api, {"repository": {"full_name": REPO, "id": 5}, "workflow_run": broken}, True)
        self.assertEqual(api.mutations, [])


if __name__ == "__main__":
    unittest.main()
