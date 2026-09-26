"""Static privilege and wiring checks for Desktop E2E visual evidence."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]


def run_bodies(text):
    lines = text.splitlines()
    bodies = []
    for index, line in enumerate(lines):
        match = re.match(r"^(\s*)run:\s*(.*)$", line)
        if not match:
            continue
        indent, value = len(match.group(1)), match.group(2)
        if value not in {"|", ">", "|-", ">-"}:
            bodies.append(value)
            continue
        block = []
        for following in lines[index + 1:]:
            if following.strip() and len(following) - len(following.lstrip()) <= indent:
                break
            block.append(following)
        bodies.append("\n".join(block))
    return bodies


class WorkflowPolicyTests(unittest.TestCase):
    def test_capture_workflow_stays_unprivileged_and_uploads_visual_evidence(self):
        text = (ROOT / ".github/workflows/desktop-e2e.yml").read_text(encoding="utf-8")
        self.assertIn("permissions:\n  contents: read", text)
        self.assertNotIn("pull_request_target", text)
        self.assertNotIn("id-token: write", text)
        self.assertNotIn("secrets.", text)
        self.assertNotIn("environment:", text)
        self.assertIn("MPD_E2E_RECORD_VIDEO: '1'", text)
        self.assertIn("MPD_E2E_CHECKPOINTS:", text)
        self.assertIn("python scripts/install-e2e-ffmpeg.py", text)
        self.assertIn("Validate visual evidence (advisory rollout)", text)
        self.assertIn("compression-level: 0", text)
        self.assertIn("e2e-job-summary.py", text)

    def test_reporter_is_metadata_only_and_read_only_during_rollout(self):
        text = (ROOT / ".github/workflows/desktop-e2e-report.yml").read_text(encoding="utf-8")
        self.assertIn("workflow_run:", text)
        self.assertIn("types: [requested, in_progress, completed]", text)
        self.assertEqual(text.count("permissions:"), 1)
        self.assertIn("contents: read", text)
        self.assertIn("actions: read", text)
        self.assertIn("pull-requests: read", text)
        self.assertNotIn("pull-requests: write", text)
        for forbidden in ("pull_request_target", "download-artifact", "actions/cache", "pip install",
                          "secrets.", "id-token: write", "environment:", "--write", "write-all",
                          "contents: write", "actions: write", "issues: write"):
            self.assertNotIn(forbidden, text)
        self.assertIn("ref: ${{ github.sha }}", text)
        self.assertIn("persist-credentials: false", text)
        self.assertIn("e2e-pr-report.py --dry-run", text)
        self.assertIn("head_repository.id", text)
        for body in run_bodies(text):
            self.assertIsNone(re.search(r"\$\{\{\s*(?:github\.event|github\.head_ref)", body))


if __name__ == "__main__":
    unittest.main()
