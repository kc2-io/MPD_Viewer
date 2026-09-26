"""Offline checks for Windows CI toolchain isolation and workflow ordering."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("setup_ci_rustup", ROOT / "scripts/setup-ci-rustup.py")
RUSTUP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUSTUP)


class CiRustupTests(unittest.TestCase):
    def test_install_uses_committed_exact_toolchain_and_private_home(self):
        with tempfile.TemporaryDirectory() as temporary:
            runner = Path(temporary)
            github_env = runner / "github-env"
            github_env.write_text("EXISTING=value\n", encoding="utf-8")
            toolchain = tomllib.loads((ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))["toolchain"]
            channel = toolchain["channel"]
            calls = []

            def run(args, **kwargs):
                calls.append((args, kwargs))
                if args == ["rustc", "--version"]:
                    return subprocess.CompletedProcess(args, 0, f"rustc {channel} (fixture)\n", "")
                return subprocess.CompletedProcess(args, 0)

            with patch.dict(os.environ, {"GITHUB_ACTIONS": "true", "RUSTUP_HOME": "developer-home"}), \
                    patch.object(RUSTUP.subprocess, "run", side_effect=run):
                RUSTUP.install(system="win32", runner_temp=runner, github_env=github_env)
                self.assertEqual(os.environ["RUSTUP_HOME"], "developer-home")

            expected = ["rustup", "toolchain", "install", channel,
                        "--profile", toolchain["profile"]]
            for component in toolchain["components"]:
                expected.extend(("--component", component))
            for target in toolchain.get("targets", []):
                expected.extend(("--target", target))
            self.assertEqual(calls[0][0], expected)
            self.assertEqual(calls[0][1]["env"]["RUSTUP_HOME"], str(runner / "mpd-rustup"))
            self.assertEqual(calls[1][1]["env"]["RUSTUP_HOME"], str(runner / "mpd-rustup"))
            self.assertEqual(github_env.read_text(encoding="utf-8"),
                             f"EXISTING=value\nRUSTUP_HOME={runner / 'mpd-rustup'}\n")

    def test_failed_install_does_not_publish_rustup_home(self):
        with tempfile.TemporaryDirectory() as temporary:
            runner = Path(temporary)
            github_env = runner / "github-env"
            github_env.write_text("", encoding="utf-8")
            with patch.dict(os.environ, {"GITHUB_ACTIONS": "true"}), \
                    patch.object(RUSTUP.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "rustup")):
                with self.assertRaises(subprocess.CalledProcessError):
                    RUSTUP.install(system="win32", runner_temp=runner, github_env=github_env)
            self.assertEqual(github_env.read_text(encoding="utf-8"), "")

    def test_unpinned_toolchain_is_rejected_before_install(self):
        with tempfile.TemporaryDirectory() as temporary:
            toolchain = Path(temporary) / "rust-toolchain.toml"
            toolchain.write_text('[toolchain]\nchannel = "stable"\nprofile = "minimal"\n', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "exact stable"):
                RUSTUP.toolchain_args(toolchain)

    def test_optional_targets_are_installed_and_invalid_targets_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            toolchain = Path(temporary) / "rust-toolchain.toml"
            prefix = '[toolchain]\nchannel = "1.98.1"\nprofile = "minimal"\n'
            toolchain.write_text(prefix + 'targets = ["x86_64-pc-windows-msvc", "thumbv8m.main-none-eabihf"]\n',
                                 encoding="utf-8")
            _, args = RUSTUP.toolchain_args(toolchain)
            self.assertEqual(args[-4:], ["--target", "x86_64-pc-windows-msvc",
                                         "--target", "thumbv8m.main-none-eabihf"])
            toolchain.write_text(prefix + 'targets = ["--bad"]\n', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "targets"):
                RUSTUP.toolchain_args(toolchain)

    def test_version_mismatch_does_not_publish_rustup_home(self):
        with tempfile.TemporaryDirectory() as temporary:
            runner = Path(temporary)
            github_env = runner / "github-env"
            github_env.write_text("", encoding="utf-8")

            def run(args, **kwargs):
                output = "rustc 1.97.0 (fixture)\n" if args == ["rustc", "--version"] else ""
                return subprocess.CompletedProcess(args, 0, output, "")

            with patch.dict(os.environ, {"GITHUB_ACTIONS": "true"}), \
                    patch.object(RUSTUP.subprocess, "run", side_effect=run):
                with self.assertRaisesRegex(ValueError, "does not match"):
                    RUSTUP.install(system="win32", runner_temp=runner, github_env=github_env)
            self.assertEqual(github_env.read_text(encoding="utf-8"), "")

    def test_workflows_install_before_implicit_rustup_and_only_on_windows(self):
        step = ("      - name: Install pinned Windows Rust toolchain in isolated home\n"
                "        if: runner.os == 'Windows'\n"
                "        run: python scripts/setup-ci-rustup.py")
        for filename in ("ci.yml", "desktop-e2e.yml", "release.yml"):
            with self.subTest(filename=filename):
                text = (ROOT / ".github/workflows" / filename).read_text(encoding="utf-8")
                self.assertEqual(text.count(step), 1)
                self.assertLess(text.index(step), text.index("rustup show"))
        check = (ROOT / "scripts/ci-check.sh").read_text(encoding="utf-8")
        self.assertIn("python3 -m unittest discover -s tests -p 'ci_rustup_test.py' -v", check)
        for filename in ("ci.yml", "release.yml"):
            with self.subTest(source_gate=filename):
                text = (ROOT / ".github/workflows" / filename).read_text(encoding="utf-8")
                self.assertIn("- run: bash scripts/ci-check.sh", text)


if __name__ == "__main__":
    unittest.main()
