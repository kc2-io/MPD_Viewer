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
        self.assertIn("types: [opened, synchronize, reopened, edited, labeled, unlabeled, ready_for_review]", text)
        self.assertIn("python scripts/install-e2e-ffmpeg.py", text)
        self.assertIn("Validate visual evidence (advisory rollout)", text)
        self.assertIn("compression-level: 0", text)
        self.assertIn("e2e-job-summary.py", text)

    def test_reporter_is_metadata_only_with_scoped_comment_permission(self):
        text = (ROOT / ".github/workflows/desktop-e2e-report.yml").read_text(encoding="utf-8")
        self.assertIn("workflow_run:", text)
        self.assertIn("types: [requested, in_progress, completed]", text)
        self.assertEqual(text.count("permissions:"), 1)
        self.assertIn("contents: read", text)
        self.assertIn("actions: read", text)
        self.assertIn("pull-requests: write", text)
        self.assertNotIn("pull-requests: read", text)
        for forbidden in ("pull_request_target", "download-artifact", "actions/cache", "pip install",
                          "secrets.", "id-token: write", "environment:", "--dry-run", "write-all",
                          "contents: write", "actions: write", "issues: write"):
            self.assertNotIn(forbidden, text)
        self.assertIn("ref: ${{ github.sha }}", text)
        self.assertIn("persist-credentials: false", text)
        self.assertIn("e2e-pr-report.py --write", text)
        self.assertEqual(text.count("\n      - uses:"), 2)
        self.assertEqual(text.count("\n      - name:"), 1)
        self.assertIn("- uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1", text)
        self.assertIn("- uses: actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97", text)
        self.assertEqual(run_bodies(text), ["python scripts/e2e-pr-report.py --write"])
        jobs = text.split("\njobs:\n", 1)[1]
        self.assertEqual(re.findall(r"^  ([A-Za-z0-9_-]+):\s*$", jobs, re.MULTILINE), ["report"])
        self.assertIn("head_repository.id", text)
        for body in run_bodies(text):
            self.assertIsNone(re.search(r"\$\{\{\s*(?:github\.event|github\.head_ref)", body))


if __name__ == "__main__":
    unittest.main()
