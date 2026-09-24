#!/usr/bin/env python3
"""Check resolved production dependencies, not optional entries in Cargo.lock.

This is a compile-graph boundary check. It does not claim that a normal native
binary was launched or that its runtime networking was observed.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

DRIVER_PACKAGE = "tauri-plugin-wdio-webdriver"
FIXTURE_FEATURE = "e2e-tests"


def assess_graph(graph):
    """Accept Cargo's package/features output only when test surfaces are absent."""
    packages = set()
    violations = set()
    found_application = False
    for line in graph.splitlines():
        if not line.strip():
            continue
        package, separator, feature_text = line.partition("|")
        if not separator or " v" not in package:
            raise ValueError("Unexpected Cargo graph format")
        name = package.split(" v", 1)[0]
        # Cargo marks repeated dependencies after the format string.
        feature_text = feature_text.removesuffix(" (*)").strip()
        features = set(filter(None, feature_text.split(",")))
        packages.add(name)
        if name == "mpd-tabber":
            found_application = True
            if "custom-protocol" not in features:
                violations.add("Application custom-protocol feature is not active")
        if name == DRIVER_PACKAGE:
            violations.add(f"Automation dependency is reachable: {name}")
        if FIXTURE_FEATURE in features:
            violations.add(f"Test fixture feature is active: {name}/{FIXTURE_FEATURE}")
    if not found_application:
        raise ValueError("Application was absent from the resolved Cargo graph")
    return {"package_count": len(packages), "violations": sorted(violations)}


def run_command(arguments):
    result = subprocess.run(arguments, capture_output=True, text=True, timeout=180)
    if result.returncode:
        # Preserve Cargo's diagnostic on a failed resolution; do not turn an
        # offline/missing-lockfile failure into a release-exclusion success.
        raise RuntimeError(f"{arguments[0]} failed ({result.returncode}): {result.stderr.strip()[:4096]}")
    return result.stdout


def main():
    toolchain = run_command(["rustc", "-vV"])
    host = next((line.removeprefix("host: ") for line in toolchain.splitlines() if line.startswith("host: ")), None)
    if not host:
        raise ValueError("Cannot establish the native target triple")
    results = []
    for defaults in (True, False):
        command = ["cargo", "tree", "--locked", "-p", "mpd-tabber", "--target", host,
                   "--features", "custom-protocol", "--edges", "normal,build", "--prefix", "none",
                   "--format", "{p}|{f}"]
        if not defaults:
            command.append("--no-default-features")
        graph = run_command(command)
        result = assess_graph(graph)
        result.update({"default_features": defaults, "target": host,
                       "graph_sha256": hashlib.sha256(graph.encode()).hexdigest()})
        results.append(result)
    evidence = {"check": "Resolved native production dependency and feature exclusion", "results": results,
                "runtime_probe": "Not performed by this graph check"}
    if "MPD_E2E_OUTPUT_DIR" in os.environ:
        output = Path(os.environ["MPD_E2E_OUTPUT_DIR"])
        output.mkdir(parents=True, exist_ok=True)
        (output / "release-boundary.json").write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(evidence, indent=2))
    return 1 if any(result["violations"] for result in results) else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
