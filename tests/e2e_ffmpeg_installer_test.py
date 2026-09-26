#!/usr/bin/env python3
"""Safety and pin tests for the hosted E2E recorder installer."""
import gzip
import importlib.util
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts/install-e2e-ffmpeg.py"
SPEC = importlib.util.spec_from_file_location("installer", SCRIPT)
installer = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(installer)


class InstallerTests(unittest.TestCase):
    def test_all_hosted_targets_and_tools_are_pinned(self):
        self.assertEqual(set(installer.ASSETS), {
            ("linux", "x64"), ("darwin", "x64"), ("darwin", "arm64"), ("win32", "x64")})
        for tools in installer.ASSETS.values():
            self.assertEqual(set(tools), {"ffmpeg", "ffprobe"})
            for filename, digest in tools.values():
                self.assertRegex(filename, r"^(?:ffmpeg|ffprobe)-[a-z0-9-]+\.gz$")
                self.assertRegex(digest, r"^[0-9a-f]{64}$")

    def test_decompression_is_bounded_and_nonempty(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, output = root / "tool.gz", root / "tool"
            with gzip.open(source, "wb") as compressed:
                compressed.write(b"fixture binary")
            installer.decompress(source, output)
            self.assertEqual(output.read_bytes(), b"fixture binary")
            empty, rejected = root / "empty.gz", root / "empty"
            with gzip.open(empty, "wb"):
                pass
            with self.assertRaisesRegex(ValueError, "empty"):
                installer.decompress(empty, rejected)

    def test_architecture_normalization(self):
        self.assertEqual(installer.architecture("AMD64"), "x64")
        self.assertEqual(installer.architecture("x86_64"), "x64")
        self.assertEqual(installer.architecture("aarch64"), "arm64")


if __name__ == "__main__":
    unittest.main()
