#!/usr/bin/env python3
"""Install the committed Rust toolchain into a Windows CI job's private rustup home."""
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[1]


def toolchain_args(path):
    toolchain = tomllib.loads(path.read_text(encoding="utf-8"))["toolchain"]
    channel = toolchain["channel"]
    profile = toolchain["profile"]
    components = toolchain.get("components", [])
    targets = toolchain.get("targets", [])
    if not isinstance(channel, str) or not re.fullmatch(r"1\.[0-9]+\.[0-9]+", channel):
        raise ValueError("Rust toolchain must use an exact stable version")
    if profile not in ("minimal", "default", "complete"):
        raise ValueError("Rust toolchain profile is invalid")
    if not isinstance(components, list) or any(
        not isinstance(component, str) or not re.fullmatch(r"[a-z][a-z0-9-]*", component)
        for component in components
    ):
        raise ValueError("Rust toolchain components are invalid")
    if not isinstance(targets, list) or any(
        not isinstance(target, str) or not re.fullmatch(
            r"[A-Za-z0-9_]+(?:\.[A-Za-z0-9_]+)*(?:-[A-Za-z0-9_]+(?:\.[A-Za-z0-9_]+)*)+", target
        )
        for target in targets
    ):
        raise ValueError("Rust toolchain targets are invalid")
    args = ["rustup", "toolchain", "install", channel, "--profile", profile]
    for component in components:
        args.extend(("--component", component))
    for target in targets:
        args.extend(("--target", target))
    return channel, args


def install(system=sys.platform, runner_temp=None, github_env=None, toolchain_file=None):
    if system != "win32" or os.environ.get("GITHUB_ACTIONS") != "true":
        raise ValueError("Isolated Rust installation is for Windows GitHub Actions only")
    channel, args = toolchain_args(Path(toolchain_file or ROOT / "rust-toolchain.toml"))
    runner = Path(runner_temp or os.environ["RUNNER_TEMP"])
    home = runner / "mpd-rustup"
    if "\n" in str(home) or "\r" in str(home):
        raise ValueError("Invalid runner temporary directory")
    home.mkdir(exist_ok=False)
    environment = os.environ.copy()
    environment["RUSTUP_HOME"] = str(home)
    subprocess.run(args, cwd=ROOT, env=environment, check=True)
    result = subprocess.run(["rustc", "--version"], cwd=ROOT, env=environment,
                            check=True, capture_output=True, text=True)
    if not result.stdout.startswith(f"rustc {channel} "):
        raise ValueError("Installed Rust version does not match rust-toolchain.toml")
    with Path(github_env or os.environ["GITHUB_ENV"]).open("a", encoding="utf-8") as output:
        output.write(f"RUSTUP_HOME={home}\n")
    print(f"Installed Rust {channel} in isolated Windows CI rustup home")


if __name__ == "__main__":
    try:
        install()
    except (KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"Windows CI Rust setup failed: {error}", file=sys.stderr)
        sys.exit(1)
