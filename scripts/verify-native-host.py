#!/usr/bin/env python3
"""Reject unexpected runner architectures rather than publish mislabeled binaries."""
import subprocess
import sys
EXPECTED = {"Windows-x64": "x86_64-pc-windows-msvc", "macOS-arm64": "aarch64-apple-darwin",
            "macOS-x64": "x86_64-apple-darwin", "Linux-x64": "x86_64-unknown-linux-gnu"}
try:
    if len(sys.argv) != 2 or sys.argv[1] not in EXPECTED:
        raise ValueError("Specify one supported native platform.")
    result = subprocess.run(["rustc", "--version", "--verbose"], text=True, check=True,
                            stdout=subprocess.PIPE)
    host = next(line.split(": ", 1)[1] for line in result.stdout.splitlines() if line.startswith("host: "))
    if host != EXPECTED[sys.argv[1]]:
        raise ValueError("Runner Rust host does not match requested artifact platform: " + host)
    print("Verified native Rust host:", host)
except (ValueError, OSError, subprocess.CalledProcessError, StopIteration) as exc:
    print("Native host verification stopped:", exc, file=sys.stderr)
    sys.exit(1)
