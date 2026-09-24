#!/usr/bin/env python3
"""Hosted-runner-only negative probe of a normal, uninstrumented native build.

A live process with no listener on the supplied WebDriver port is narrower than
GUI readiness or a general network-isolation assertion. The normal application
may legitimately run its local player host on a different port.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time

MARKER = ".mpd-normal-probe"


def require_hosted(environment):
    if environment.get("GITHUB_ACTIONS") != "true" or environment.get("MPD_E2E_RUNNER_ENVIRONMENT") != "github-hosted":
        raise ValueError("Normal-app probe runs only in this workflow's disposable GitHub-hosted account, never a local user profile")
    supplied = Path(environment["RUNNER_TEMP"])
    if not supplied.is_absolute():
        raise ValueError("Runner temporary directory must be absolute")
    root = supplied.resolve(strict=True)
    if not root.is_absolute() or not root.is_dir():
        raise ValueError("A valid hosted runner temporary directory is required")
    return root


def owned_root(environment):
    parent = require_hosted(environment)
    supplied = Path(environment["MPD_E2E_PRODUCTION_ROOT"])
    if not supplied.is_absolute() or supplied.is_symlink() or getattr(supplied.lstat(), "st_file_attributes", 0) & 0x400:
        raise ValueError("Normal probe root must not be a link")
    root = supplied.resolve(strict=True)
    run_id = environment["MPD_E2E_PRODUCTION_RUN_ID"]
    if root.parent != parent or not root.name.startswith("mpd-normal-e2e-") or root.is_symlink():
        raise ValueError("Normal probe root is outside its owned runner directory")
    if len(run_id) != 32 or any(character not in "0123456789abcdef" for character in run_id):
        raise ValueError("Normal probe run ID is invalid")
    if (root / MARKER).read_text(encoding="utf-8") != run_id:
        raise ValueError("Normal probe ownership marker does not match")
    return root, run_id


def binary_name():
    return "MPD_Viewer-normal-probe.exe" if os.name == "nt" else "MPD_Viewer-normal-probe"


def stage():
    parent = require_hosted(os.environ)
    source = Path(os.environ["MPD_E2E_BINARY"]).resolve(strict=True)
    if not source.is_file():
        raise ValueError("Normal native build output is absent")
    root = Path(tempfile.mkdtemp(prefix="mpd-normal-e2e-", dir=parent))
    run_id = secrets.token_hex(16)
    (root / MARKER).write_text(run_id, encoding="utf-8")
    shutil.copy2(source, root / binary_name())
    with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as environment:
        environment.write(f"MPD_E2E_PRODUCTION_ROOT={root}\nMPD_E2E_PRODUCTION_RUN_ID={run_id}\n")
    print("Staged normal custom-protocol build separately from the instrumented build")


def port_open(port):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as connection:
        connection.settimeout(0.2)
        return connection.connect_ex(("127.0.0.1", port)) == 0


def probe():
    root, run_id = owned_root(os.environ)
    binary = root / binary_name()
    test_root = root / "ignored-e2e-root"
    test_root.mkdir()
    (test_root / ".mpd-e2e-root").write_text(run_id, encoding="utf-8")
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    if port_open(port):
        raise ValueError("Negative-probe port is already in use")
    environment = dict(os.environ)
    environment.update({"MPD_E2E_ROOT": str(test_root), "MPD_E2E_RUN_ID": run_id,
                        "MPD_E2E_SCENARIO": "web", "TAURI_WEBDRIVER_PORT": str(port)})
    evidence = {"check": "Normal build ignores E2E startup and cleanup hooks",
                "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "passed": False,
                "observation_seconds": 12, "command_flag": "--e2e-cleanup",
                "boundary": "Process survival and absence of listener on supplied IPv4 WebDriver port; not native GUI readiness or absence of all application listeners."}
    process = None
    previous_handlers = {}
    try:
        def interrupted(signum, frame):
            raise RuntimeError("Normal-build probe interrupted")
        for signum in (signal.SIGINT, signal.SIGTERM):
            previous_handlers[signum] = signal.signal(signum, interrupted)
        process = subprocess.Popen([str(binary), "--e2e-cleanup"], env=environment,
                                   stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        deadline = time.monotonic() + evidence["observation_seconds"]
        observations = 0
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise RuntimeError(f"Normal app exited during probe with code {process.returncode}")
            if port_open(port):
                raise RuntimeError("Normal build opened the supplied test-driver port")
            observations += 1
            time.sleep(0.2)
        if set(path.name for path in test_root.iterdir()) != {".mpd-e2e-root"}:
            raise RuntimeError("Normal build wrote into the test-only fixture root")
        evidence.update({"passed": True, "port_observations": observations, "test_root_untouched": True})
    except Exception as error:
        evidence["error"] = str(error)
        raise
    finally:
        for signum, handler in previous_handlers.items():
            signal.signal(signum, handler)
        if process is not None and process.poll() is None:
            # Terminate only this Popen child; never find or kill apps by name.
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
        evidence["owned_process_terminated"] = process is None or process.poll() is not None
        output = Path(os.environ["MPD_E2E_OUTPUT_DIR"])
        output.mkdir(parents=True, exist_ok=True)
        (output / "production-runtime-probe.json").write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
        # Recheck the resolved path and marker immediately before recursive cleanup.
        verified, _ = owned_root(os.environ)
        shutil.rmtree(verified)
    print("Normal native build survived the bounded test-hook probe without opening a driver port")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("stage", "run"))
    args = parser.parse_args()
    stage() if args.mode == "stage" else probe()


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
