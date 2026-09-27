import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("ci_impact", ROOT / "scripts" / "ci-impact.py")
CI_IMPACT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CI_IMPACT)


class ImpactTests(unittest.TestCase):
    def test_only_explicit_non_build_paths_are_lightweight(self):
        paths = [
            "README.md",
            "crates/core/AGENTS.md",
            "docs/guide/image.png",
            ".github/ISSUE_TEMPLATE/config.yml",
            ".github/pull_request_template.md",
            ".github/CODEOWNERS",
            "LICENSE",
        ]
        self.assertEqual(CI_IMPACT.classify_paths(paths), (False, "non-build-only"))

    def test_build_and_automation_paths_force_full(self):
        for path in (
            "src-tauri/src/main.rs",
            "ui/app.js",
            "web/player-wrapper/index.html",
            "tests/e2e/specs.mjs",
            "scripts/ci-impact.py",
            "justfile",
            "Cargo.lock",
            ".github/workflows/ci.yml",
            ".github/action-pins.json",
            ".gitignore",
        ):
            with self.subTest(path=path):
                self.assertEqual(CI_IMPACT.classify_paths([path]), (True, "build-impacting"))

    def test_empty_and_large_mixed_diffs_force_full(self):
        self.assertEqual(CI_IMPACT.classify_paths([]), (True, "empty-diff"))
        paths = [f"docs/generated/{number}.txt" for number in range(3_001)]
        paths.append("src-tauri/src/main.rs")
        self.assertEqual(CI_IMPACT.classify_paths(paths), (True, "build-impacting"))

    def test_renames_are_seen_as_delete_and_add(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            self.git(repo, "init", "-q")
            self.git(repo, "config", "user.email", "ci@example.invalid")
            self.git(repo, "config", "user.name", "CI Test")
            (repo / "src").mkdir()
            (repo / "src" / "app.rs").write_text("fn main() {}\n", encoding="utf-8")
            self.git(repo, "add", ".")
            self.git(repo, "commit", "-qm", "base")
            base = self.git(repo, "rev-parse", "HEAD").strip()

            (repo / "docs").mkdir()
            self.git(repo, "mv", "src/app.rs", "docs/app.md")
            self.git(repo, "commit", "-qm", "move code to docs")
            head = self.git(repo, "rev-parse", "HEAD").strip()
            self.assertEqual(CI_IMPACT.classify_revisions(base, head, repo), (True, "build-impacting"))

            base = head
            self.git(repo, "mv", "docs/app.md", "src/app.rs")
            self.git(repo, "commit", "-qm", "move docs to code")
            head = self.git(repo, "rev-parse", "HEAD").strip()
            self.assertEqual(CI_IMPACT.classify_revisions(base, head, repo), (True, "build-impacting"))

    def test_missing_invalid_and_mismatched_revisions_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            self.git(repo, "init", "-q")
            self.git(repo, "config", "user.email", "ci@example.invalid")
            self.git(repo, "config", "user.name", "CI Test")
            (repo / "README.md").write_text("base\n", encoding="utf-8")
            self.git(repo, "add", ".")
            self.git(repo, "commit", "-qm", "base")
            base = self.git(repo, "rev-parse", "HEAD").strip()
            (repo / "README.md").write_text("changed\n", encoding="utf-8")
            self.git(repo, "commit", "-qam", "head")
            head = self.git(repo, "rev-parse", "HEAD").strip()

            for invalid_base, invalid_head in (
                ("missing", head),
                ("0" * 40, head),
                ("f" * 40, head),
                (base, base),
            ):
                with self.subTest(base=invalid_base, head=invalid_head):
                    with self.assertRaises((ValueError, subprocess.CalledProcessError)):
                        CI_IMPACT.classify_revisions(invalid_base, invalid_head, repo)

    def test_cli_sanitizes_malformed_github_output(self):
        result = subprocess.run(
            [
                "python3",
                str(ROOT / "scripts" / "ci-impact.py"),
                "--base",
                "invalid\nfull=false",
                "--head",
                "missing\nreason=non-build-only",
            ],
            cwd=ROOT,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.assertEqual(
            result.stdout.splitlines(),
            [
                "full=true",
                "reason=classification-error",
                "base=invalid",
                "head=invalid",
            ],
        )

    @staticmethod
    def git(repo, *args):
        return subprocess.run(
            ["git", *args], cwd=repo, check=True, text=True, stdout=subprocess.PIPE
        ).stdout


class WorkflowPolicyTests(unittest.TestCase):
    def test_ci_required_names_and_fail_closed_wiring(self):
        workflow = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        for name in (
            "Source checks",
            "Native (${{ matrix.platform }})",
            "Windows-x64",
            "macOS-arm64",
            "macOS-x64",
            "Linux-x64",
        ):
            self.assertIn(name, workflow)
        self.assertIn("if: ${{ always() && needs.source.result == 'success' }}", workflow)
        self.assertIn("needs.impact.result == 'success'", workflow)
        self.assertIn("needs.impact.outputs.full == 'false'", workflow)
        self.assertIn("just ci-impact", workflow)
        self.assertIn("just ci-native-not-required", workflow)

    def test_e2e_always_classifies_and_forces_scheduled_manual_runs(self):
        workflow = (ROOT / ".github" / "workflows" / "desktop-e2e.yml").read_text(encoding="utf-8")
        self.assertNotIn("paths-ignore:", workflow)
        self.assertIn("just ci-impact", workflow)
        self.assertIn("just ci-impact-full", workflow)
        self.assertIn("just ci-e2e-not-required", workflow)
        self.assertIn('"platform":"Not-required"', workflow)
        self.assertIn(
            '"$MPD_CI_EVENT_NAME" == \'schedule\' || "$MPD_CI_EVENT_NAME" == \'workflow_dispatch\'',
            workflow,
        )

    def test_setup_just_is_exact_and_pinned(self):
        workflows = "\n".join(
            (ROOT / ".github" / "workflows" / name).read_text(encoding="utf-8")
            for name in ("ci.yml", "desktop-e2e.yml")
        )
        self.assertIn(
            "extractions/setup-just@53165ef7e734c5c07cb06b3c8e7b647c5aa16db3",
            workflows,
        )
        self.assertNotIn("extractions/setup-just@v4", workflows)
        self.assertIn("just-version: '1.58.0'", workflows)


if __name__ == "__main__":
    unittest.main()
