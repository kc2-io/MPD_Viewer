#!/usr/bin/env python3
"""Record bounded, non-secret host facts for the native GUI run."""
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess


def command(*args):
    if not shutil.which(args[0]):
        return {"available": False}
    try:
        result = subprocess.run(args, capture_output=True, text=True, timeout=10)
        return {"exit_code": result.returncode, "output": result.stdout.strip()[:4096]}
    except (OSError, subprocess.TimeoutExpired) as error:
        return {"error": type(error).__name__}


def main():
    output = Path(os.environ["MPD_E2E_OUTPUT_DIR"]) if "MPD_E2E_OUTPUT_DIR" in os.environ else Path(os.environ["RUNNER_TEMP"]) / "mpd-e2e-evidence"
    if "GITHUB_ENV" in os.environ and "MPD_E2E_OUTPUT_DIR" not in os.environ:
        with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as environment:
            environment.write(f"MPD_E2E_OUTPUT_DIR={output}\n")
    output.mkdir(parents=True, exist_ok=True)
    facts = {
        "commit": os.environ.get("GITHUB_SHA", "local"),
        "platform": os.environ.get("MPD_E2E_PLATFORM", platform.system()),
        "os": platform.platform(),
        "architecture": platform.machine(),
        "runner_image": os.environ.get("ImageOS", "local"),
        "runner_image_version": os.environ.get("ImageVersion", "unknown"),
        "run_id": os.environ.get("GITHUB_RUN_ID", "local"),
        "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT", "local"),
        "extended": os.environ.get("MPD_E2E_EXTENDED") == "1",
        "node": command("node", "--version"),
        "rust": command("rustc", "--version"),
        "npm_dependencies": command("npm.cmd" if os.name == "nt" else "npm", "ls", "--prefix", "tests/e2e", "--depth=0", "--json"),
        "evidence_boundary": "Native Tauri application with controlled fixtures; no live Twitch or native OS input certification.",
    }
    if platform.system() == "Linux":
        facts["display"] = command("xdpyinfo")
        facts["webkitgtk"] = command("pkg-config", "--modversion", "webkit2gtk-4.1")
    elif platform.system() == "Windows":
        facts["display"] = command("powershell", "-NoProfile", "-Command", "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Screen]::AllScreens | Select-Object Bounds,Primary | ConvertTo-Json -Compress")
        facts["installed_webview2"] = command("powershell", "-NoProfile", "-Command", "Get-ChildItem -LiteralPath (([Environment]::GetFolderPath('ProgramFilesX86')) + '\\Microsoft\\EdgeWebView\\Application') -Directory | Select-Object -ExpandProperty Name")
    elif platform.system() == "Darwin":
        facts["display"] = command("system_profiler", "SPDisplaysDataType")
        facts["macos"] = command("sw_vers")
    (output / "runner-manifest.json").write_text(json.dumps(facts, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
